//! GitHub Copilot provider with OAuth device-flow authentication.
//!
//! Authenticates via GitHub's device code flow (same as VS Code Copilot),
//! then exchanges the OAuth token for short-lived Copilot API keys.
//! Tokens are cached to disk and auto-refreshed.
//!
//! **Note:** This uses VS Code's OAuth client ID (`Iv1.b507a08c87ecfe98`) and
//! editor headers. This is the same approach used by LiteLLM, Codex CLI,
//! and other third-party Copilot integrations. The Copilot token endpoint is
//! private; there is no public OAuth scope or app registration for it.
//! GitHub could change or revoke this at any time, which would break all
//! third-party integrations simultaneously.

use crate::providers::traits::{
    ChatMessage, ChatRequest as ProviderChatRequest, ChatResponse as ProviderChatResponse,
    Provider, ToolCall as ProviderToolCall,
};
use crate::security::SecretStore;
use crate::tools::ToolSpec;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::warn;
use zeroize::Zeroizing;

/// GitHub OAuth client ID for Copilot (VS Code extension).
const GITHUB_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const GITHUB_ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_API_KEY_URL: &str = "https://api.github.com/copilot_internal/v2/token";
const DEFAULT_API: &str = "https://api.githubcopilot.com";
const OAUTH_POLLING_SAFETY_MARGIN_MS: u64 = 3000;
const TOKEN_STORE_ENCRYPTION_ENABLED: bool = true;

// ── Token types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default = "default_interval")]
    interval: u64,
    #[serde(default = "default_expires_in")]
    expires_in: u64,
}

fn default_interval() -> u64 {
    5
}

fn default_expires_in() -> u64 {
    900
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    interval: Option<u64>,
}

fn oauth_poll_delay_secs(interval_secs: u64) -> Duration {
    Duration::from_secs(interval_secs)
        .saturating_add(Duration::from_millis(OAUTH_POLLING_SAFETY_MARGIN_MS))
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiKeyInfo {
    token: String,
    expires_at: i64,
    #[serde(default)]
    endpoints: Option<ApiEndpoints>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiEndpoints {
    api: Option<String>,
}

struct CachedApiKey {
    token: Zeroizing<String>,
    api_endpoint: String,
    expires_at: i64,
}

// ── Chat completions types ───────────────────────────────────────

#[derive(Debug, Serialize)]
struct ApiChatRequest {
    model: String,
    messages: Vec<ApiMessage>,
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<NativeToolSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<NativeToolCall>>,
}

#[derive(Debug, Serialize)]
struct NativeToolSpec {
    #[serde(rename = "type")]
    kind: String,
    function: NativeToolFunctionSpec,
}

#[derive(Debug, Serialize)]
struct NativeToolFunctionSpec {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct NativeToolCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    function: NativeFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
struct NativeFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct ApiChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<NativeToolCall>>,
}

// ── Provider ─────────────────────────────────────────────────────

/// GitHub Copilot provider with automatic OAuth and token refresh.
///
/// On first use, prompts the user to visit github.com/login/device.
/// Tokens are cached to `~/.config/corvus/copilot/` and refreshed
/// automatically.
pub struct CopilotProvider {
    github_token: Option<String>,
    /// Mutex ensures only one caller refreshes tokens at a time,
    /// preventing duplicate device flow prompts or redundant API calls.
    refresh_lock: Arc<Mutex<Option<CachedApiKey>>>,
    http: Client,
    secret_store: SecretStore,
    token_dir: PathBuf,
}

impl CopilotProvider {
    pub fn new(github_token: Option<&str>) -> Self {
        let corvus_dir = directories::ProjectDirs::from("", "", "corvus")
            .map(|dir| dir.config_dir().to_path_buf())
            .unwrap_or_else(|| {
                // Fall back to a user-specific temp directory to avoid
                // shared-directory symlink attacks.
                let user = std::env::var("USER")
                    .or_else(|_| std::env::var("USERNAME"))
                    .unwrap_or_else(|_| "unknown".to_string());
                std::env::temp_dir().join(format!("corvus-copilot-{user}"))
            });
        let token_dir = corvus_dir.join("copilot");

        if let Err(err) = std::fs::create_dir_all(&token_dir) {
            warn!(
                "Failed to create Copilot token directory {:?}: {err}. Token caching is disabled.",
                token_dir
            );
        } else {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                if let Err(err) =
                    std::fs::set_permissions(&corvus_dir, std::fs::Permissions::from_mode(0o700))
                {
                    warn!(
                        "Failed to set Corvus config directory permissions on {:?}: {err}",
                        corvus_dir
                    );
                }

                if let Err(err) =
                    std::fs::set_permissions(&token_dir, std::fs::Permissions::from_mode(0o700))
                {
                    warn!(
                        "Failed to set Copilot token directory permissions on {:?}: {err}",
                        token_dir
                    );
                }
            }

            #[cfg(windows)]
            {
                if let Err(err) = harden_windows_acl(&corvus_dir, true) {
                    warn!(
                        "Failed to harden Corvus config directory ACL on {:?}: {err}",
                        corvus_dir
                    );
                }
                if let Err(err) = harden_windows_acl(&token_dir, true) {
                    warn!(
                        "Failed to harden Copilot token directory ACL on {:?}: {err}",
                        token_dir
                    );
                }
            }
        }

        Self {
            github_token: github_token
                .filter(|token| !token.is_empty())
                .map(String::from),
            refresh_lock: Arc::new(Mutex::new(None)),
            http: Client::builder()
                .timeout(Duration::from_secs(120))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
            secret_store: SecretStore::new(&corvus_dir, TOKEN_STORE_ENCRYPTION_ENABLED),
            token_dir,
        }
    }

    async fn read_token_file_secure(&self, path: &Path) -> Option<Zeroizing<String>> {
        let data = tokio::fs::read_to_string(path).await.ok()?;
        let value = data.trim();
        if value.is_empty() {
            return None;
        }

        match self.secret_store.decrypt(value) {
            Ok(decrypted) => {
                let decrypted = Zeroizing::new(decrypted);
                let token = decrypted.trim();
                if token.is_empty() {
                    None
                } else {
                    Some(Zeroizing::new(token.to_string()))
                }
            }
            Err(err) => {
                warn!("Failed to decrypt Copilot token file {:?}: {err}", path);
                None
            }
        }
    }

    async fn write_token_file_secure(&self, path: &Path, content: &str) {
        let path = path.to_path_buf();
        let path_display = path.display().to_string();
        let content = Zeroizing::new(content.to_string());
        let secret_store = self.secret_store.clone();

        let result = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let encrypted = secret_store.encrypt(content.as_str())?;
            write_file_secure_blocking(&path, &encrypted)?;
            Ok(())
        })
        .await;

        match result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                warn!(
                    "Failed to write secure Copilot token file {}: {err}",
                    path_display
                )
            }
            Err(err) => warn!(
                "Failed to spawn token write task for {}: {err}",
                path_display
            ),
        }
    }

    /// Required headers for Copilot API requests (editor identification).
    const COPILOT_HEADERS: [(&str, &str); 4] = [
        ("Editor-Version", "vscode/1.85.1"),
        ("Editor-Plugin-Version", "copilot/1.155.0"),
        ("User-Agent", "GithubCopilot/1.155.0"),
        ("Accept", "application/json"),
    ];

    fn convert_tools(tools: Option<&[ToolSpec]>) -> Option<Vec<NativeToolSpec>> {
        tools.map(|items| {
            items
                .iter()
                .map(|tool| NativeToolSpec {
                    kind: "function".to_string(),
                    function: NativeToolFunctionSpec {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        parameters: tool.parameters.clone(),
                    },
                })
                .collect()
        })
    }

    fn convert_messages(messages: &[ChatMessage]) -> Vec<ApiMessage> {
        messages
            .iter()
            .map(|message| {
                if message.role == "assistant" {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&message.content) {
                        if let Some(tool_calls_value) = value.get("tool_calls") {
                            if let Ok(parsed_calls) =
                                serde_json::from_value::<Vec<ProviderToolCall>>(tool_calls_value.clone())
                            {
                                let tool_calls = parsed_calls
                                    .into_iter()
                                    .map(|tool_call| NativeToolCall {
                                        id: Some(tool_call.id),
                                        kind: Some("function".to_string()),
                                        function: NativeFunctionCall {
                                            name: tool_call.name,
                                            arguments: tool_call.arguments,
                                        },
                                    })
                                    .collect::<Vec<_>>();

                                let content = value
                                    .get("content")
                                    .and_then(serde_json::Value::as_str)
                                    .map(ToString::to_string);

                                return ApiMessage {
                                    role: "assistant".to_string(),
                                    content,
                                    tool_call_id: None,
                                    tool_calls: Some(tool_calls),
                                };
                            }
                        }
                    }
                }

                if message.role == "tool" {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&message.content) {
                        let tool_call_id = value
                            .get("tool_call_id")
                            .and_then(serde_json::Value::as_str)
                            .map(ToString::to_string);
                        let content = value
                            .get("content")
                            .and_then(serde_json::Value::as_str)
                            .map(ToString::to_string);

                        return ApiMessage {
                            role: "tool".to_string(),
                            content,
                            tool_call_id,
                            tool_calls: None,
                        };
                    }
                }

                ApiMessage {
                    role: message.role.clone(),
                    content: Some(message.content.clone()),
                    tool_call_id: None,
                    tool_calls: None,
                }
            })
            .collect()
    }

    /// Send a chat completions request with required Copilot headers.
    async fn send_chat_request(
        &self,
        messages: Vec<ApiMessage>,
        tools: Option<&[ToolSpec]>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResponse> {
        let (token, endpoint) = self.get_api_key().await?;
        let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));

        let native_tools = Self::convert_tools(tools);
        let request = ApiChatRequest {
            model: model.to_string(),
            messages,
            temperature,
            tool_choice: native_tools.as_ref().map(|_| "auto".to_string()),
            tools: native_tools,
        };

        let mut req = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&request);

        for (header, value) in &Self::COPILOT_HEADERS {
            req = req.header(*header, *value);
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            return Err(super::api_error("GitHub Copilot", response).await);
        }

        let api_response: ApiChatResponse = response.json().await?;
        let choice = api_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No response from GitHub Copilot"))?;

        let tool_calls = choice
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tool_call| ProviderToolCall {
                id: tool_call
                    .id
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                name: tool_call.function.name,
                arguments: tool_call.function.arguments,
            })
            .collect();

        Ok(ProviderChatResponse {
            text: choice.message.content,
            tool_calls,
        })
    }

    /// Get a valid Copilot API key, refreshing or re-authenticating as needed.
    /// Uses a Mutex to ensure only one caller refreshes at a time.
    async fn get_api_key(&self) -> anyhow::Result<(String, String)> {
        let mut cached = self.refresh_lock.lock().await;

        if let Some(cached_key) = cached.as_ref() {
            if chrono::Utc::now().timestamp() + 120 < cached_key.expires_at {
                return Ok((
                    cached_key.token.as_str().to_string(),
                    cached_key.api_endpoint.clone(),
                ));
            }
        }

        if let Some(info) = self.load_api_key_from_disk().await {
            if chrono::Utc::now().timestamp() + 120 < info.expires_at {
                let endpoint = info
                    .endpoints
                    .as_ref()
                    .and_then(|e| e.api.clone())
                    .unwrap_or_else(|| DEFAULT_API.to_string());
                let token = Zeroizing::new(info.token);
                let token_out = token.as_str().to_string();

                *cached = Some(CachedApiKey {
                    token,
                    api_endpoint: endpoint.clone(),
                    expires_at: info.expires_at,
                });
                return Ok((token_out, endpoint));
            }
        }

        let access_token = Zeroizing::new(self.get_github_access_token().await?);
        let api_key_info = self.exchange_for_api_key(access_token.as_ref()).await?;
        self.save_api_key_to_disk(&api_key_info).await;

        let endpoint = api_key_info
            .endpoints
            .as_ref()
            .and_then(|e| e.api.clone())
            .unwrap_or_else(|| DEFAULT_API.to_string());

        let api_token = Zeroizing::new(api_key_info.token);
        let api_token_out = api_token.as_str().to_string();
        *cached = Some(CachedApiKey {
            token: api_token,
            api_endpoint: endpoint.clone(),
            expires_at: api_key_info.expires_at,
        });

        Ok((api_token_out, endpoint))
    }

    /// Get a GitHub access token from config, cache, or device flow.
    async fn get_github_access_token(&self) -> anyhow::Result<String> {
        if let Some(token) = &self.github_token {
            return Ok(token.clone());
        }

        let access_token_path = self.token_dir.join("access-token");
        if let Some(token) = self.read_token_file_secure(&access_token_path).await {
            return Ok(token.as_str().to_string());
        }

        let token = Zeroizing::new(self.device_code_login().await?);
        self.write_token_file_secure(&access_token_path, token.as_ref())
            .await;
        Ok(token.as_str().to_string())
    }

    /// Run GitHub OAuth device code flow.
    async fn device_code_login(&self) -> anyhow::Result<String> {
        let response: DeviceCodeResponse = self
            .http
            .post(GITHUB_DEVICE_CODE_URL)
            .header("User-Agent", "corvus/agent-runtime")
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "client_id": GITHUB_CLIENT_ID,
                "scope": "read:user"
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let mut poll_interval_secs = response.interval.max(5);
        let expires_in = response.expires_in.max(1);
        let expires_at = tokio::time::Instant::now() + Duration::from_secs(expires_in);

        eprintln!(
            "\nGitHub Copilot authentication is required.\n\
             Visit: {}\n\
             Code: {}\n\
             Waiting for authorization...\n",
            response.verification_uri, response.user_code
        );

        while tokio::time::Instant::now() < expires_at {
            tokio::time::sleep(oauth_poll_delay_secs(poll_interval_secs)).await;

            let token_response: AccessTokenResponse = self
                .http
                .post(GITHUB_ACCESS_TOKEN_URL)
                .header("User-Agent", "corvus/agent-runtime")
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "client_id": GITHUB_CLIENT_ID,
                    "device_code": response.device_code,
                    "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
                }))
                .send()
                .await?
                .json()
                .await?;

            if let Some(token) = token_response.access_token {
                eprintln!("Authentication succeeded.\n");
                return Ok(token);
            }

            match token_response.error.as_deref() {
                Some("slow_down") => {
                    poll_interval_secs = token_response
                        .interval
                        .filter(|secs| *secs > 0)
                        .unwrap_or_else(|| poll_interval_secs.saturating_add(5));
                }
                Some("authorization_pending") | None => {}
                Some("expired_token") => {
                    anyhow::bail!("GitHub device authorization expired")
                }
                Some(error) => anyhow::bail!("GitHub auth failed: {error}"),
            }
        }

        anyhow::bail!("Timed out waiting for GitHub authorization")
    }

    /// Exchange a GitHub access token for a Copilot API key.
    async fn exchange_for_api_key(&self, access_token: &str) -> anyhow::Result<ApiKeyInfo> {
        let mut request = self.http.get(GITHUB_API_KEY_URL);
        for (header, value) in &Self::COPILOT_HEADERS {
            request = request.header(*header, *value);
        }
        request = request.header("Authorization", format!("token {access_token}"));

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let sanitized = super::sanitize_api_error(&body);

            if status.as_u16() == 401 || status.as_u16() == 403 {
                let access_token_path = self.token_dir.join("access-token");
                tokio::fs::remove_file(&access_token_path).await.ok();
            }

            anyhow::bail!(
                "Failed to get Copilot API key ({status}): {sanitized}. \
                 Ensure your GitHub account has an active Copilot subscription."
            );
        }

        let info: ApiKeyInfo = response.json().await?;
        Ok(info)
    }

    async fn load_api_key_from_disk(&self) -> Option<ApiKeyInfo> {
        let path = self.token_dir.join("api-key.json");
        let data = self.read_token_file_secure(&path).await?;
        serde_json::from_str(data.as_ref()).ok()
    }

    async fn save_api_key_to_disk(&self, info: &ApiKeyInfo) {
        let path = self.token_dir.join("api-key.json");
        if let Ok(json) = serde_json::to_string_pretty(info).map(Zeroizing::new) {
            self.write_token_file_secure(&path, json.as_ref()).await;
        }
    }
}

#[cfg(windows)]
fn windows_icacls_path() -> PathBuf {
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    PathBuf::from(system_root)
        .join("System32")
        .join("icacls.exe")
}

#[cfg(windows)]
fn harden_windows_acl(path: &Path, is_directory: bool) -> std::io::Result<()> {
    let username = std::env::var("USERNAME").map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "USERNAME is unset; cannot harden ACL",
        )
    })?;
    let grant = if is_directory {
        format!("{username}:(OI)(CI)F")
    } else {
        format!("{username}:(R,W)")
    };

    let output = std::process::Command::new(windows_icacls_path())
        .arg(path)
        .args(["/inheritance:r", "/grant:r"])
        .arg(grant)
        .output()?;

    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "failed to harden ACL via icacls (exit code {:?})",
                output.status.code()
            ),
        ));
    }

    Ok(())
}

#[cfg(windows)]
fn create_secure_file_windows(path: &Path) -> std::io::Result<std::fs::File> {
    use std::ffi::OsStr;
    use std::iter;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::FromRawHandle;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{LocalFree, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL,
    };

    const GENERIC_WRITE: u32 = 0x40000000;
    let path_wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let sddl: Vec<u16> = OsStr::new("D:P(A;;GA;;;OW)")
        .encode_wide()
        .chain(iter::once(0))
        .collect();

    let mut security_descriptor = null_mut();
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1 as u32,
            &mut security_descriptor,
            null_mut(),
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }

    let mut security_attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: security_descriptor,
        bInheritHandle: 0,
    };

    let handle = unsafe {
        CreateFileW(
            path_wide.as_ptr(),
            GENERIC_WRITE,
            0,
            &mut security_attributes,
            CREATE_ALWAYS,
            FILE_ATTRIBUTE_NORMAL,
            0,
        )
    };

    unsafe {
        LocalFree(security_descriptor as isize);
    }

    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }

    let file = unsafe { std::fs::File::from_raw_handle(handle as *mut _) };
    Ok(file)
}

/// Write a file with restrictive owner-only permissions.
/// This is synchronous and intended to run inside `spawn_blocking`.
fn write_file_secure_blocking(path: &Path, content: &str) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }
    #[cfg(windows)]
    {
        use std::io::Write;

        let mut file = create_secure_file_windows(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }
    #[cfg(all(not(unix), not(windows)))]
    {
        let _ = (path, content);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "secure token file permissions are not implemented for this platform",
        ))
    }
}

#[async_trait]
impl Provider for CopilotProvider {
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let mut messages = Vec::new();
        if let Some(system) = system_prompt {
            messages.push(ApiMessage {
                role: "system".to_string(),
                content: Some(system.to_string()),
                tool_call_id: None,
                tool_calls: None,
            });
        }
        messages.push(ApiMessage {
            role: "user".to_string(),
            content: Some(message.to_string()),
            tool_call_id: None,
            tool_calls: None,
        });

        let response = self
            .send_chat_request(messages, None, model, temperature)
            .await?;
        Ok(response.text.unwrap_or_default())
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let response = self
            .send_chat_request(Self::convert_messages(messages), None, model, temperature)
            .await?;
        Ok(response.text.unwrap_or_default())
    }

    async fn chat(
        &self,
        request: ProviderChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResponse> {
        self.send_chat_request(
            Self::convert_messages(request.messages),
            request.tools,
            model,
            temperature,
        )
        .await
    }

    fn supports_native_tools(&self) -> bool {
        true
    }

    async fn warmup(&self) -> anyhow::Result<()> {
        let _ = self.get_api_key().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_without_token() {
        let provider = CopilotProvider::new(None);
        assert!(provider.github_token.is_none());
    }

    #[test]
    fn new_with_token() {
        let provider = CopilotProvider::new(Some("ghp_test"));
        assert_eq!(provider.github_token.as_deref(), Some("ghp_test"));
    }

    #[test]
    fn empty_token_treated_as_none() {
        let provider = CopilotProvider::new(Some(""));
        assert!(provider.github_token.is_none());
    }

    #[tokio::test]
    async fn cache_starts_empty() {
        let provider = CopilotProvider::new(None);
        let cached = provider.refresh_lock.lock().await;
        assert!(cached.is_none());
    }

    #[test]
    fn copilot_headers_include_required_fields() {
        let headers = CopilotProvider::COPILOT_HEADERS;
        let editor_version = headers
            .iter()
            .find(|(header, _)| *header == "Editor-Version")
            .map(|(_, value)| *value);
        let plugin_version = headers
            .iter()
            .find(|(header, _)| *header == "Editor-Plugin-Version")
            .map(|(_, value)| *value);
        let user_agent = headers
            .iter()
            .find(|(header, _)| *header == "User-Agent")
            .map(|(_, value)| *value);

        assert_eq!(editor_version, Some("vscode/1.85.1"));
        assert_eq!(plugin_version, Some("copilot/1.155.0"));
        assert_eq!(user_agent, Some("GithubCopilot/1.155.0"));
    }

    #[test]
    fn default_interval_and_expiry() {
        assert_eq!(default_interval(), 5);
        assert_eq!(default_expires_in(), 900);
    }

    #[test]
    fn oauth_poll_delay_applies_safety_margin() {
        let delay = oauth_poll_delay_secs(5);
        assert_eq!(delay, Duration::from_millis(8000));
    }

    #[test]
    fn supports_native_tools() {
        let provider = CopilotProvider::new(None);
        assert!(provider.supports_native_tools());
    }

    #[test]
    fn convert_tools_maps_function_specs() {
        let tools = vec![ToolSpec {
            name: "sum".to_string(),
            description: "adds two numbers".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "a": { "type": "number" },
                    "b": { "type": "number" }
                },
                "required": ["a", "b"]
            }),
        }];

        let native = CopilotProvider::convert_tools(Some(&tools)).expect("tools must map");
        assert_eq!(native.len(), 1);
        assert_eq!(native[0].kind, "function");
        assert_eq!(native[0].function.name, "sum");
        assert_eq!(native[0].function.description, "adds two numbers");
    }

    #[test]
    fn convert_messages_parses_assistant_tool_calls_payload() {
        let assistant_payload = serde_json::json!({
            "content": "Working on it",
            "tool_calls": [
                {
                    "id": "call_1",
                    "name": "sum",
                    "arguments": "{\"a\":1,\"b\":2}"
                }
            ]
        })
        .to_string();

        let messages = vec![ChatMessage {
            role: "assistant".to_string(),
            content: assistant_payload,
        }];

        let converted = CopilotProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        let tool_calls = converted[0]
            .tool_calls
            .as_ref()
            .expect("tool calls expected");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id.as_deref(), Some("call_1"));
        assert_eq!(tool_calls[0].function.name, "sum");
    }

    #[test]
    fn convert_messages_parses_tool_role_payload() {
        let tool_payload = serde_json::json!({
            "tool_call_id": "call_42",
            "content": "{\"result\":3}"
        })
        .to_string();
        let messages = vec![ChatMessage {
            role: "tool".to_string(),
            content: tool_payload,
        }];

        let converted = CopilotProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "tool");
        assert_eq!(converted[0].tool_call_id.as_deref(), Some("call_42"));
        assert_eq!(converted[0].content.as_deref(), Some("{\"result\":3}"));
    }

    #[test]
    fn convert_messages_falls_back_for_invalid_json() {
        let messages = vec![ChatMessage {
            role: "assistant".to_string(),
            content: "not-json".to_string(),
        }];

        let converted = CopilotProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "assistant");
        assert_eq!(converted[0].content.as_deref(), Some("not-json"));
        assert!(converted[0].tool_calls.is_none());
    }
}
