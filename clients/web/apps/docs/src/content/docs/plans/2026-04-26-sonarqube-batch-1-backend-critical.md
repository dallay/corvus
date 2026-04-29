---
title: SonarQube Batch 1 Backend Critical Implementation Plan
description: Implementation plan for the first SonarQube remediation batch focused on critical backend Rust issues in the agent runtime.
owner: team-platform
status: draft
lastReviewed: 2026-04-26
appliesTo: agent-runtime Rust remediation
docType: architecture
---

# SonarQube Batch 1 Backend Critical Implementation Plan

> **For agentic workers:** Implement this plan task-by-task using the `dispatching-parallel-agents`
> skill for independent tasks, or execute inline with review checkpoints.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the current Rust CRITICAL SonarQube issues in the agent runtime by reducing cognitive complexity without changing CLI, gateway, orchestration, or security behavior.

**Architecture:** Keep each fix local to its current module and favor extraction over rewrites. The main strategy is to split validation, early-return handling, outcome mapping, and path-argument checks into small pure helpers so Sonar complexity falls while behavior remains byte-for-byte equivalent at the contract level.

**Tech Stack:** Rust, Tokio, Axum, Serde/serde_json, existing inline unit tests in `main.rs`, `gateway/mod.rs`, `gateway/webhook_dispatch.rs`, `security/policy.rs`, and `tools/delegate_launch.rs`, plus `cargo fmt`, `cargo clippy`, and targeted `cargo test`.

---

## File Structure

### Files to modify

- `clients/agent-runtime/src/tools/delegate_launch.rs`
  - Split child request validation/parsing out of `DelegateLaunchTool::execute`.
  - Keep structured validation errors and launch contract unchanged.
- `clients/agent-runtime/src/main.rs`
  - Split code-session fast path handling, run execution, and summary/finalization logic out of `handle_code_command`.
- `clients/agent-runtime/src/gateway/webhook_dispatch.rs`
  - Split handled-ingress mapping logic out of `execute`.
  - Keep webhook terminal outcome mapping unchanged.
- `clients/agent-runtime/src/gateway/mod.rs`
  - Split auth/session/idempotency/dispatcher/legacy branches for `/webhook` and `/web/chat/stream`.
  - Preserve current status codes, JSON bodies, and SSE behavior.
- `clients/agent-runtime/src/security/policy.rs`
  - Split path-token parsing and path safety checks out of `is_segment_valid`.
  - Preserve deny-by-default behavior.

### Existing tests to extend in-place

- `clients/agent-runtime/src/tools/delegate_launch.rs`
- `clients/agent-runtime/src/main.rs`
- `clients/agent-runtime/src/gateway/webhook_dispatch.rs`
- `clients/agent-runtime/src/gateway/mod.rs`
- `clients/agent-runtime/src/security/policy.rs`

### Validation commands

Run from: `clients/agent-runtime`

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test delegate_launch::tests --lib
cargo test cli_shared_ingress_handles_compact_before_agent_execution --lib
cargo test cli_session_command_success_returns_message --lib
cargo test webhook_dispatch::tests --lib
cargo test gateway::mod::tests::legacy_webhook_preview_does_not_emit_synthetic_events_sse --lib
cargo test gateway::mod::tests::legacy_webhook_preview_intercepts_slash_session_commands --lib
cargo test gateway::mod::tests::webhook_non_preview_blocks_approval_and_keeps_session_id --lib
cargo test gateway::mod::tests::webhook_non_preview_unblocks_with_approval_override --lib
cargo test policy::tests --lib
```

Expected result:

- `cargo fmt` exits 0 with no diff.
- `cargo clippy` exits 0.
- All targeted tests pass.

---

## Task 1: Reduce complexity in `delegate_launch.rs`

**Files:**
- Modify: `clients/agent-runtime/src/tools/delegate_launch.rs`
- Test: inline tests in `clients/agent-runtime/src/tools/delegate_launch.rs`

- [ ] **Step 1: Add a focused validation test for child parsing before changing production code**

Add one more validation-oriented test near the existing `delegate_launch` tests:

```rust
#[tokio::test]
async fn rejects_child_without_agent_name_before_dispatch() {
    let result = tool()
        .execute(serde_json::json!({
            "children": [
                { "child_id": "a", "agent_name": "", "prompt": "p" }
            ]
        }))
        .await
        .unwrap();

    assert!(!result.success);
    assert!(result
        .error
        .as_deref()
        .unwrap_or("")
        .contains("agent_name"));
}
```

- [ ] **Step 2: Run the delegate launch test slice before refactoring**

Run:

```bash
cargo test delegate_launch::tests --lib
```

Expected: PASS on current tests, plus PASS on the new one after compilation.

- [ ] **Step 3: Extract child array lookup and validation from `execute`**

Inside `impl DelegateLaunchTool`, add small helpers above `execute`:

```rust
fn parse_children_array<'a>(args: &'a serde_json::Value) -> anyhow::Result<&'a Vec<serde_json::Value>> {
    let children_val = match args.get("children") {
        Some(v) => v,
        None => return Err(anyhow::anyhow!("Missing 'children' parameter")),
    };

    match children_val.as_array() {
        Some(items) if !items.is_empty() => Ok(items),
        _ => Err(anyhow::anyhow!("'children' must be a non-empty array")),
    }
}

fn child_rejects_streaming(item: &serde_json::Value) -> bool {
    item.get("stream").is_some()
        || item.get("stream_results").is_some()
        || item.get("stream_tool_progress").is_some()
}
```

Then replace the inlined `children` extraction in `execute` with:

```rust
let children_arr = match Self::parse_children_array(&args) {
    Ok(items) => items,
    Err(error) => return Ok(Self::validation_error(error.to_string())),
};
```

- [ ] **Step 4: Extract per-child parsing into a single helper**

Add a helper that returns a ready `ChildLaunchRequest` and updates duplicate tracking through a mutable `HashSet` argument:

```rust
fn parse_child_request(
    item: &serde_json::Value,
    launch_index: usize,
    seen_ids: &mut std::collections::HashSet<String>,
) -> anyhow::Result<ChildLaunchRequest> {
    if Self::child_rejects_streaming(item) {
        anyhow::bail!("streaming payloads remain out of scope for this slice");
    }

    let child_id = match item.get("child_id").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => anyhow::bail!("Child at index {launch_index} is missing a non-empty 'child_id'"),
    };

    if !seen_ids.insert(child_id.clone()) {
        anyhow::bail!("Duplicate child_id '{child_id}'");
    }

    let agent_name = match item.get("agent_name").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => anyhow::bail!("Child '{child_id}' is missing a non-empty 'agent_name'"),
    };

    let prompt = match item.get("prompt").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => anyhow::bail!("Child '{child_id}' is missing a non-empty 'prompt'"),
    };

    let context = item
        .get("context")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let execution = item
        .get("execution")
        .cloned()
        .map(serde_json::from_value::<crate::agent::coordinator::ChildExecutionSpec>)
        .transpose()
        .map_err(|error| anyhow::anyhow!("invalid execution metadata: {error}"))?;

    Ok(ChildLaunchRequest {
        child_id: ChildAgentId(child_id),
        agent_name,
        prompt,
        context,
        launch_index: u32::try_from(launch_index).unwrap_or(u32::MAX),
        execution,
    })
}
```

- [ ] **Step 5: Extract remote bridge rejection into a dedicated helper**

Add:

```rust
fn validate_transport(request: &ChildLaunchRequest) -> Option<ToolResult> {
    if request.execution.as_ref().and_then(|spec| spec.transport.clone())
        == Some(CoordinatorTransport::RemoteBridge)
    {
        return Some(Self::structured_validation_error(
            "remote_bridge_deferred",
            "Requested child execution transport 'remote_bridge' is deferred and not available in the local orchestration slice",
        ));
    }

    None
}
```

Then make the loop in `execute` look like:

```rust
for (launch_index, item) in children_arr.iter().enumerate() {
    let child_request = match Self::parse_child_request(item, launch_index, &mut seen_ids) {
        Ok(request) => request,
        Err(error) => return Ok(Self::validation_error(error.to_string())),
    };

    if let Some(result) = Self::validate_transport(&child_request) {
        return Ok(result);
    }

    child_requests.push(child_request);
}
```

- [ ] **Step 6: Re-run delegate launch tests after the extraction**

Run:

```bash
cargo test delegate_launch::tests --lib
```

Expected: PASS. No snapshot/structured output regressions.

---

## Task 2: Reduce complexity in `main.rs` `handle_code_command`

**Files:**
- Modify: `clients/agent-runtime/src/main.rs`
- Test: inline tests in `clients/agent-runtime/src/main.rs`

- [ ] **Step 1: Preserve the handled-ingress fast path with an explicit helper contract**

Add a helper above `handle_code_command`:

```rust
async fn maybe_print_code_fast_path(config: &Config, message: Option<&str>) -> Result<bool> {
    if let Some(raw_message) = message {
        if let Some(result_message) = maybe_handle_cli_handled_ingress(config, raw_message).await? {
            println!("{result_message}");
            return Ok(true);
        }
    }

    Ok(false)
}
```

Then replace the current nested block in `handle_code_command` with:

```rust
if maybe_print_code_fast_path(&config, message.as_deref()).await? {
    return Ok(());
}
```

- [ ] **Step 2: Extract the message-vs-interactive execution path**

Add a code-surface-specific helper instead of keeping the long inline block:

```rust
async fn run_code_message_or_interactive(
    agent: &mut crate::agent::Agent,
    message: Option<String>,
    provider_name: &str,
    model_name: &str,
    session_start: Instant,
) -> Result<()> {
    let Some(message) = message else {
        return agent.run_interactive().await;
    };

    let turn_result = agent
        .turn_with_context(&message, crate::agent::TurnContext::default())
        .await;

    if let Ok(turn_result) = &turn_result {
        if let Some(response) = turn_result.final_text.as_deref() {
            println!("{response}");
        }
        if let Some(err) = cli_blocking_error_from_turn_result(turn_result) {
            finish_cli_session(
                agent,
                provider_name,
                model_name,
                session_start,
                CliSessionSurface::Code,
                "code",
            );
            return Err(err);
        }
    }

    turn_result.map(|_| ())
}
```

- [ ] **Step 3: Simplify `handle_code_command` to orchestration-only flow**

Make the function body use the same shape as the existing agent helper pattern:

```rust
let mut agent = crate::agent::Agent::code_from_config(&config)?;
let session_start = Instant::now();

if override_budget {
    apply_cli_budget_override(&agent, CliSessionSurface::Code)?;
}

agent.record_agent_start_event(&provider_name, &model_name);
let run_result = run_code_message_or_interactive(
    &mut agent,
    message,
    &provider_name,
    &model_name,
    session_start,
)
.await;
finish_cli_session(
    &agent,
    &provider_name,
    &model_name,
    session_start,
    CliSessionSurface::Code,
    "code",
);
run_result
```

This must replace the current long `if let Some(msg)` branch and the duplicated summary/finalization block.

- [ ] **Step 4: Run the inline `main.rs` handled-ingress tests**

Run:

```bash
cargo test cli_shared_ingress_handles_compact_before_agent_execution --lib
cargo test cli_session_command_success_returns_message --lib
cargo test cli_resume_target_without_caller_scope_preserves_denied_error_path --lib
cargo test cli_unknown_slash_like_input_falls_through --lib
cargo test cli_tools_command_returns_effective_tool_listing --lib
```

Expected: PASS. These tests confirm the refactor preserved fast-path behavior and error propagation.

---

## Task 3: Reduce complexity in `webhook_dispatch.rs` `execute`

**Files:**
- Modify: `clients/agent-runtime/src/gateway/webhook_dispatch.rs`
- Test: inline tests in `clients/agent-runtime/src/gateway/webhook_dispatch.rs`

- [ ] **Step 1: Extract handled-ingress mapping into a pure helper**

Add a helper that converts the `HandledIngress` branch into an optional `WebhookTurnResult`:

```rust
fn handled_ingress_to_webhook_result(
    request: &WebhookTurnRequest,
    model: &str,
    handled: HandledIngress,
) -> Option<WebhookTurnResult> {
    match handled {
        HandledIngress::Handled(HandledIngressOutcome::SessionCommandSuccess(success)) => {
            Some(WebhookTurnResult {
                session_id: request.session_id.clone(),
                model: model.to_string(),
                outcome: WebhookTerminalOutcome::Completed,
                response_text: Some(success.message.clone()),
                event_frames: Vec::new(),
                tools_called: Vec::new(),
            })
        }
        HandledIngress::Handled(HandledIngressOutcome::SessionCommandFailure { failure, .. }) => {
            Some(WebhookTurnResult {
                session_id: request.session_id.clone(),
                model: model.to_string(),
                outcome: WebhookTerminalOutcome::Failed,
                response_text: Some(failure.message),
                event_frames: Vec::new(),
                tools_called: Vec::new(),
            })
        }
        HandledIngress::Handled(HandledIngressOutcome::Blocking(blocking)) => {
            Some(map_canonical_result(
                request,
                model,
                CanonicalWebhookResult::Blocking(blocking),
            ))
        }
        HandledIngress::NotHandled => None,
    }
}
```

- [ ] **Step 2: Replace the large ingress match in `execute` with the helper**

Replace the current block beginning at:

```rust
match evaluate_webhook_ingress(...).await {
```

with:

```rust
let handled_ingress = evaluate_webhook_ingress(
    memory.as_ref(),
    &tool_snapshot,
    &request,
    clamped_mode,
)
.await;

if let Some(result) = handled_ingress_to_webhook_result(&request, model, handled_ingress) {
    return result;
}
```

- [ ] **Step 3: Add a narrow test for the extracted helper behavior**

Add a test near the existing webhook dispatch tests:

```rust
#[test]
fn handled_ingress_failure_maps_to_failed_webhook_result() {
    let request = sample_request(WebhookSessionSource::Explicit);
    let handled = HandledIngress::Handled(HandledIngressOutcome::SessionCommandFailure {
        class: crate::pre_execution::SessionCommandFailureClass::Failed,
        failure: crate::session_commands::SessionCommandFailure {
            kind: crate::session_commands::SessionCommandFailureKind::UnknownSession,
            message: "boom".into(),
        },
    });

    let result = handled_ingress_to_webhook_result(&request, "test-model", handled)
        .expect("expected handled result");

    assert_eq!(result.outcome, WebhookTerminalOutcome::Failed);
    assert_eq!(result.response_text.as_deref(), Some("boom"));
}
```

- [ ] **Step 4: Run the webhook dispatch test slice**

Run:

```bash
cargo test webhook_dispatch::tests --lib
```

Expected: PASS. Outcome mapping, SSE sanitization, and budget governance behavior stay identical.

---

## Task 4: Reduce complexity in `security/policy.rs` `is_segment_valid`

**Files:**
- Modify: `clients/agent-runtime/src/security/policy.rs`
- Test: inline tests in `clients/agent-runtime/src/security/policy.rs`

- [ ] **Step 1: Add a focused path-flag regression test**

Add a test that protects the flag-value path parsing behavior before refactoring:

```rust
#[test]
fn command_with_flag_embedded_absolute_path_is_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("grep --file=/etc/passwd foo.txt"));
}
```

- [ ] **Step 2: Extract likely-path and effective-arg parsing into helpers**

Move the nested helper logic out of `is_segment_valid` into private methods:

```rust
fn is_likely_path(arg: &str) -> bool {
    (arg.contains('/') && !arg.contains(':'))
        || arg.starts_with('~')
        || arg.starts_with('.')
        || arg.contains(std::path::MAIN_SEPARATOR)
}

fn effective_path_arg<'a>(arg: &'a str) -> &'a str {
    if arg.starts_with("--") {
        arg.split_once('=').map(|(_, value)| value).unwrap_or(arg)
    } else if arg.starts_with('-') && arg.len() > 2 {
        arg.char_indices()
            .nth(2)
            .map(|(idx, _)| &arg[idx..])
            .unwrap_or("")
    } else {
        arg
    }
}
```

- [ ] **Step 3: Extract path safety validation into a helper**

Add:

```rust
fn is_path_argument_safe(&self, effective_arg: &str) -> bool {
    if !Self::is_likely_path(effective_arg) {
        return true;
    }

    if Path::new(effective_arg)
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
        || (self.workspace_only && (effective_arg.starts_with('/') || effective_arg.starts_with('~')))
    {
        return false;
    }

    !matches_any_forbidden_path(effective_arg, &self.forbidden_paths)
}
```

Then replace the long `for` loop body in `is_segment_valid` with:

```rust
for arg in &normalized_args {
    let effective_arg = Self::effective_path_arg(arg);
    if !self.is_path_argument_safe(effective_arg) {
        return false;
    }
}
```

- [ ] **Step 4: Re-run the policy test slice**

Run:

```bash
cargo test policy::tests --lib
```

Expected: PASS. Read-only blocking, allowlist behavior, path traversal blocking, and medium/high-risk command handling stay unchanged.

---

## Task 5: Reduce complexity in `gateway/mod.rs` `handle_webhook`

**Files:**
- Modify: `clients/agent-runtime/src/gateway/mod.rs`
- Test: inline tests in `clients/agent-runtime/src/gateway/mod.rs`

- [ ] **Step 1: Extract session upsert failure policy into a helper**

Add a helper near `handle_webhook`:

```rust
async fn ensure_webhook_session(
    state: &AppState,
    session_id: &str,
    token_hash: Option<&str>,
    reserved_idempotency_key: Option<&str>,
) -> Option<WebhookResponse> {
    if let Err(error) = state.mem.upsert_session(session_id, token_hash).await {
        if token_hash.is_some() {
            tracing::error!("session upsert failed for token-scoped request: {error:#}");
            release_idempotency_key(state, reserved_idempotency_key, false);
            return Some((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Session tracking failed"})),
            ));
        }
        tracing::debug!("session upsert best-effort failed: {error}");
    }

    None
}
```

- [ ] **Step 2: Extract the dispatcher branch into a helper**

Add:

```rust
async fn execute_dispatcher_webhook(
    state: &AppState,
    config: &Config,
    session_id: &str,
    session_source: webhook_dispatch::WebhookSessionSource,
    token_hash: Option<String>,
    message: &str,
    webhook_body_execution_mode: Option<ExecutionMode>,
    server_execution_mode: ExecutionMode,
    is_preview: bool,
    reserved_idempotency_key: Option<&str>,
) -> WebhookResponse {
    log_webhook_runtime_path(session_id, true, "dispatcher_flag_enabled");
    let dispatch_result = webhook_dispatch::execute(
        config,
        Arc::clone(&state.provider),
        Arc::clone(&state.mem),
        Arc::clone(&state.observer),
        state.cost_tracker.clone(),
        &state.model,
        webhook_dispatch::WebhookTurnRequest {
            session_id: session_id.to_string(),
            session_source,
            caller_token_hash: token_hash.clone(),
            message: message.to_string(),
            execution_mode: resolve_webhook_execution_mode(
                server_execution_mode,
                webhook_body_execution_mode,
            ),
            include_sse_frames: is_preview,
        },
    )
    .await;
    log_webhook_terminal_outcome(
        session_id,
        "dispatcher_agent",
        webhook_outcome_label(&dispatch_result.outcome),
    );
    let (response, persist_idempotency) = webhook_response_from_dispatch_result(dispatch_result);
    release_idempotency_key(state, reserved_idempotency_key, persist_idempotency);
    update_session_activity_if_persisted(state, session_id, token_hash.as_deref(), persist_idempotency).await;
    response
}
```

- [ ] **Step 3: Extract the shared handled-ingress short-circuit path**

Add:

```rust
async fn maybe_execute_legacy_http_ingress(
    state: &AppState,
    session_id: &str,
    session_source: webhook_dispatch::WebhookSessionSource,
    scrubbed_message: &str,
    token_hash: Option<&str>,
    reserved_idempotency_key: Option<&str>,
) -> Option<WebhookResponse> {
    let http_source = webhook_http_source(session_source);
    let maybe_response = maybe_handle_http_ingress(
        state,
        session_id,
        http_source,
        scrubbed_message,
        token_hash,
    )
    .await;

    if let Some((response, persist_idempotency)) = maybe_response {
        release_idempotency_key(state, reserved_idempotency_key, persist_idempotency);
        update_session_activity_if_persisted(state, session_id, token_hash, persist_idempotency).await;
        return Some(response);
    }

    None
}
```

Then use it in both preview and non-preview handled-ingress branches.

- [ ] **Step 4: Replace the main body of `handle_webhook` with a linear orchestrator flow**

The refactored function should keep this shape:

```rust
if let Some(rejection) = webhook_auth_rejection(&state, peer_addr, &headers) {
    return rejection;
}

let webhook_body = match parse_webhook_body(body) {
    Ok(body) => body,
    Err(rejection) => return rejection,
};

let (session_id, session_source) = match resolve_session_id(&headers) {
    Ok(resolved) => resolved,
    Err(response) => return response,
};

let reserved_idempotency_key = match reserve_webhook_idempotency_key(&state, &headers) {
    Ok(key) => key,
    Err(response) => return response,
};

if let Some(response) = ensure_webhook_session(
    &state,
    &session_id,
    token_hash.as_deref(),
    reserved_idempotency_key.as_deref(),
).await {
    return response;
}

if dispatcher_enabled {
    return execute_dispatcher_webhook(...).await;
}

if let Some(response) = maybe_execute_legacy_http_ingress(...).await {
    return response;
}
```

Keep the existing plan-mode and cost-governance fail-closed branches intact after that.

- [ ] **Step 5: Run the most relevant `/webhook` regression tests**

Run:

```bash
cargo test gateway::mod::tests::legacy_webhook_preview_does_not_emit_synthetic_events_sse --lib
cargo test gateway::mod::tests::legacy_webhook_preview_intercepts_slash_session_commands --lib
cargo test gateway::mod::tests::webhook_non_preview_blocks_approval_and_keeps_session_id --lib
cargo test gateway::mod::tests::webhook_non_preview_unblocks_with_approval_override --lib
cargo test gateway::mod::tests::webhook_non_preview_timeout_aborts_with_session_scope --lib
```

Expected: PASS. This confirms no regressions in preview behavior, slash interception, approval blocking, or timeout handling.

---

## Task 6: Reduce complexity in `gateway/mod.rs` `handle_chat_stream`

**Files:**
- Modify: `clients/agent-runtime/src/gateway/mod.rs`
- Test: inline tests in `clients/agent-runtime/src/gateway/mod.rs`

- [ ] **Step 1: Extract stream session setup and tool snapshot creation**

Add a helper:

```rust
async fn prepare_stream_request(
    state: &AppState,
    headers: &HeaderMap,
    body: WebhookJsonBody,
) -> Result<
    (
        WebhookBody,
        String,
        webhook_dispatch::WebhookSessionSource,
        Option<String>,
        Config,
        crate::bootstrap::SlashToolSnapshot,
    ),
    WebhookResponse,
> {
    let webhook_body = parse_webhook_body(body)?;
    let (session_id, session_source) = resolve_session_id(headers)?;
    let token_hash = utils::extract_bearer_token(headers).map(|t| compute_token_hash(&t));

    if let Err(error) = state.mem.upsert_session(&session_id, token_hash.as_deref()).await {
        if token_hash.is_some() {
            tracing::error!("session upsert failed for token-scoped request: {error:#}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Session tracking failed"})),
            ));
        }
        tracing::debug!("session upsert best-effort failed: {error}");
    }

    let config = state.config.lock().clone();
    let tool_snapshot = crate::bootstrap::slash_tool_snapshot_from_config(&config).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to derive effective tool snapshot"})),
        )
    })?;

    Ok((webhook_body, session_id, session_source, token_hash, config, tool_snapshot))
}
```

- [ ] **Step 2: Extract handled-ingress SSE response building**

Move the `if let HandledIngress::Handled(...)` branch into:

```rust
async fn maybe_stream_handled_ingress_response(
    state: &AppState,
    handled_ingress: &crate::pre_execution::HandledIngress,
    session_id: &str,
    token_hash: Option<&str>,
) -> Option<axum::response::Response> {
    // move the current event/status construction here unchanged
}
```

The body of this helper should preserve the current `chunk`, `done`, and `error` event payloads exactly.

- [ ] **Step 3: Replace the nested front half of `handle_chat_stream` with a linear flow**

The function should read like:

```rust
if let Some(rejection) = webhook_auth_rejection(&state, peer_addr, &headers) {
    return Err(rejection);
}

let (webhook_body, session_id, session_source, token_hash, config, tool_snapshot) =
    prepare_stream_request(&state, &headers, body).await?;

let handled_ingress = crate::pre_execution::adapt_handled_ingress(
    crate::pre_execution::evaluate_ingress(
        state.mem.as_ref(),
        &tool_snapshot,
        ingress_context,
        &scrubbed_message,
        true,
    )
    .await,
);

if let Some(response) = maybe_stream_handled_ingress_response(
    &state,
    &handled_ingress,
    &session_id,
    token_hash.as_deref(),
)
.await {
    return Ok(response);
}
```

Leave the dispatcher-vs-legacy stream outcome mapping below that branch unchanged except for variable plumbing.

- [ ] **Step 4: Run the stream router regression test slice**

Run at least the existing stream router tests that exercise `/web/chat/stream` in `gateway/mod.rs`. If there is no single named stream smoke test, run the full gateway lib test slice:

```bash
cargo test gateway::mod::tests --lib
```

Expected: PASS. The SSE contract must remain stable.

---

## Task 7: Full Batch 1 validation

**Files:**
- No additional code changes expected
- Validate all modified Rust files

- [ ] **Step 1: Run formatting**

Run:

```bash
cargo fmt --all -- --check
```

Expected: PASS. If it fails, run `cargo fmt --all`, then re-run the check.

- [ ] **Step 2: Run lints**

Run:

```bash
cargo clippy --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 3: Run the targeted Batch 1 tests together**

Run:

```bash
cargo test delegate_launch::tests --lib && cargo test webhook_dispatch::tests --lib && cargo test policy::tests --lib && cargo test gateway::mod::tests --lib && cargo test cli_shared_ingress_handles_compact_before_agent_execution --lib && cargo test cli_session_command_success_returns_message --lib && cargo test cli_resume_target_without_caller_scope_preserves_denied_error_path --lib && cargo test cli_unknown_slash_like_input_falls_through --lib && cargo test cli_tools_command_returns_effective_tool_listing --lib
```

Expected: PASS.

- [ ] **Step 4: Re-check SonarCloud after code is stable**

Use the SonarQube MCP query for project `dallay_corvus` and confirm the Batch 1 CRITICAL issues no longer appear for:

- `clients/agent-runtime/src/tools/delegate_launch.rs`
- `clients/agent-runtime/src/main.rs`
- `clients/agent-runtime/src/gateway/webhook_dispatch.rs`
- `clients/agent-runtime/src/gateway/mod.rs`
- `clients/agent-runtime/src/security/policy.rs`

Expected: those CRITICAL complexity issues are cleared, or any residual issue count is smaller and traceable to a specific remaining function.

---

## Self-review checklist

- Batch 1 scope only: yes.
- No unrelated dependency additions: required.
- Security posture preserved in gateway/policy code: required.
- All five CRITICAL Rust targets covered by explicit tasks: yes.
- Validation commands are concrete and runnable from `clients/agent-runtime`: yes.
