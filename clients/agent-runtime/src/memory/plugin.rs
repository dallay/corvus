use super::traits::{Memory, MemoryCategory, MemoryEntry, MemoryValidationResult};
use crate::config::MemoryConfig;
use crate::plugins::ResolvedMemoryPlugin;
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use wasmi::{Caller, Config as WasmiConfig, Engine, Extern, Linker, Module, Store};

const MAX_PLUGIN_REQUEST_BYTES: usize = 64 * 1024;
const MAX_PLUGIN_RESPONSE_BYTES: usize = 1024 * 1024;
const HOST_SQL_TIMEOUT_CAP_MS: u64 = 10_000;

#[derive(Clone)]
pub struct PluginBackedMemory {
    plugin_id: String,
    executor: PluginWasmExecutor,
}

impl PluginBackedMemory {
    pub fn new(
        plugin_runtime: ResolvedMemoryPlugin,
        config: &MemoryConfig,
        _workspace_dir: &Path,
    ) -> Result<Self> {
        if plugin_runtime.locked.runtime_api.trim() != "corvus-plugin/v1" {
            bail!(
                "unsupported memory plugin runtime_api '{}' for plugin '{}'",
                plugin_runtime.locked.runtime_api,
                plugin_runtime.locked.id
            );
        }

        let executor = PluginWasmExecutor::new(&plugin_runtime, config)?;
        Ok(Self {
            plugin_id: plugin_runtime.locked.id,
            executor,
        })
    }

    fn rpc_request(op: &str, args: Value) -> PluginRpcRequest {
        PluginRpcRequest {
            request_id: Uuid::new_v4().to_string(),
            op: op.to_string(),
            args,
            meta: None,
        }
    }
}

#[async_trait]
impl Memory for PluginBackedMemory {
    fn name(&self) -> &str {
        &self.plugin_id
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> Result<()> {
        let request = Self::rpc_request(
            "store",
            json!({
                "key": key,
                "content": content,
                "category": category_to_string(&category),
                "session_id": session_id,
            }),
        );
        let response = self.executor.invoke_memory(request).await?;
        response.ensure_ok()?;
        Ok(())
    }

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        if query.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let request = Self::rpc_request(
            "recall",
            json!({
                "query": query,
                "limit": limit,
                "session_id": session_id,
            }),
        );
        let response = self.executor.invoke_memory(request).await?;
        let result = response.ensure_ok()?;

        let entries = parse_wire_entries(result.get("entries"))?;
        Ok(entries)
    }

    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
        if key.trim().is_empty() {
            return Ok(None);
        }

        let request = Self::rpc_request("get", json!({ "key": key }));
        let response = self.executor.invoke_memory(request).await?;
        let result = response.ensure_ok()?;

        let Some(item) = result.get("item") else {
            return Ok(None);
        };
        if item.is_null() {
            return Ok(None);
        }

        let wire: WireMemoryEntry = serde_json::from_value(item.clone())
            .context("plugin returned malformed memory entry for get")?;
        Ok(Some(wire.into_memory_entry()))
    }

    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        let request = Self::rpc_request(
            "list",
            json!({
                "category": category.map(category_to_string),
                "session_id": session_id,
            }),
        );
        let response = self.executor.invoke_memory(request).await?;
        let result = response.ensure_ok()?;
        parse_wire_entries(result.get("entries"))
    }

    async fn forget(&self, key: &str) -> Result<bool> {
        if key.trim().is_empty() {
            return Ok(false);
        }

        let request = Self::rpc_request("forget", json!({ "key": key }));
        let response = self.executor.invoke_memory(request).await?;
        let result = response.ensure_ok()?;
        Ok(result
            .get("deleted")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }

    async fn count(&self) -> Result<usize> {
        let request = Self::rpc_request("count", json!({}));
        let response = self.executor.invoke_memory(request).await?;
        let result = response.ensure_ok()?;
        let raw = result.get("count").and_then(Value::as_u64).unwrap_or(0);
        #[allow(clippy::cast_possible_truncation)]
        {
            Ok(raw as usize)
        }
    }

    async fn health_check(&self) -> bool {
        let request = Self::rpc_request("health", json!({}));
        let Ok(response) = self.executor.invoke_health(request).await else {
            return false;
        };
        let Ok(result) = response.ensure_ok() else {
            return false;
        };

        let status_ok = result
            .get("status")
            .and_then(Value::as_str)
            .map(|value| value.eq_ignore_ascii_case("ok"))
            .unwrap_or(false);
        let db_ok = result
            .get("db_reachable")
            .and_then(Value::as_bool)
            .unwrap_or(status_ok);

        status_ok && db_ok
    }

    async fn validate_response(
        &self,
        user_query: &str,
        response: &str,
        session_id: Option<&str>,
    ) -> Result<MemoryValidationResult> {
        let request = Self::rpc_request(
            "validate_response",
            json!({
                "query": user_query,
                "response_text": response,
                "session_id": session_id,
            }),
        );

        let rpc_response = self.executor.invoke_memory(request).await?;
        let result = rpc_response.ensure_ok()?;

        let valid = result.get("valid").and_then(Value::as_bool).unwrap_or(true);
        let violations = result
            .get("violations")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(MemoryValidationResult { valid, violations })
    }
}

#[derive(Clone)]
struct PluginWasmExecutor {
    inner: Arc<PluginWasmExecutorInner>,
}

struct PluginWasmExecutorInner {
    wasm_path: PathBuf,
    engine: Engine,
    module: Module,
    memory_entrypoint: String,
    health_entrypoint: String,
    fuel_limit: u64,
    timeout: Duration,
    max_output_bytes: usize,
    surreal: SurrealSqlClient,
}

impl PluginWasmExecutor {
    fn new(plugin_runtime: &ResolvedMemoryPlugin, config: &MemoryConfig) -> Result<Self> {
        let wasm_path = plugin_runtime.wasm_path.clone();
        let wasm_bytes = fs::read(&wasm_path).with_context(|| {
            format!(
                "failed to read plugin wasm artifact: {}",
                wasm_path.display()
            )
        })?;

        if wasm_bytes.is_empty() {
            bail!("plugin wasm artifact is empty: {}", wasm_path.display());
        }

        let mut engine_config = WasmiConfig::default();
        engine_config.consume_fuel(true);
        let engine = Engine::new(&engine_config);
        let module = Module::new(&engine, &wasm_bytes)
            .with_context(|| format!("failed to parse wasm module: {}", wasm_path.display()))?;

        let memory_entrypoint = plugin_runtime
            .manifest
            .entrypoints
            .memory
            .clone()
            .unwrap_or_else(|| "memory_v1".to_string());
        let health_entrypoint = plugin_runtime
            .manifest
            .entrypoints
            .health
            .clone()
            .unwrap_or_else(|| "health_v1".to_string());

        let fuel_limit = plugin_runtime.manifest.limits.fuel.max(10_000);
        let timeout = Duration::from_millis(plugin_runtime.manifest.limits.timeout_ms.max(200));
        let max_output_bytes = plugin_runtime
            .manifest
            .limits
            .max_output_bytes
            .clamp(8 * 1024, MAX_PLUGIN_RESPONSE_BYTES);

        let surreal = SurrealSqlClient::from_memory_config(config)?;

        Ok(Self {
            inner: Arc::new(PluginWasmExecutorInner {
                wasm_path,
                engine,
                module,
                memory_entrypoint,
                health_entrypoint,
                fuel_limit,
                timeout,
                max_output_bytes,
                surreal,
            }),
        })
    }

    async fn invoke_memory(&self, request: PluginRpcRequest) -> Result<PluginRpcResponse> {
        self.invoke_with_entrypoint(self.inner.memory_entrypoint.clone(), request)
            .await
    }

    async fn invoke_health(&self, request: PluginRpcRequest) -> Result<PluginRpcResponse> {
        self.invoke_with_entrypoint(self.inner.health_entrypoint.clone(), request)
            .await
    }

    async fn invoke_with_entrypoint(
        &self,
        entrypoint: String,
        request: PluginRpcRequest,
    ) -> Result<PluginRpcResponse> {
        let timeout = self.inner.timeout;
        let executor = self.clone();
        let join = tokio::task::spawn_blocking(move || executor.invoke_sync(&entrypoint, request));

        let joined = tokio::time::timeout(timeout, join).await.map_err(|_| {
            anyhow!(
                "plugin invocation timed out after {} ms",
                timeout.as_millis()
            )
        })?;

        let response =
            joined.map_err(|join_error| anyhow!("plugin worker panicked: {join_error}"))??;
        Ok(response)
    }

    fn invoke_sync(
        &self,
        entrypoint: &str,
        request: PluginRpcRequest,
    ) -> Result<PluginRpcResponse> {
        let request_bytes =
            serde_json::to_vec(&request).context("failed to encode plugin request")?;
        if request_bytes.len() > MAX_PLUGIN_REQUEST_BYTES {
            bail!(
                "plugin request payload exceeds {} bytes limit",
                MAX_PLUGIN_REQUEST_BYTES
            );
        }

        let host_state = HostState {
            surreal: self.inner.surreal.clone(),
            max_output_bytes: self.inner.max_output_bytes,
        };

        let mut store = Store::new(&self.inner.engine, host_state);
        store
            .set_fuel(self.inner.fuel_limit)
            .context("failed to configure wasm fuel budget")?;

        let mut linker = Linker::new(&self.inner.engine);
        linker
            .func_wrap("corvus", "host_sql_v1", host_sql_v1)
            .context("failed to register host_sql_v1 import")?;

        let instance = linker
            .instantiate(&mut store, &self.inner.module)
            .and_then(|pre| pre.start(&mut store))
            .with_context(|| {
                format!(
                    "failed to instantiate memory plugin wasm: {}",
                    self.inner.wasm_path.display()
                )
            })?;

        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| anyhow!("plugin must export linear memory named 'memory'"))?;

        let alloc = instance
            .get_typed_func::<i32, i32>(&store, "alloc_v1")
            .or_else(|_| instance.get_typed_func::<i32, i32>(&store, "alloc"))
            .context("plugin must export alloc_v1(size) -> ptr")?;
        let dealloc = instance
            .get_typed_func::<(i32, i32), ()>(&store, "dealloc_v1")
            .or_else(|_| instance.get_typed_func::<(i32, i32), ()>(&store, "dealloc"))
            .context("plugin must export dealloc_v1(ptr, len)")?;
        let entry = instance
            .get_typed_func::<(i32, i32, i32, i32), i32>(&store, entrypoint)
            .with_context(|| format!("plugin missing entrypoint '{}'", entrypoint))?;

        let req_len_i32 =
            i32::try_from(request_bytes.len()).context("request exceeds i32 length")?;
        let req_ptr = alloc
            .call(&mut store, req_len_i32)
            .context("plugin alloc failed for request buffer")?;
        write_memory_bytes(&memory, &mut store, req_ptr, &request_bytes)
            .context("failed writing request bytes into plugin memory")?;

        let out_ptr_slot = alloc
            .call(&mut store, 4)
            .context("plugin alloc failed for response pointer slot")?;
        let out_len_slot = alloc
            .call(&mut store, 4)
            .context("plugin alloc failed for response length slot")?;

        let call_result = entry
            .call(
                &mut store,
                (req_ptr, req_len_i32, out_ptr_slot, out_len_slot),
            )
            .with_context(|| format!("plugin entrypoint '{}' failed", entrypoint))?;

        if call_result != 0 {
            let _ = dealloc.call(&mut store, (req_ptr, req_len_i32));
            let _ = dealloc.call(&mut store, (out_ptr_slot, 4));
            let _ = dealloc.call(&mut store, (out_len_slot, 4));
            bail!(
                "plugin entrypoint '{}' returned non-zero status {}",
                entrypoint,
                call_result
            );
        }

        let resp_ptr = read_i32_le(&memory, &store, out_ptr_slot)
            .context("failed reading plugin response pointer")?;
        let resp_len = read_i32_le(&memory, &store, out_len_slot)
            .context("failed reading plugin response length")?;

        if resp_ptr < 0 || resp_len < 0 {
            bail!("plugin returned negative response pointer/length");
        }

        #[allow(clippy::cast_sign_loss)]
        let resp_len_usize = resp_len as usize;
        if resp_len_usize > self.inner.max_output_bytes {
            bail!(
                "plugin response exceeds configured max_output_bytes ({} > {})",
                resp_len_usize,
                self.inner.max_output_bytes
            );
        }

        let response_bytes = read_memory_bytes(&memory, &store, resp_ptr, resp_len)
            .context("failed reading plugin response bytes")?;

        let _ = dealloc.call(&mut store, (req_ptr, req_len_i32));
        let _ = dealloc.call(&mut store, (out_ptr_slot, 4));
        let _ = dealloc.call(&mut store, (out_len_slot, 4));
        if resp_len > 0 {
            let _ = dealloc.call(&mut store, (resp_ptr, resp_len));
        }

        let response: PluginRpcResponse = serde_json::from_slice(&response_bytes)
            .context("failed decoding plugin JSON response")?;
        Ok(response)
    }
}

#[derive(Clone)]
struct SurrealSqlClient {
    sql_url: Url,
    namespace: String,
    database: String,
    username: Option<String>,
    password: Option<String>,
    token: Option<String>,
    client: reqwest::blocking::Client,
}

impl std::fmt::Debug for SurrealSqlClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SurrealSqlClient")
            .field("sql_url", &self.sql_url)
            .field("namespace", &self.namespace)
            .field("database", &self.database)
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .field("token", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl SurrealSqlClient {
    fn from_memory_config(config: &MemoryConfig) -> Result<Self> {
        let raw_endpoint = config
            .surreal
            .url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:8000".to_string());
        let endpoint = parse_endpoint(&raw_endpoint)?;
        validate_endpoint_security(&endpoint, config.surreal.allow_http_loopback)?;
        let base_http = normalize_http_endpoint(endpoint)?;

        let sql_url = base_http
            .join("sql")
            .with_context(|| format!("invalid SurrealDB SQL endpoint from '{}'", raw_endpoint))?;

        let client = reqwest::blocking::Client::builder()
            .build()
            .context("failed building SurrealDB HTTP client")?;

        Ok(Self {
            sql_url,
            namespace: non_empty_or_default(config.surreal.namespace.clone(), "corvus"),
            database: non_empty_or_default(config.surreal.database.clone(), "memory"),
            username: trim_optional(config.surreal.username.clone()),
            password: trim_optional(config.surreal.password.clone()),
            token: trim_optional(config.surreal.token.clone()),
            client,
        })
    }

    fn execute_sql(&self, payload: HostSqlRequest) -> Value {
        let timeout_ms = payload
            .timeout_ms
            .unwrap_or(5_000)
            .clamp(100, HOST_SQL_TIMEOUT_CAP_MS);

        let mut request = self
            .client
            .post(self.sql_url.clone())
            .header("Accept", "application/json")
            .header("Surreal-NS", self.namespace.as_str())
            .header("Surreal-DB", self.database.as_str())
            .timeout(Duration::from_millis(timeout_ms));

        if let Some(token) = self.token.as_deref() {
            request = request.bearer_auth(token);
        } else if let (Some(user), Some(pass)) =
            (self.username.as_deref(), self.password.as_deref())
        {
            request = request.basic_auth(user, Some(pass));
        }

        if let Some(vars) = payload.vars.as_ref().and_then(Value::as_object) {
            for (key, value) in vars {
                request = request.query(&[(key, surreal_query_literal(value))]);
            }
        }

        let response = match request.body(payload.query).send() {
            Ok(resp) => resp,
            Err(error) => {
                return json!({
                    "ok": false,
                    "status": 0,
                    "error": {
                        "code": "TRANSPORT_ERROR",
                        "message": error.to_string(),
                    }
                });
            }
        };

        let status = response.status();
        let body = response.text().unwrap_or_else(|_| String::new());
        let parsed_body = serde_json::from_str::<Value>(&body).unwrap_or_else(|_| {
            json!({
                "raw": body,
            })
        });

        json!({
            "ok": status.is_success(),
            "status": status.as_u16(),
            "result": parsed_body,
        })
    }
}

#[derive(Debug, Clone)]
struct HostState {
    surreal: SurrealSqlClient,
    max_output_bytes: usize,
}

fn host_sql_v1(
    mut caller: Caller<'_, HostState>,
    req_ptr: i32,
    req_len: i32,
    resp_ptr: i32,
    resp_cap: i32,
) -> i64 {
    let outcome = (|| -> Result<Vec<u8>> {
        let request_bytes = read_caller_memory_bytes(&mut caller, req_ptr, req_len)?;
        let request: HostSqlRequest = serde_json::from_slice(&request_bytes)
            .context("host_sql_v1 received invalid request JSON")?;
        let payload = caller.data().surreal.execute_sql(request);
        let encoded =
            serde_json::to_vec(&payload).context("failed to encode host_sql_v1 response")?;
        Ok(encoded)
    })();

    let output_bytes = match outcome {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!(
                error = %error,
                "host_sql_v1 transport error contacting SurrealDB"
            );
            let payload = json!({
                "ok": false,
                "status": 0,
                "error": {
                    "code": "HOST_SQL_ERROR",
                    "message": error.to_string(),
                }
            });
            serde_json::to_vec(&payload).unwrap_or_else(|_| b"{\"ok\":false}".to_vec())
        }
    };

    if output_bytes.len() > caller.data().max_output_bytes {
        let payload = json!({
            "ok": false,
            "status": 0,
            "error": {
                "code": "HOST_SQL_OUTPUT_TOO_LARGE",
                "message": "host sql output exceeded configured size limit"
            }
        });
        let fallback = serde_json::to_vec(&payload).unwrap_or_else(|_| b"{\"ok\":false}".to_vec());
        return write_hostcall_response(&mut caller, &fallback, resp_ptr, resp_cap);
    }

    write_hostcall_response(&mut caller, &output_bytes, resp_ptr, resp_cap)
}

fn write_hostcall_response(
    caller: &mut Caller<'_, HostState>,
    bytes: &[u8],
    resp_ptr: i32,
    resp_cap: i32,
) -> i64 {
    if resp_cap < 0 {
        return pack_host_status_len(4, 0);
    }

    #[allow(clippy::cast_sign_loss)]
    let cap = resp_cap as usize;
    if cap < bytes.len() {
        return pack_host_status_len(1, bytes.len());
    }

    if bytes.is_empty() {
        return pack_host_status_len(0, 0);
    }

    if let Err(error) = write_caller_memory_bytes(caller, resp_ptr, bytes) {
        tracing::debug!("host_sql_v1 write failed: {error}");
        return pack_host_status_len(3, 0);
    }

    pack_host_status_len(0, bytes.len())
}

#[derive(Debug, Clone, Serialize)]
struct PluginRpcRequest {
    request_id: String,
    op: String,
    args: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct PluginRpcResponse {
    request_id: String,
    ok: bool,
    #[serde(default)]
    result: Value,
    #[serde(default)]
    error: Option<PluginRpcError>,
}

impl PluginRpcResponse {
    fn ensure_ok(self) -> Result<Value> {
        if self.ok {
            return Ok(self.result);
        }

        let message = self.error.as_ref().map_or_else(
            || "plugin returned unknown error".to_string(),
            |err| format!("{}: {}", err.code, err.message),
        );
        bail!(message)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct PluginRpcError {
    code: String,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct HostSqlRequest {
    query: String,
    #[serde(default)]
    vars: Option<Value>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct WireMemoryEntry {
    id: String,
    key: String,
    content: String,
    category: String,
    timestamp: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    score: Option<f64>,
}

impl WireMemoryEntry {
    fn into_memory_entry(self) -> MemoryEntry {
        MemoryEntry {
            id: self.id,
            key: self.key,
            content: self.content,
            category: category_from_string(&self.category),
            timestamp: self.timestamp,
            session_id: self.session_id,
            score: self.score,
        }
    }
}

fn parse_wire_entries(value: Option<&Value>) -> Result<Vec<MemoryEntry>> {
    let payload = value.cloned().unwrap_or_else(|| Value::Array(Vec::new()));
    let wire: Vec<WireMemoryEntry> = serde_json::from_value(payload)
        .context("plugin returned malformed memory entries payload")?;
    Ok(wire
        .into_iter()
        .map(WireMemoryEntry::into_memory_entry)
        .collect())
}

fn category_to_string(category: &MemoryCategory) -> String {
    match category {
        MemoryCategory::Core => "core".to_string(),
        MemoryCategory::Daily => "daily".to_string(),
        MemoryCategory::Conversation => "conversation".to_string(),
        MemoryCategory::Custom(value) => value.clone(),
    }
}

fn category_from_string(value: &str) -> MemoryCategory {
    match value {
        "core" => MemoryCategory::Core,
        "daily" => MemoryCategory::Daily,
        "conversation" => MemoryCategory::Conversation,
        other => MemoryCategory::Custom(other.to_string()),
    }
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn non_empty_or_default(value: Option<String>, default_value: &str) -> String {
    trim_optional(value).unwrap_or_else(|| default_value.to_string())
}

fn parse_endpoint(raw_endpoint: &str) -> Result<Url> {
    let raw = raw_endpoint.trim();
    if raw.is_empty() {
        bail!("SurrealDB endpoint URL cannot be empty");
    }

    Url::parse(raw)
        .or_else(|_| Url::parse(&format!("http://{raw}")))
        .with_context(|| format!("invalid SurrealDB endpoint URL '{raw_endpoint}'"))
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host == "[::1]"
}

fn validate_endpoint_security(endpoint: &Url, allow_http_loopback: bool) -> Result<()> {
    let scheme = endpoint.scheme();
    let host = endpoint.host_str().unwrap_or_default();

    match scheme {
        "https" => Ok(()),
        "http" => {
            if !allow_http_loopback {
                bail!(
                    "refusing insecure SurrealDB URL: http is disabled (set memory.surreal.allow_http_loopback=true for localhost only)"
                );
            }
            if !is_loopback_host(host) {
                bail!("refusing insecure SurrealDB URL over http for non-loopback host '{host}'");
            }
            Ok(())
        }
        "ws" => {
            if !allow_http_loopback {
                bail!(
                    "refusing insecure SurrealDB URL: http is disabled (set memory.surreal.allow_http_loopback=true for localhost only)"
                );
            }
            if !is_loopback_host(host) {
                bail!("refusing insecure SurrealDB URL over http for non-loopback host '{host}'");
            }
            Ok(())
        }
        "wss" => {
            if !allow_http_loopback && !is_loopback_host(host) {
                bail!("refusing insecure SurrealDB URL over http for non-loopback host '{host}'");
            }
            Ok(())
        }
        other => {
            bail!("unsupported SurrealDB URL scheme '{other}', expected one of http/https/ws/wss")
        }
    }
}

fn normalize_http_endpoint(mut endpoint: Url) -> Result<Url> {
    match endpoint.scheme() {
        "https" => {}
        "http" => {}
        "wss" => endpoint
            .set_scheme("https")
            .map_err(|_| anyhow!("failed converting wss endpoint to https"))?,
        "ws" => endpoint
            .set_scheme("http")
            .map_err(|_| anyhow!("failed converting ws endpoint to http"))?,
        other => {
            bail!("unsupported SurrealDB URL scheme '{other}', expected one of http/https/ws/wss")
        }
    }

    if endpoint.path() == "/rpc" {
        endpoint.set_path("/");
    }

    Ok(endpoint)
}

fn surreal_query_literal(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "null".to_string()),
    }
}

fn read_memory_bytes(
    memory: &wasmi::Memory,
    store: &Store<HostState>,
    ptr: i32,
    len: i32,
) -> Result<Vec<u8>> {
    if ptr < 0 || len < 0 {
        bail!("negative memory pointer/length from plugin");
    }

    #[allow(clippy::cast_sign_loss)]
    let len_usize = len as usize;
    let mut bytes = vec![0u8; len_usize];
    memory
        .read(store, ptr as usize, &mut bytes)
        .context("failed reading bytes from plugin memory")?;
    Ok(bytes)
}

fn write_memory_bytes(
    memory: &wasmi::Memory,
    store: &mut Store<HostState>,
    ptr: i32,
    bytes: &[u8],
) -> Result<()> {
    if ptr < 0 {
        bail!("negative plugin memory pointer");
    }

    memory
        .write(store, ptr as usize, bytes)
        .context("failed writing bytes into plugin memory")
}

fn read_i32_le(memory: &wasmi::Memory, store: &Store<HostState>, ptr: i32) -> Result<i32> {
    let mut raw = [0u8; 4];
    memory
        .read(store, ptr as usize, &mut raw)
        .context("failed reading i32 from plugin memory")?;
    Ok(i32::from_le_bytes(raw))
}

fn pack_host_status_len(status: usize, len: usize) -> i64 {
    let status_u32 = u32::try_from(status).unwrap_or(u32::MAX);
    let len_u32 = u32::try_from(len).unwrap_or(u32::MAX);
    let packed = (u64::from(status_u32) << 32) | u64::from(len_u32);
    i64::from_ne_bytes(packed.to_ne_bytes())
}

fn read_caller_memory_bytes(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
) -> Result<Vec<u8>> {
    if ptr < 0 || len < 0 {
        bail!("negative memory pointer/length in host import");
    }

    let memory = caller_memory(caller)?;
    #[allow(clippy::cast_sign_loss)]
    let len_usize = len as usize;
    let mut bytes = vec![0u8; len_usize];
    memory
        .read(caller, ptr as usize, &mut bytes)
        .context("failed reading request bytes from caller memory")?;
    Ok(bytes)
}

fn write_caller_memory_bytes(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    bytes: &[u8],
) -> Result<()> {
    if ptr < 0 {
        bail!("negative response pointer in host import");
    }

    let memory = caller_memory(caller)?;
    memory
        .write(caller, ptr as usize, bytes)
        .context("failed writing response bytes into caller memory")
}

fn caller_memory(caller: &mut Caller<'_, HostState>) -> Result<wasmi::Memory> {
    let extern_memory = caller
        .get_export("memory")
        .ok_or_else(|| anyhow!("plugin did not export memory"))?;

    match extern_memory {
        Extern::Memory(memory) => Ok(memory),
        _ => bail!("plugin export 'memory' is not a memory export"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn category_roundtrip_core_values() {
        let categories = [
            MemoryCategory::Core,
            MemoryCategory::Daily,
            MemoryCategory::Conversation,
            MemoryCategory::Custom("project".to_string()),
        ];

        for category in categories {
            let serialized = category_to_string(&category);
            let parsed = category_from_string(&serialized);
            assert_eq!(category.to_string(), parsed.to_string());
        }
    }

    #[test]
    fn normalize_ws_endpoint_to_http() {
        let endpoint = parse_endpoint("ws://127.0.0.1:8000/rpc").unwrap();
        let normalized = normalize_http_endpoint(endpoint).unwrap();
        assert_eq!(normalized.as_str(), "http://127.0.0.1:8000/");
    }

    #[test]
    fn reject_insecure_non_loopback_http_endpoint() {
        let endpoint = parse_endpoint("http://10.0.0.7:8000").unwrap();
        let error = validate_endpoint_security(&endpoint, true).unwrap_err();
        assert!(error.to_string().contains("non-loopback"));
    }

    #[test]
    fn reject_non_loopback_wss_when_http_loopback_is_disabled() {
        let endpoint = parse_endpoint("wss://example.com/rpc").unwrap();
        let error = validate_endpoint_security(&endpoint, false).unwrap_err();
        assert!(error.to_string().contains("non-loopback"));
    }

    #[test]
    fn pack_status_len_roundtrips_values() {
        let packed = pack_host_status_len(1, 2048);
        let raw = u64::from_ne_bytes(packed.to_ne_bytes());
        let status = (raw >> 32) as u32;
        let len = (raw & 0xFFFF_FFFF) as u32;
        assert_eq!(status, 1);
        assert_eq!(len, 2048);
    }

    #[test]
    fn invoke_sync_executes_wasm_roundtrip_successfully() {
        let wasm_bytes = wat::parse_str(
            r#"(module
                (memory (export "memory") 1)
                (global $heap (mut i32) (i32.const 1024))
                (func (export "alloc_v1") (param $size i32) (result i32)
                  (local $ptr i32)
                  global.get $heap
                  local.set $ptr
                  global.get $heap
                  local.get $size
                  i32.add
                  global.set $heap
                  local.get $ptr
                )
                (func (export "dealloc_v1") (param i32 i32))
                (func (export "memory_v1") (param $req_ptr i32) (param $req_len i32) (param $out_ptr_slot i32) (param $out_len_slot i32) (result i32)
                  local.get $out_ptr_slot
                  i32.const 1536
                  i32.store
                  local.get $out_len_slot
                  i32.const 40
                  i32.store
                  i32.const 0
                )
                (data (i32.const 1536) "{\"request_id\":\"r\",\"ok\":true,\"result\":{}}")
            )"#,
        )
        .unwrap();

        let mut engine_config = WasmiConfig::default();
        engine_config.consume_fuel(true);
        let engine = Engine::new(&engine_config);
        let module = Module::new(&engine, &wasm_bytes).unwrap();

        let surreal = SurrealSqlClient {
            sql_url: Url::parse("http://127.0.0.1:8000/sql").unwrap(),
            namespace: "test".to_string(),
            database: "test".to_string(),
            username: None,
            password: None,
            token: None,
            client: reqwest::blocking::Client::builder().build().unwrap(),
        };

        let executor = PluginWasmExecutor {
            inner: Arc::new(PluginWasmExecutorInner {
                wasm_path: PathBuf::from("inline-test.wasm"),
                engine,
                module,
                memory_entrypoint: "memory_v1".to_string(),
                health_entrypoint: "health_v1".to_string(),
                fuel_limit: 100_000,
                timeout: Duration::from_secs(1),
                max_output_bytes: 1024,
                surreal,
            }),
        };

        let request = PluginRpcRequest {
            request_id: "req-1".to_string(),
            op: "health".to_string(),
            args: json!({}),
            meta: None,
        };

        let response = executor.invoke_sync("memory_v1", request).unwrap();
        assert!(response.ok);
        assert_eq!(response.request_id, "r");
        assert_eq!(response.result, json!({}));
    }
}
