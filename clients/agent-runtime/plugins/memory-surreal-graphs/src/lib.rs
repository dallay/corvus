use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::slice;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_TERMS: usize = 12;
const MAX_RECALL_LIMIT: usize = 100;
const MAX_OUTPUT_BYTES: usize = 262_144;

static SCHEMA_INIT_RESULT: OnceLock<std::result::Result<(), String>> = OnceLock::new();

#[link(wasm_import_module = "corvus")]
extern "C" {
    fn host_sql_v1(req_ptr: i32, req_len: i32, resp_ptr: i32, resp_cap: i32) -> i64;
}

#[derive(Debug, Deserialize)]
struct PluginRequest {
    request_id: String,
    op: String,
    #[serde(default)]
    args: Value,
}

#[derive(Debug, Serialize)]
struct PluginResponse {
    request_id: String,
    ok: bool,
    result: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<PluginError>,
}

#[derive(Debug, Serialize)]
struct PluginError {
    code: String,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HostSqlRequest {
    query: String,
    #[serde(default)]
    vars: Option<Value>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct StoreArgs {
    key: String,
    content: String,
    category: String,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RecallArgs {
    query: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GetArgs {
    key: String,
}

#[derive(Debug, Deserialize)]
struct ListArgs {
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ForgetArgs {
    key: String,
}

#[derive(Debug, Deserialize)]
struct ValidateResponseArgs {
    query: String,
    response_text: String,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct EntryOut {
    id: String,
    key: String,
    content: String,
    category: String,
    timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<f64>,
}

#[derive(Debug, Clone)]
struct CandidateEntry {
    entry: EntryOut,
    edge_boost: f64,
}

#[no_mangle]
pub extern "C" fn alloc_v1(size: i32) -> i32 {
    if size <= 0 {
        return 0;
    }

    let mut buffer = Vec::<u8>::with_capacity(size as usize);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr as i32
}

#[no_mangle]
pub extern "C" fn dealloc_v1(ptr: i32, len: i32) {
    if ptr <= 0 || len <= 0 {
        return;
    }

    unsafe {
        let _ = Vec::<u8>::from_raw_parts(ptr as *mut u8, 0, len as usize);
    }
}

#[no_mangle]
pub extern "C" fn memory_v1(
    req_ptr: i32,
    req_len: i32,
    resp_ptr_out: i32,
    resp_len_out: i32,
) -> i32 {
    dispatch(req_ptr, req_len, resp_ptr_out, resp_len_out)
}

#[no_mangle]
pub extern "C" fn health_v1(
    req_ptr: i32,
    req_len: i32,
    resp_ptr_out: i32,
    resp_len_out: i32,
) -> i32 {
    dispatch(req_ptr, req_len, resp_ptr_out, resp_len_out)
}

#[no_mangle]
pub extern "C" fn run() -> i32 {
    0
}

fn dispatch(req_ptr: i32, req_len: i32, resp_ptr_out: i32, resp_len_out: i32) -> i32 {
    let response = match execute(req_ptr, req_len) {
        Ok(value) => value,
        Err((request_id, code, message)) => PluginResponse {
            request_id,
            ok: false,
            result: json!({}),
            error: Some(PluginError { code, message }),
        },
    };

    let encoded = serde_json::to_vec(&response).unwrap_or_else(|_| {
        br#"{"request_id":"unknown","ok":false,"result":{},"error":{"code":"ENCODE_ERROR","message":"failed to encode plugin response"}}"#.to_vec()
    });

    if encoded.len() > MAX_OUTPUT_BYTES {
        return 8;
    }

    let resp_len_i32 = match i32::try_from(encoded.len()) {
        Ok(value) => value,
        Err(_) => {
            return 5;
        }
    };

    let resp_ptr = alloc_v1(resp_len_i32);
    if resp_ptr <= 0 {
        return 6;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(encoded.as_ptr(), resp_ptr as *mut u8, encoded.len());
    }

    if write_i32(resp_ptr_out, resp_ptr).is_err() || write_i32(resp_len_out, resp_len_i32).is_err()
    {
        dealloc_v1(resp_ptr, resp_len_i32);
        return 7;
    }

    0
}

fn execute(
    req_ptr: i32,
    req_len: i32,
) -> std::result::Result<PluginResponse, (String, String, String)> {
    let request_bytes = read_input(req_ptr, req_len)
        .map_err(|error| ("unknown".to_string(), "INVALID_REQUEST".to_string(), error))?;
    let request: PluginRequest = serde_json::from_slice(&request_bytes).map_err(|error| {
        (
            "unknown".to_string(),
            "INVALID_REQUEST".to_string(),
            format!("request JSON decode failed: {error}"),
        )
    })?;

    if let Err(error) = ensure_schema() {
        return Err((request.request_id, "SCHEMA_INIT_FAILED".to_string(), error));
    }

    let result = match request.op.as_str() {
        "store" => {
            let args: StoreArgs =
                serde_json::from_value(request.args.clone()).map_err(|error| {
                    (
                        request.request_id.clone(),
                        "INVALID_ARGS".to_string(),
                        format!("store args invalid: {error}"),
                    )
                })?;
            store_memory(args).map(|stored_id| json!({ "id": stored_id, "created": true }))
        }
        "recall" => {
            let args: RecallArgs =
                serde_json::from_value(request.args.clone()).map_err(|error| {
                    (
                        request.request_id.clone(),
                        "INVALID_ARGS".to_string(),
                        format!("recall args invalid: {error}"),
                    )
                })?;
            recall_memory(args).map(|entries| json!({ "entries": entries }))
        }
        "get" => {
            let args: GetArgs = serde_json::from_value(request.args.clone()).map_err(|error| {
                (
                    request.request_id.clone(),
                    "INVALID_ARGS".to_string(),
                    format!("get args invalid: {error}"),
                )
            })?;
            get_memory(args).map(|entry| json!({ "item": entry }))
        }
        "list" => {
            let args: ListArgs = serde_json::from_value(request.args.clone()).map_err(|error| {
                (
                    request.request_id.clone(),
                    "INVALID_ARGS".to_string(),
                    format!("list args invalid: {error}"),
                )
            })?;
            list_memory(args).map(|entries| json!({ "entries": entries }))
        }
        "forget" => {
            let args: ForgetArgs =
                serde_json::from_value(request.args.clone()).map_err(|error| {
                    (
                        request.request_id.clone(),
                        "INVALID_ARGS".to_string(),
                        format!("forget args invalid: {error}"),
                    )
                })?;
            forget_memory(args).map(|deleted| json!({ "deleted": deleted }))
        }
        "count" => count_memory().map(|count| json!({ "count": count })),
        "health" => health_status().map(|healthy| {
            if healthy {
                json!({
                    "status": "ok",
                    "abi_version": "memory_v1",
                    "db_reachable": true,
                })
            } else {
                json!({
                    "status": "down",
                    "abi_version": "memory_v1",
                    "db_reachable": false,
                })
            }
        }),
        "validate_response" => {
            let args: ValidateResponseArgs =
                serde_json::from_value(request.args.clone()).map_err(|error| {
                    (
                        request.request_id.clone(),
                        "INVALID_ARGS".to_string(),
                        format!("validate_response args invalid: {error}"),
                    )
                })?;
            validate_response(args).map(|(valid, violations)| {
                json!({
                    "valid": valid,
                    "violations": violations,
                })
            })
        }
        _ => Err(format!("unsupported memory operation '{}'", request.op)),
    };

    match result {
        Ok(result_payload) => Ok(PluginResponse {
            request_id: request.request_id,
            ok: true,
            result: result_payload,
            error: None,
        }),
        Err(message) => Err((request.request_id, "OPERATION_FAILED".to_string(), message)),
    }
}

fn read_input(req_ptr: i32, req_len: i32) -> std::result::Result<Vec<u8>, String> {
    if req_ptr < 0 || req_len < 0 {
        return Err("negative request pointer/length".to_string());
    }

    unsafe {
        let slice = slice::from_raw_parts(req_ptr as *const u8, req_len as usize);
        Ok(slice.to_vec())
    }
}

fn write_i32(ptr: i32, value: i32) -> std::result::Result<(), String> {
    if ptr <= 0 {
        return Err("invalid output slot pointer".to_string());
    }

    let bytes = value.to_le_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
    }
    Ok(())
}

fn ensure_schema() -> std::result::Result<(), String> {
    SCHEMA_INIT_RESULT
        .get_or_init(|| {
            let statements = [
                "DEFINE TABLE memory_entries SCHEMALESS;",
                "DEFINE TABLE memory_edges SCHEMALESS;",
                "DEFINE TABLE ontology_terms SCHEMALESS;",
                "DEFINE TABLE ontology_rules SCHEMALESS;",
                "DEFINE INDEX idx_memory_entries_key ON TABLE memory_entries COLUMNS key UNIQUE;",
                "DEFINE INDEX idx_memory_entries_category ON TABLE memory_entries COLUMNS category;",
                "DEFINE INDEX idx_memory_entries_session ON TABLE memory_entries COLUMNS session_id;",
                "DEFINE INDEX idx_memory_edges_relation ON TABLE memory_edges COLUMNS relation_type;",
                "DEFINE INDEX idx_memory_edges_to ON TABLE memory_edges COLUMNS to_id;",
            ];

            for statement in statements {
                sql_query(statement, None, Some(3_000)).map_err(|error| {
                    format!("schema bootstrap failed for statement '{statement}': {error}")
                })?;
            }

            Ok(())
        })
        .clone()
}

fn store_memory(args: StoreArgs) -> std::result::Result<String, String> {
    let key = args.key.trim();
    let content = args.content.trim();
    if key.is_empty() {
        return Err("memory key cannot be empty".to_string());
    }
    if content.is_empty() {
        return Err("memory content cannot be empty".to_string());
    }

    let category = normalize_category(&args.category);
    let key_hash = sha256_hex(key);
    let now = now_string();
    let session_literal = option_literal(args.session_id.as_deref());
    let entry_record_id = format!("memory_entries:{key_hash}");

    let upsert_query = format!(
        "UPSERT type::thing(\"memory_entries\", {id}) CONTENT {{ key: {key}, content: {content}, category: {category}, timestamp: {timestamp}, updated_at: {updated_at}, session_id: {session_id} }};",
        id = surreal_string_literal(&key_hash),
        key = surreal_string_literal(key),
        content = surreal_string_literal(content),
        category = surreal_string_literal(&category),
        timestamp = surreal_string_literal(&now),
        updated_at = surreal_string_literal(&now),
        session_id = session_literal,
    );
    sql_query(&upsert_query, None, Some(5_000))?;

    let terms = extract_terms(&format!("{key} {content}"));
    for term in terms {
        let term_hash = sha256_hex(&term);
        let term_record_id = format!("ontology_terms:{term_hash}");

        let upsert_term_query = format!(
            "UPSERT type::thing(\"ontology_terms\", {term_id}) CONTENT {{ term: {term}, updated_at: {updated_at} }};",
            term_id = surreal_string_literal(&term_hash),
            term = surreal_string_literal(&term),
            updated_at = surreal_string_literal(&now),
        );
        let _ = sql_query(&upsert_term_query, None, Some(4_000));

        let edge_query = format!(
            "CREATE memory_edges CONTENT {{ relation_type: \"entry_term\", from_id: {from_id}, to_id: {to_id}, at: {at}, session_id: {session_id} }};",
            from_id = surreal_string_literal(&entry_record_id),
            to_id = surreal_string_literal(&term_record_id),
            at = surreal_string_literal(&now),
            session_id = option_literal(args.session_id.as_deref()),
        );
        let _ = sql_query(&edge_query, None, Some(4_000));
    }

    Ok(entry_record_id)
}

fn recall_memory(args: RecallArgs) -> std::result::Result<Vec<EntryOut>, String> {
    let query = args.query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let limit = args.limit.unwrap_or(10).clamp(1, MAX_RECALL_LIMIT);
    let query_lower = query.to_lowercase();
    let session_filter = args
        .session_id
        .as_deref()
        .map(|sid| format!(" AND session_id = {}", surreal_string_literal(sid)))
        .unwrap_or_default();

    let keyword_query = format!(
        "SELECT * FROM memory_entries WHERE (string::contains(string::lowercase(key), {query}) OR string::contains(string::lowercase(content), {query})) {session_filter} ORDER BY updated_at DESC LIMIT {limit};",
        query = surreal_string_literal(&query_lower),
    );
    let keyword_rows = sql_rows(&keyword_query, None, Some(6_000))?;

    let mut candidates: HashMap<String, CandidateEntry> = HashMap::new();
    for row in keyword_rows {
        if let Some(entry) = row_to_entry(&row) {
            candidates.insert(
                entry.id.clone(),
                CandidateEntry {
                    entry,
                    edge_boost: 0.0,
                },
            );
        }
    }

    let mut expanded_ids = HashSet::new();
    for term in extract_terms(query) {
        let term_hash = sha256_hex(&term);
        let term_record_id = format!("ontology_terms:{term_hash}");
        let edge_lookup = format!(
            "SELECT from_id FROM memory_edges WHERE relation_type = \"entry_term\" AND to_id = {to_id} LIMIT 64;",
            to_id = surreal_string_literal(&term_record_id),
        );

        for edge_row in sql_rows(&edge_lookup, None, Some(4_000)).unwrap_or_default() {
            if let Some(from_id) = edge_row.get("from_id").and_then(Value::as_str) {
                expanded_ids.insert(from_id.to_string());
            }
        }
    }

    if !expanded_ids.is_empty() {
        let id_list = Value::Array(
            expanded_ids
                .iter()
                .map(|id| Value::String(id.clone()))
                .collect(),
        );

        let expanded_query = format!(
            "SELECT * FROM memory_entries WHERE id IN {ids} LIMIT {limit};",
            ids = id_list,
            limit = limit * 2,
        );
        for row in sql_rows(&expanded_query, None, Some(6_000)).unwrap_or_default() {
            if let Some(entry) = row_to_entry(&row) {
                let candidate = candidates
                    .entry(entry.id.clone())
                    .or_insert(CandidateEntry {
                        entry,
                        edge_boost: 0.0,
                    });
                candidate.edge_boost = 0.2;
            }
        }
    }

    let terms = extract_terms(query);
    let mut ranked: Vec<EntryOut> = candidates
        .into_values()
        .map(|candidate| {
            let mut entry = candidate.entry;
            let searchable = format!(
                "{} {}",
                entry.key.to_lowercase(),
                entry.content.to_lowercase()
            );
            let matches = terms
                .iter()
                .filter(|term| searchable.contains(term.as_str()))
                .count();

            let mut score = if terms.is_empty() {
                0.5
            } else {
                (matches as f64) / (terms.len() as f64)
            };
            if searchable.contains(&query_lower) {
                score += 0.15;
            }
            score += candidate.edge_boost;
            entry.score = Some(score.min(1.0));
            entry
        })
        .collect();

    ranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.truncate(limit);

    Ok(ranked)
}

fn get_memory(args: GetArgs) -> std::result::Result<Option<EntryOut>, String> {
    let key = args.key.trim();
    if key.is_empty() {
        return Ok(None);
    }

    let query = format!(
        "SELECT * FROM memory_entries WHERE key = {key} LIMIT 1;",
        key = surreal_string_literal(key),
    );
    let rows = sql_rows(&query, None, Some(4_000))?;
    Ok(rows.into_iter().find_map(|row| row_to_entry(&row)))
}

fn list_memory(args: ListArgs) -> std::result::Result<Vec<EntryOut>, String> {
    let mut clauses = Vec::new();
    if let Some(category) = args.category.as_deref() {
        clauses.push(format!("category = {}", surreal_string_literal(category)));
    }
    if let Some(session_id) = args.session_id.as_deref() {
        clauses.push(format!(
            "session_id = {}",
            surreal_string_literal(session_id)
        ));
    }

    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };

    let query =
        format!("SELECT * FROM memory_entries{where_clause} ORDER BY updated_at DESC LIMIT 1000;");
    let rows = sql_rows(&query, None, Some(6_000))?;
    Ok(rows
        .into_iter()
        .filter_map(|row| row_to_entry(&row))
        .collect())
}

fn forget_memory(args: ForgetArgs) -> std::result::Result<bool, String> {
    let key = args.key.trim();
    if key.is_empty() {
        return Ok(false);
    }

    let key_hash = sha256_hex(key);
    let entry_record_id = format!("memory_entries:{key_hash}");

    let exists_query = format!(
        "SELECT id FROM memory_entries WHERE key = {key} LIMIT 1;",
        key = surreal_string_literal(key),
    );
    let existing = sql_rows(&exists_query, None, Some(3_000))?;
    if existing.is_empty() {
        return Ok(false);
    }

    let delete_entry_query = format!(
        "DELETE type::thing(\"memory_entries\", {id});",
        id = surreal_string_literal(&key_hash),
    );
    let _ = sql_query(&delete_entry_query, None, Some(4_000));

    let delete_edges_query = format!(
        "DELETE memory_edges WHERE from_id = {id} OR to_id = {id};",
        id = surreal_string_literal(&entry_record_id),
    );
    let _ = sql_query(&delete_edges_query, None, Some(4_000));

    Ok(true)
}

fn count_memory() -> std::result::Result<u64, String> {
    let rows = sql_rows(
        "SELECT count() AS count FROM memory_entries GROUP ALL;",
        None,
        Some(4_000),
    )?;

    let Some(first) = rows.first() else {
        return Ok(0);
    };

    Ok(first.get("count").and_then(Value::as_u64).unwrap_or(0))
}

fn health_status() -> std::result::Result<bool, String> {
    let payload = sql_query("RETURN 1;", None, Some(2_000))?;
    Ok(payload.get("ok").and_then(Value::as_bool).unwrap_or(false))
}

fn validate_response(
    args: ValidateResponseArgs,
) -> std::result::Result<(bool, Vec<String>), String> {
    let mut violations = Vec::new();
    let session_filter = args
        .session_id
        .as_deref()
        .map(|sid| format!(" AND session_id = {}", surreal_string_literal(sid)))
        .unwrap_or_default();
    let response_text = args.response_text.trim();
    if response_text.is_empty() {
        violations.push("response is empty".to_string());
        return Ok((false, violations));
    }

    let response_lower = response_text.to_lowercase();

    let rules_query =
        format!("SELECT * FROM ontology_rules WHERE enabled = true{session_filter} LIMIT 200;");
    let rules = sql_rows(&rules_query, None, Some(5_000)).unwrap_or_default();

    for rule in rules {
        let Some(rule_type) = rule.get("rule_type").and_then(Value::as_str) else {
            continue;
        };
        let Some(term) = rule.get("term").and_then(Value::as_str) else {
            continue;
        };
        let term_norm = term.trim().to_lowercase();
        if term_norm.is_empty() {
            continue;
        }

        match rule_type {
            "must_include_term" => {
                if !response_lower.contains(&term_norm) {
                    violations.push(format!("missing required domain term '{term}'"));
                }
            }
            "forbid_term" => {
                if response_lower.contains(&term_norm) {
                    violations.push(format!("response contains forbidden domain term '{term}'"));
                }
            }
            _ => {}
        }
    }

    let query_terms = extract_terms(&args.query);
    if !query_terms.is_empty() {
        let has_query_grounding = query_terms
            .iter()
            .any(|term| response_lower.contains(term.as_str()));
        if !has_query_grounding {
            violations.push(
                "response is not grounded in query concepts from the knowledge graph".to_string(),
            );
        }
    }

    Ok((violations.is_empty(), violations))
}

fn sql_rows(
    query: &str,
    vars: Option<Value>,
    timeout_ms: Option<u64>,
) -> std::result::Result<Vec<Value>, String> {
    let payload = sql_query(query, vars, timeout_ms)?;
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let message = payload
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("surreal host returned an error");
        return Err(message.to_string());
    }

    let result = payload.get("result").cloned().unwrap_or(Value::Null);
    if let Some(arr) = result.as_array() {
        if let Some(first_statement) = arr.first() {
            return Ok(first_statement
                .get("result")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default());
        }
    }

    Ok(Vec::new())
}

fn sql_query(
    query: &str,
    vars: Option<Value>,
    timeout_ms: Option<u64>,
) -> std::result::Result<Value, String> {
    let request = HostSqlRequest {
        query: query.to_string(),
        vars,
        timeout_ms,
    };
    let request_bytes = serde_json::to_vec(&request)
        .map_err(|error| format!("failed to encode host sql request: {error}"))?;

    if request_bytes.len() > i32::MAX as usize {
        return Err("host sql request exceeds i32 length".to_string());
    }

    let mut capacity = 0usize;
    for _attempt in 0..3 {
        let response_ptr = if capacity == 0 {
            0
        } else {
            alloc_v1(i32::try_from(capacity).unwrap_or(i32::MAX))
        };

        let packed = unsafe {
            host_sql_v1(
                request_bytes.as_ptr() as i32,
                request_bytes.len() as i32,
                response_ptr,
                i32::try_from(capacity).unwrap_or(i32::MAX),
            )
        };

        let (status, len) = unpack_status_len(packed);
        match status {
            0 => {
                if len == 0 {
                    if response_ptr > 0 {
                        dealloc_v1(response_ptr, i32::try_from(capacity).unwrap_or(i32::MAX));
                    }
                    return Ok(json!({
                        "ok": true,
                        "status": 200,
                        "result": [],
                    }));
                }

                let bytes =
                    unsafe { slice::from_raw_parts(response_ptr as *const u8, len) }.to_vec();
                if response_ptr > 0 {
                    dealloc_v1(response_ptr, i32::try_from(capacity).unwrap_or(i32::MAX));
                }

                let payload = serde_json::from_slice::<Value>(&bytes).map_err(|error| {
                    format!("failed to decode host sql response payload: {error}")
                })?;
                return Ok(payload);
            }
            1 => {
                if response_ptr > 0 {
                    dealloc_v1(response_ptr, i32::try_from(capacity).unwrap_or(i32::MAX));
                }
                capacity = len;
                continue;
            }
            2 => {
                if response_ptr > 0 {
                    dealloc_v1(response_ptr, i32::try_from(capacity).unwrap_or(i32::MAX));
                }
                return Err("host sql query timed out".to_string());
            }
            3 => {
                if response_ptr > 0 {
                    dealloc_v1(response_ptr, i32::try_from(capacity).unwrap_or(i32::MAX));
                }
                return Err("host sql transport/database error".to_string());
            }
            _ => {
                if response_ptr > 0 {
                    dealloc_v1(response_ptr, i32::try_from(capacity).unwrap_or(i32::MAX));
                }
                return Err("host sql returned invalid status".to_string());
            }
        }
    }

    Err("host sql request exceeded resize retries".to_string())
}

fn unpack_status_len(packed: i64) -> (usize, usize) {
    let raw = u64::from_ne_bytes(packed.to_ne_bytes());
    let status = (raw >> 32) as u32;
    let len = (raw & 0xFFFF_FFFF) as u32;
    (status as usize, len as usize)
}

fn extract_terms(text: &str) -> Vec<String> {
    let stop_words = [
        "this", "that", "with", "from", "have", "your", "what", "when", "where", "which", "about",
        "would", "could", "should", "there", "their", "were", "been", "will", "just", "more",
        "some", "then", "than",
    ];

    let mut unique = HashSet::new();
    text.split(|ch: char| !ch.is_alphanumeric())
        .filter_map(|token| {
            let normalized = token.trim().to_lowercase();
            if normalized.len() < 4 {
                return None;
            }
            if stop_words.contains(&normalized.as_str()) {
                return None;
            }
            if unique.insert(normalized.clone()) {
                Some(normalized)
            } else {
                None
            }
        })
        .take(MAX_TERMS)
        .collect()
}

fn normalize_category(raw: &str) -> String {
    match raw.trim() {
        "" => "core".to_string(),
        "core" | "daily" | "conversation" => raw.trim().to_string(),
        other => other.to_string(),
    }
}

fn row_to_entry(row: &Value) -> Option<EntryOut> {
    let id = row
        .get("id")
        .and_then(as_record_id_string)
        .or_else(|| row.get("id").and_then(Value::as_str).map(ToOwned::to_owned))?;
    let key = row.get("key")?.as_str()?.to_string();
    let content = row.get("content")?.as_str()?.to_string();
    let category = row
        .get("category")
        .and_then(Value::as_str)
        .unwrap_or("core")
        .to_string();
    let timestamp = row
        .get("updated_at")
        .and_then(Value::as_str)
        .or_else(|| row.get("timestamp").and_then(Value::as_str))
        .unwrap_or("")
        .to_string();
    let session_id = row
        .get("session_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    Some(EntryOut {
        id,
        key,
        content,
        category,
        timestamp,
        session_id,
        score: None,
    })
}

fn as_record_id_string(value: &Value) -> Option<String> {
    if let Some(id) = value.as_str() {
        return Some(id.to_string());
    }

    let tb = value.get("tb")?.as_str()?;
    let inner_id = value.get("id")?;
    let formatted_id = if let Some(raw) = inner_id.as_str() {
        raw.to_string()
    } else {
        inner_id.to_string()
    };
    Some(format!("{tb}:{formatted_id}"))
}

fn surreal_string_literal(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn option_literal(value: Option<&str>) -> String {
    value
        .map(surreal_string_literal)
        .unwrap_or_else(|| "NONE".to_string())
}

fn sha256_hex(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(digest)
}

fn now_string() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    now.to_string()
}
