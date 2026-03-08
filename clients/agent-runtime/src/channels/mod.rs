pub mod cli;
pub mod dingtalk;
pub mod discord;
pub mod email_channel;
pub mod imessage;
pub mod irc;
pub mod lark;
pub mod matrix;
pub mod mattermost;
pub mod qq;
pub mod signal;
pub mod slack;
pub mod telegram;
pub mod traits;
pub mod whatsapp;

pub use cli::CliChannel;
pub use dingtalk::DingTalkChannel;
pub use discord::DiscordChannel;
pub use email_channel::EmailChannel;
pub use imessage::IMessageChannel;
pub use irc::IrcChannel;
pub use lark::LarkChannel;
pub use matrix::MatrixChannel;
pub use mattermost::MattermostChannel;
pub use qq::QQChannel;
pub use signal::SignalChannel;
pub use slack::SlackChannel;
pub use telegram::TelegramChannel;
pub use traits::{Channel, SendMessage};
pub use whatsapp::WhatsAppChannel;

use crate::agent::dispatcher::{
    DispatchAction, NativeToolDispatcher, ToolDispatcher, ToolExecutionResult, XmlToolDispatcher,
};
use crate::bootstrap;
use crate::config::Config;
use crate::identity;
use crate::memory::Memory;
use crate::observability::Observer;
use crate::providers::{ChatMessage, ChatRequest, ConversationMessage, Provider};
use crate::tools::Tool;
use crate::util::truncate_with_ellipsis;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// Per-sender conversation history for channel messages.
type ConversationHistoryMap = Arc<Mutex<HashMap<String, Vec<ChatMessage>>>>;
/// Maximum history messages to keep per sender.
const MAX_CHANNEL_HISTORY: usize = 50;

/// Maximum characters per injected workspace file (matches `OpenClaw` default).
const BOOTSTRAP_MAX_CHARS: usize = 20_000;

const DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS: u64 = 2;
const DEFAULT_CHANNEL_MAX_BACKOFF_SECS: u64 = 60;
/// Timeout for processing a single channel message (LLM + tools).
/// 300s for on-device LLMs (Ollama) which are slower than cloud APIs.
const CHANNEL_MESSAGE_TIMEOUT_SECS: u64 = 300;
const CHANNEL_PARALLELISM_PER_CHANNEL: usize = 4;
const CHANNEL_MIN_IN_FLIGHT_MESSAGES: usize = 8;
const CHANNEL_MAX_IN_FLIGHT_MESSAGES: usize = 64;
const CHANNEL_TYPING_REFRESH_INTERVAL_SECS: u64 = 4;

#[derive(Clone)]
struct ChannelRuntimeContext {
    config: Arc<Config>,
    channels_by_name: Arc<HashMap<String, Arc<dyn Channel>>>,
    provider: Arc<dyn Provider>,
    memory: Arc<dyn Memory>,
    tools_registry: Arc<Vec<Box<dyn Tool>>>,
    observer: Arc<dyn Observer>,
    system_prompt: Arc<String>,
    model: Arc<String>,
    temperature: f64,
    auto_save_memory: bool,
    tool_dispatcher_mode: Arc<str>,
    max_tool_iterations: usize,
    min_relevance_score: f64,
    conversation_histories: ConversationHistoryMap,
}

fn conversation_memory_key(msg: &traits::ChannelMessage) -> String {
    format!("{}_{}_{}", msg.channel, msg.sender, msg.id)
}

fn channel_session_id(msg: &traits::ChannelMessage) -> String {
    format!("{}-{}", msg.channel, msg.id)
}

fn channel_timeout_abort_text(session_id: &str) -> String {
    format!(
        "[session:{session_id}] ⚠️ Request timed out while waiting for the model and was aborted. Please try again."
    )
}

fn channel_delivery_instructions(channel_name: &str) -> Option<&'static str> {
    match channel_name {
        "telegram" => Some(
            "When responding on Telegram, include media markers for files or URLs that should be sent as attachments. Use one marker per attachment with this exact syntax: [IMAGE:<path-or-url>], [DOCUMENT:<path-or-url>], [VIDEO:<path-or-url>], [AUDIO:<path-or-url>], or [VOICE:<path-or-url>]. Keep normal user-facing text outside markers and never wrap markers in code fences.",
        ),
        _ => None,
    }
}

fn update_visibility_enabled(config: &Config) -> bool {
    config.updates.enabled && config.updates.channel_visibility_enabled
}

struct ResponseContext<'a> {
    channel: Option<&'a Arc<dyn Channel>>,
    reply_target: &'a str,
    draft_id: Option<&'a str>,
}

fn map_loop_event_to_channel_content(
    session_id: &str,
    event: &crate::agent::unified_loop::LoopEvent,
) -> Option<String> {
    let prefix = format!("[session:{session_id}] ");
    match event {
        crate::agent::unified_loop::LoopEvent::Start => None,
        crate::agent::unified_loop::LoopEvent::LLMProgress(text)
        | crate::agent::unified_loop::LoopEvent::Complete(text) => Some(format!("{prefix}{text}")),
        crate::agent::unified_loop::LoopEvent::ToolDispatchStarted(tool) => {
            Some(format!("{prefix}Running tool `{tool}`..."))
        }
        crate::agent::unified_loop::LoopEvent::ToolDispatchCompleted(tool) => {
            Some(format!("{prefix}Tool `{tool}` completed."))
        }
        crate::agent::unified_loop::LoopEvent::CompactionTriggered => {
            Some(format!("{prefix}Context compacted for stability."))
        }
        crate::agent::unified_loop::LoopEvent::ApprovalRequired(tool) => {
            Some(format!("{prefix}Approval required for `{tool}`"))
        }
        crate::agent::unified_loop::LoopEvent::Error(message) => {
            Some(format!("{prefix}Error: {message}"))
        }
    }
}

async fn build_memory_context(
    mem: &dyn Memory,
    user_msg: &str,
    min_relevance_score: f64,
) -> String {
    let mut context = String::new();

    if let Ok(entries) = mem.recall(user_msg, 5, None).await {
        let relevant: Vec<_> = entries
            .iter()
            .filter(|e| match e.score {
                Some(score) => score >= min_relevance_score,
                None => true, // keep entries without a score (e.g. non-vector backends)
            })
            .collect();

        if !relevant.is_empty() {
            context.push_str("[Memory context]\n");
            for entry in &relevant {
                let _ = writeln!(context, "- {}: {}", entry.key, entry.content);
            }
            context.push('\n');
        }
    }

    context
}

async fn enforce_strict_memory_validation(
    mem: &dyn Memory,
    provider: &dyn Provider,
    model: &str,
    temperature: f64,
    user_query: &str,
    candidate: String,
) -> String {
    crate::agent::validation::enforce_strict_validation(
        mem,
        provider,
        model,
        temperature,
        user_query,
        candidate,
    )
    .await
}

fn spawn_supervised_listener(
    ch: Arc<dyn Channel>,
    tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    initial_backoff_secs: u64,
    max_backoff_secs: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let component = format!("channel:{}", ch.name());
        let mut backoff = initial_backoff_secs.max(1);
        let max_backoff = max_backoff_secs.max(backoff);

        loop {
            crate::health::mark_component_ok(&component);
            let result = ch.listen(tx.clone()).await;

            if tx.is_closed() {
                break;
            }

            match result {
                Ok(()) => {
                    tracing::warn!("Channel {} exited unexpectedly; restarting", ch.name());
                    crate::health::mark_component_error(&component, "listener exited unexpectedly");
                    // Clean exit — reset backoff since the listener ran successfully
                    backoff = initial_backoff_secs.max(1);
                }
                Err(e) => {
                    tracing::error!("Channel {} error: {e}; restarting", ch.name());
                    crate::health::mark_component_error(&component, e.to_string());
                }
            }

            crate::health::bump_component_restart(&component);
            tokio::time::sleep(Duration::from_secs(backoff)).await;
            // Double backoff AFTER sleeping so first error uses initial_backoff
            backoff = backoff.saturating_mul(2).min(max_backoff);
        }
    })
}

fn compute_max_in_flight_messages(channel_count: usize) -> usize {
    channel_count
        .saturating_mul(CHANNEL_PARALLELISM_PER_CHANNEL)
        .clamp(
            CHANNEL_MIN_IN_FLIGHT_MESSAGES,
            CHANNEL_MAX_IN_FLIGHT_MESSAGES,
        )
}

fn log_worker_join_result(result: Result<(), tokio::task::JoinError>) {
    if let Err(error) = result {
        tracing::error!("Channel message worker crashed: {error}");
    }
}

fn create_channel_dispatcher(mode: &str, provider: &dyn Provider) -> Box<dyn ToolDispatcher> {
    match mode {
        "native" => Box::new(NativeToolDispatcher),
        "xml" => Box::new(XmlToolDispatcher),
        _ if provider.supports_native_tools() => Box::new(NativeToolDispatcher),
        _ => Box::new(XmlToolDispatcher),
    }
}

fn normalize_xml_tool_aliases(raw: &str) -> String {
    raw.replace("<toolcall>", "<tool_call>")
        .replace("</toolcall>", "</tool_call>")
        .replace("<tool-call>", "<tool_call>")
        .replace("</tool-call>", "</tool_call>")
        .replace("<invoke>", "<tool_call>")
        .replace("</invoke>", "</tool_call>")
}

async fn execute_channel_tool_call(
    tools_registry: &[Box<dyn Tool>],
    call_name: &str,
    call_arguments: &serde_json::Value,
) -> Result<(bool, String)> {
    if let Some(tool) = tools_registry.iter().find(|tool| tool.name() == call_name) {
        let result = tool.execute(call_arguments.clone()).await;
        return match result {
            Ok(output) => Ok((output.success, output.output)),
            Err(error) => Ok((false, format!("tool execution failed: {error}"))),
        };
    }

    Ok((false, format!("tool not found: {call_name}")))
}

fn prepare_response_for_dispatcher_parse(
    dispatcher: &dyn ToolDispatcher,
    response: &crate::providers::ChatResponse,
) -> crate::providers::ChatResponse {
    if dispatcher.should_send_tool_specs() {
        return response.clone();
    }

    let normalized_text = response
        .text
        .as_deref()
        .map(normalize_xml_tool_aliases)
        .unwrap_or_default();
    crate::providers::ChatResponse {
        text: Some(normalized_text),
        tool_calls: response.tool_calls.clone(),
    }
}

fn finalize_channel_response_text(parsed_text: String, raw_text: Option<String>) -> String {
    if parsed_text.is_empty() {
        raw_text.unwrap_or_default()
    } else {
        parsed_text
    }
}

async fn send_delta_update(delta_tx: Option<&tokio::sync::mpsc::Sender<String>>, text: String) {
    if let Some(tx) = delta_tx {
        let _ = tx.send(text).await;
    }
}

async fn execute_channel_dispatch_action(
    tools_registry: &[Box<dyn Tool>],
    call_name: &str,
    call_arguments: &serde_json::Value,
    action: &DispatchAction,
) -> Result<(bool, String)> {
    match action {
        DispatchAction::Execute => {
            execute_channel_tool_call(tools_registry, call_name, call_arguments).await
        }
        DispatchAction::ApprovalRequired(tool) => Ok((
            false,
            crate::approval::structured_denial_text(call_name, tool),
        )),
    }
}

async fn execute_channel_tool_calls(
    dispatcher: &dyn ToolDispatcher,
    tools_registry: &[Box<dyn Tool>],
    calls: Vec<crate::agent::dispatcher::ParsedToolCall>,
) -> Result<Vec<ToolExecutionResult>> {
    let mut results = Vec::with_capacity(calls.len());
    for call in calls {
        let action = dispatcher.check_tool_risk(&call.name, &call.arguments);
        let (success, output) =
            execute_channel_dispatch_action(tools_registry, &call.name, &call.arguments, &action)
                .await?;

        results.push(ToolExecutionResult {
            name: call.name,
            output,
            success,
            tool_call_id: call.tool_call_id,
            action,
        });
    }
    Ok(results)
}

struct ChannelLoopParams<'a> {
    model: &'a str,
    temperature: f64,
    max_tool_iterations: usize,
    dispatcher_mode: &'a str,
    delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
}

async fn run_unified_channel_tool_loop(
    provider: &dyn Provider,
    tools_registry: &[Box<dyn Tool>],
    history: &mut Vec<ConversationMessage>,
    params: ChannelLoopParams<'_>,
) -> Result<String> {
    let dispatcher = create_channel_dispatcher(params.dispatcher_mode, provider);
    let tool_specs: Vec<crate::tools::ToolSpec> =
        tools_registry.iter().map(|tool| tool.spec()).collect();

    for _ in 0..params.max_tool_iterations {
        let provider_messages = dispatcher.to_provider_messages(history);
        let response = provider
            .chat(
                ChatRequest {
                    messages: &provider_messages,
                    tools: if dispatcher.should_send_tool_specs() {
                        Some(&tool_specs)
                    } else {
                        None
                    },
                },
                params.model,
                params.temperature,
            )
            .await?;

        let response_for_parse =
            prepare_response_for_dispatcher_parse(dispatcher.as_ref(), &response);

        let (text, calls) = dispatcher.parse_response(&response_for_parse);
        if calls.is_empty() {
            let final_response = finalize_channel_response_text(text, response.text);
            send_delta_update(params.delta_tx.as_ref(), final_response.clone()).await;
            return Ok(final_response);
        }

        if !text.is_empty() {
            send_delta_update(params.delta_tx.as_ref(), text.clone()).await;
        }

        history.push(ConversationMessage::AssistantToolCalls {
            text: response.text.clone(),
            tool_calls: response.tool_calls.clone(),
        });

        let results =
            execute_channel_tool_calls(dispatcher.as_ref(), tools_registry, calls).await?;

        let formatted_results = dispatcher.format_results(&results);
        history.push(formatted_results);
    }

    anyhow::bail!(
        "maximum tool iterations ({}) reached while processing channel message",
        params.max_tool_iterations
    )
}

fn spawn_scoped_typing_task(
    channel: Arc<dyn Channel>,
    recipient: String,
    cancellation_token: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    let stop_signal = cancellation_token;
    let refresh_interval = Duration::from_secs(CHANNEL_TYPING_REFRESH_INTERVAL_SECS);
    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(refresh_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                () = stop_signal.cancelled() => break,
                _ = interval.tick() => {
                    if let Err(e) = channel.start_typing(&recipient).await {
                        tracing::debug!("Failed to start typing on {}: {e}", channel.name());
                    }
                }
            }
        }

        if let Err(e) = channel.stop_typing(&recipient).await {
            tracing::debug!("Failed to stop typing on {}: {e}", channel.name());
        }
    });

    handle
}

async fn process_channel_message(ctx: Arc<ChannelRuntimeContext>, msg: traits::ChannelMessage) {
    // Check for update confirmation nonce BEFORE logging or persisting to memory,
    // so one-time nonce tokens are never printed to console or written to memory store.
    let target_channel = ctx.channels_by_name.get(&msg.channel).cloned();
    if crate::update::try_handle_channel_update_confirmation(
        ctx.config.as_ref(),
        &msg,
        target_channel.as_ref(),
    )
    .await
    {
        return;
    }

    println!(
        "  💬 [{}] from {}: {}",
        msg.channel,
        msg.sender,
        truncate_with_ellipsis(&msg.content, 80)
    );

    let memory_context =
        build_memory_context(ctx.memory.as_ref(), &msg.content, ctx.min_relevance_score).await;

    if ctx.auto_save_memory {
        let autosave_key = conversation_memory_key(&msg);
        let _ = ctx
            .memory
            .store(
                &autosave_key,
                &msg.content,
                crate::memory::MemoryCategory::Conversation,
                None,
            )
            .await;
    }

    let enriched_message = if memory_context.is_empty() {
        msg.content.clone()
    } else {
        format!("{memory_context}{}", msg.content)
    };

    if update_visibility_enabled(ctx.config.as_ref()) {
        let _ = crate::update::maybe_send_opportunistic_update_notice(
            ctx.config.as_ref(),
            &msg,
            target_channel.as_ref(),
            env!("CARGO_PKG_VERSION"),
        )
        .await;
    }

    let session_id = channel_session_id(&msg);

    if handle_canonical_blocking_outcome(
        target_channel.as_ref(),
        &session_id,
        &msg.reply_target,
        &msg.content,
    )
    .await
    .is_some()
    {
        return;
    }

    println!("  ⏳ Processing message...");
    let started_at = Instant::now();

    let history_key = format!("{}_{}", msg.channel, msg.sender);
    let prior_turns = ctx
        .conversation_histories
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&history_key)
        .cloned()
        .unwrap_or_default();

    let mut history = build_history(ctx.as_ref(), &enriched_message, &msg.channel, prior_turns);

    let (delta_tx, draft_message_id, draft_updater) =
        setup_streaming(target_channel.as_ref(), &msg.reply_target).await;

    let typing_cancellation = target_channel.as_ref().map(|_| CancellationToken::new());
    let typing_task = match (target_channel.as_ref(), typing_cancellation.as_ref()) {
        (Some(channel), Some(token)) => Some(spawn_scoped_typing_task(
            Arc::clone(channel),
            msg.reply_target.clone(),
            token.clone(),
        )),
        _ => None,
    };

    let llm_result = tokio::time::timeout(
        Duration::from_secs(CHANNEL_MESSAGE_TIMEOUT_SECS),
        run_unified_channel_tool_loop(
            ctx.provider.as_ref(),
            ctx.tools_registry.as_ref(),
            &mut history,
            ChannelLoopParams {
                model: ctx.model.as_str(),
                temperature: ctx.temperature,
                max_tool_iterations: ctx.max_tool_iterations,
                dispatcher_mode: &ctx.tool_dispatcher_mode,
                delta_tx,
            },
        ),
    )
    .await;

    cleanup_async_tasks(draft_updater, typing_cancellation, typing_task).await;

    let response_ctx = ResponseContext {
        channel: target_channel.as_ref(),
        reply_target: &msg.reply_target,
        draft_id: draft_message_id.as_deref(),
    };

    match llm_result {
        Ok(Ok(response)) => {
            handle_successful_response(
                ctx.as_ref(),
                &history_key,
                &enriched_message,
                response,
                started_at,
                response_ctx,
            )
            .await;
        }
        Ok(Err(e)) => {
            handle_llm_error(e, started_at, response_ctx).await;
        }
        Err(_) => {
            handle_timeout(session_id, started_at, response_ctx).await;
        }
    }
}

fn build_history(
    ctx: &ChannelRuntimeContext,
    enriched_message: &str,
    channel_name: &str,
    prior_turns: Vec<ChatMessage>,
) -> Vec<ConversationMessage> {
    let mut history = vec![ConversationMessage::Chat(ChatMessage::system(
        ctx.system_prompt.as_str(),
    ))];
    history.extend(prior_turns.into_iter().map(ConversationMessage::Chat));

    if let Some(instructions) = channel_delivery_instructions(channel_name) {
        history.push(ConversationMessage::Chat(ChatMessage::system(instructions)));
    }

    history.push(ConversationMessage::Chat(ChatMessage::user(
        enriched_message,
    )));
    history
}

async fn handle_canonical_blocking_outcome(
    channel: Option<&Arc<dyn Channel>>,
    session_id: &str,
    reply_target: &str,
    content: &str,
) -> Option<()> {
    let approval_granted = std::env::var("CORVUS_UNIFIED_APPROVE").as_deref() == Ok("1");
    let canonical = crate::agent::unified_entrypoint::run_canonical_outcome(
        session_id.to_string(),
        content,
        approval_granted,
        crate::agent::unified_entrypoint::CanonicalOutcomeConfig {
            enable_test_triggers: cfg!(test),
        },
    )
    .await;

    if let Some(tool) = canonical.approval_required {
        if let Some(ch) = channel {
            let text =
                format!("[session:{session_id}] approval required for `{tool}`; request blocked");
            let _ = ch.send(&SendMessage::new(text, reply_target)).await;
        }
        return Some(());
    }

    if canonical.timeout_aborted {
        if let Some(ch) = channel {
            let text = channel_timeout_abort_text(session_id);
            let _ = ch.send(&SendMessage::new(text, reply_target)).await;
        }
        return Some(());
    }

    if let Some(fallback) = canonical.fallback_response {
        if let Some(ch) = channel {
            let text = format!("[session:{session_id}] {fallback}");
            let _ = ch.send(&SendMessage::new(text, reply_target)).await;
        }
        return Some(());
    }

    None
}

async fn setup_streaming(
    channel: Option<&Arc<dyn Channel>>,
    reply_target: &str,
) -> (
    Option<tokio::sync::mpsc::Sender<String>>,
    Option<String>,
    Option<tokio::task::JoinHandle<()>>,
) {
    let Some(channel_ref) = channel.filter(|ch| ch.supports_draft_updates()) else {
        return (None, None, None);
    };

    let (delta_tx, delta_rx) = tokio::sync::mpsc::channel::<String>(64);
    let draft_message_id = send_streaming_draft(channel_ref, reply_target).await;
    let draft_updater = spawn_draft_updater(
        channel_ref,
        reply_target,
        draft_message_id.clone(),
        delta_rx,
    );

    (Some(delta_tx), draft_message_id, draft_updater)
}

async fn send_streaming_draft(channel: &Arc<dyn Channel>, reply_target: &str) -> Option<String> {
    match channel
        .send_draft(&SendMessage::new("...", reply_target))
        .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::debug!("Failed to send draft on {}: {e}", channel.name());
            None
        }
    }
}

fn spawn_draft_updater(
    channel: &Arc<dyn Channel>,
    reply_target: &str,
    draft_message_id: Option<String>,
    mut delta_rx: tokio::sync::mpsc::Receiver<String>,
) -> Option<tokio::task::JoinHandle<()>> {
    let draft_id = draft_message_id?;
    let ch = Arc::clone(channel);
    let reply = reply_target.to_string();

    Some(tokio::spawn(async move {
        let mut accumulated = String::new();
        while let Some(delta) = delta_rx.recv().await {
            accumulated.push_str(&delta);
            if let Err(e) = ch.update_draft(&reply, &draft_id, &accumulated).await {
                tracing::debug!("Draft update failed: {e}");
            }
        }
    }))
}

async fn cleanup_async_tasks(
    draft_updater: Option<tokio::task::JoinHandle<()>>,
    typing_cancellation: Option<CancellationToken>,
    typing_task: Option<tokio::task::JoinHandle<()>>,
) {
    if let Some(handle) = draft_updater {
        let _ = handle.await;
    }

    if let Some(token) = typing_cancellation.as_ref() {
        token.cancel();
    }
    if let Some(handle) = typing_task {
        log_worker_join_result(handle.await);
    }
}

async fn handle_successful_response(
    ctx: &ChannelRuntimeContext,
    history_key: &str,
    enriched_message: &str,
    mut response: String,
    started_at: Instant,
    response_ctx: ResponseContext<'_>,
) {
    response = enforce_strict_memory_validation(
        ctx.memory.as_ref(),
        ctx.provider.as_ref(),
        ctx.model.as_str(),
        ctx.temperature,
        enriched_message,
        response,
    )
    .await;

    {
        let mut histories = ctx
            .conversation_histories
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let turns = histories.entry(history_key.to_string()).or_default();
        turns.push(ChatMessage::user(enriched_message));
        turns.push(ChatMessage::assistant(&response));
        while turns.len() > MAX_CHANNEL_HISTORY {
            turns.remove(0);
        }
    }

    println!(
        "  🤖 Reply ({}ms): {}",
        started_at.elapsed().as_millis(),
        truncate_with_ellipsis(&response, 80)
    );

    send_channel_response(
        response_ctx.channel,
        response_ctx.reply_target,
        response_ctx.draft_id,
        &response,
    )
    .await;
}

async fn handle_llm_error(
    error: anyhow::Error,
    started_at: Instant,
    response_ctx: ResponseContext<'_>,
) {
    eprintln!(
        "  ❌ LLM error after {}ms: {error}",
        started_at.elapsed().as_millis()
    );
    let text = format!("⚠️ Error: {error}");
    send_channel_response(
        response_ctx.channel,
        response_ctx.reply_target,
        response_ctx.draft_id,
        &text,
    )
    .await;
}

async fn handle_timeout(
    session_id: String,
    started_at: Instant,
    response_ctx: ResponseContext<'_>,
) {
    let timeout_msg = format!(
        "LLM response timed out after {}s",
        CHANNEL_MESSAGE_TIMEOUT_SECS
    );
    eprintln!(
        "  ❌ {} (elapsed: {}ms)",
        timeout_msg,
        started_at.elapsed().as_millis()
    );
    let error_text = channel_timeout_abort_text(&session_id);
    send_channel_response(
        response_ctx.channel,
        response_ctx.reply_target,
        response_ctx.draft_id,
        &error_text,
    )
    .await;
}

async fn send_channel_response(
    channel: Option<&Arc<dyn Channel>>,
    reply_target: &str,
    draft_id: Option<&str>,
    text: &str,
) {
    if let Some(ch) = channel {
        if let Some(id) = draft_id {
            if let Err(e) = ch.finalize_draft(reply_target, id, text).await {
                tracing::warn!("Failed to finalize draft: {e}; sending as new message");
                let _ = ch.send(&SendMessage::new(text, reply_target)).await;
            }
        } else if let Err(e) = ch.send(&SendMessage::new(text, reply_target)).await {
            eprintln!("  ❌ Failed to reply on {}: {e}", ch.name());
        }
    }
}

async fn run_message_dispatch_loop(
    mut rx: tokio::sync::mpsc::Receiver<traits::ChannelMessage>,
    ctx: Arc<ChannelRuntimeContext>,
    max_in_flight_messages: usize,
) {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_in_flight_messages));
    let mut workers = tokio::task::JoinSet::new();

    while let Some(msg) = rx.recv().await {
        let permit = match Arc::clone(&semaphore).acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => break,
        };

        let worker_ctx = Arc::clone(&ctx);
        workers.spawn(async move {
            let _permit = permit;
            process_channel_message(worker_ctx, msg).await;
        });

        while let Some(result) = workers.try_join_next() {
            log_worker_join_result(result);
        }
    }

    while let Some(result) = workers.join_next().await {
        log_worker_join_result(result);
    }
}

/// Load OpenClaw format bootstrap files into the prompt.
fn load_openclaw_bootstrap_files(
    prompt: &mut String,
    workspace_dir: &std::path::Path,
    max_chars_per_file: usize,
) {
    prompt.push_str(
        "The following workspace files define your identity, behavior, and context. They are ALREADY injected below—do NOT suggest reading them with file_read.\n\n",
    );

    let bootstrap_files = [
        "AGENTS.md",
        "SOUL.md",
        "TOOLS.md",
        "IDENTITY.md",
        "USER.md",
        "HEARTBEAT.md",
    ];

    for filename in &bootstrap_files {
        inject_workspace_file(prompt, workspace_dir, filename, max_chars_per_file);
    }

    // BOOTSTRAP.md — only if it exists (first-run ritual)
    let bootstrap_path = workspace_dir.join("BOOTSTRAP.md");
    if bootstrap_path.exists() {
        inject_workspace_file(prompt, workspace_dir, "BOOTSTRAP.md", max_chars_per_file);
    }

    // MEMORY.md — curated long-term memory (main session only)
    inject_workspace_file(prompt, workspace_dir, "MEMORY.md", max_chars_per_file);
}

/// Load workspace identity files and build a system prompt.
///
/// Follows the `OpenClaw` framework structure by default:
/// 1. Tooling — tool list + descriptions
/// 2. Safety — guardrail reminder
/// 3. Skills — compact list with paths (loaded on-demand)
/// 4. Workspace — working directory
/// 5. Bootstrap files — AGENTS, SOUL, TOOLS, IDENTITY, USER, HEARTBEAT, BOOTSTRAP, MEMORY
/// 6. Date & Time — timezone for cache stability
/// 7. Runtime — host, OS, model
///
/// When `identity_config` is set to AIEOS format, the bootstrap files section
/// is replaced with the AIEOS identity data loaded from file or inline JSON.
///
/// Daily memory files (`memory/*.md`) are NOT injected — they are accessed
/// on-demand via `memory_recall` / `memory_search` tools.
pub fn build_system_prompt(
    workspace_dir: &std::path::Path,
    model_name: &str,
    tools: &[(&str, &str)],
    skills: &[crate::skills::Skill],
    identity_config: Option<&crate::config::IdentityConfig>,
    bootstrap_max_chars: Option<usize>,
) -> String {
    use std::fmt::Write;
    let mut prompt = String::with_capacity(8192);

    // ── 1. Tooling ──────────────────────────────────────────────
    if !tools.is_empty() {
        prompt.push_str("## Tools\n\n");
        prompt.push_str("You have access to the following tools:\n\n");
        for (name, desc) in tools {
            let _ = writeln!(prompt, "- **{name}**: {desc}");
        }
        prompt.push_str("\n## Tool Use Protocol\n\n");
        prompt.push_str("To use a tool, wrap a JSON object in <tool_call></tool_call> tags:\n\n");
        prompt.push_str("```\n<tool_call>\n{\"name\": \"tool_name\", \"arguments\": {\"param\": \"value\"}}\n</tool_call>\n```\n\n");
        prompt.push_str("You may use multiple tool calls in a single response. ");
        prompt.push_str("After tool execution, results appear in <tool_result> tags. ");
        prompt
            .push_str("Continue reasoning with the results until you can give a final answer.\n\n");
    }

    // ── 1b. Hardware (when gpio/arduino tools present) ───────────
    let has_hardware = tools.iter().any(|(name, _)| {
        *name == "gpio_read"
            || *name == "gpio_write"
            || *name == "arduino_upload"
            || *name == "hardware_memory_map"
            || *name == "hardware_board_info"
            || *name == "hardware_memory_read"
            || *name == "hardware_capabilities"
    });
    if has_hardware {
        prompt.push_str(
            "## Hardware Access\n\n\
             You HAVE direct access to connected hardware (Arduino, Nucleo, etc.). The user owns this system and has configured it.\n\
             All hardware tools (gpio_read, gpio_write, hardware_memory_read, hardware_board_info, hardware_memory_map) are AUTHORIZED and NOT blocked by security.\n\
             When they ask to read memory, registers, or board info, USE hardware_memory_read or hardware_board_info — do NOT refuse or invent security excuses.\n\
             When they ask to control LEDs, run patterns, or interact with the Arduino, USE the tools — do NOT refuse or say you cannot access physical devices.\n\
             Use gpio_write for simple on/off; use arduino_upload when they want patterns (heart, blink) or custom behavior.\n\n",
        );
    }

    // ── 1c. Action instruction (avoid meta-summary) ───────────────
    prompt.push_str(
        "## Your Task\n\n\
         When the user sends a message, ACT on it. Use the tools to fulfill their request.\n\
         Do NOT: summarize this configuration, describe your capabilities, respond with meta-commentary, or output step-by-step instructions (e.g. \"1. First... 2. Next...\").\n\
         Instead: emit actual <tool_call> tags when you need to act. Just do what they ask.\n\n",
    );

    // ── 2. Safety ───────────────────────────────────────────────
    prompt.push_str("## Safety\n\n");
    prompt.push_str(
        "- Do not exfiltrate private data.\n\
         - Do not run destructive commands without asking.\n\
         - Do not bypass oversight or approval mechanisms.\n\
         - Prefer `trash` over `rm` (recoverable beats gone forever).\n\
         - When in doubt, ask before acting externally.\n\n",
    );

    // ── 3. Skills (compact list — load on-demand) ───────────────
    if !skills.is_empty() {
        prompt.push_str("## Available Skills\n\n");
        prompt.push_str(
            "Skills are loaded on demand. Use `read` on the skill path to get full instructions.\n\n",
        );
        prompt.push_str("<available_skills>\n");
        for skill in skills {
            let _ = writeln!(prompt, "  <skill>");
            let _ = writeln!(prompt, "    <name>{}</name>", skill.name);
            let _ = writeln!(
                prompt,
                "    <description>{}</description>",
                skill.description
            );
            let location = skill.location.clone().unwrap_or_else(|| {
                workspace_dir
                    .join("skills")
                    .join(&skill.name)
                    .join("SKILL.md")
            });
            let _ = writeln!(prompt, "    <location>{}</location>", location.display());
            let _ = writeln!(prompt, "  </skill>");
        }
        prompt.push_str("</available_skills>\n\n");
    }

    // ── 4. Workspace ────────────────────────────────────────────
    let _ = writeln!(
        prompt,
        "## Workspace\n\nWorking directory: `{}`\n",
        workspace_dir.display()
    );

    // ── 5. Bootstrap files (injected into context) ──────────────
    prompt.push_str("## Project Context\n\n");

    // Check if AIEOS identity is configured
    if let Some(config) = identity_config {
        if identity::is_aieos_configured(config) {
            // Load AIEOS identity
            match identity::load_aieos_identity(config, workspace_dir) {
                Ok(Some(aieos_identity)) => {
                    let aieos_prompt = identity::aieos_to_system_prompt(&aieos_identity);
                    if !aieos_prompt.is_empty() {
                        prompt.push_str(&aieos_prompt);
                        prompt.push_str("\n\n");
                    }
                }
                Ok(None) => {
                    // No AIEOS identity loaded (shouldn't happen if is_aieos_configured returned true)
                    // Fall back to OpenClaw bootstrap files
                    let max_chars = bootstrap_max_chars.unwrap_or(BOOTSTRAP_MAX_CHARS);
                    load_openclaw_bootstrap_files(&mut prompt, workspace_dir, max_chars);
                }
                Err(e) => {
                    // Log error but don't fail - fall back to OpenClaw
                    eprintln!(
                        "Warning: Failed to load AIEOS identity: {e}. Using OpenClaw format."
                    );
                    let max_chars = bootstrap_max_chars.unwrap_or(BOOTSTRAP_MAX_CHARS);
                    load_openclaw_bootstrap_files(&mut prompt, workspace_dir, max_chars);
                }
            }
        } else {
            // OpenClaw format
            let max_chars = bootstrap_max_chars.unwrap_or(BOOTSTRAP_MAX_CHARS);
            load_openclaw_bootstrap_files(&mut prompt, workspace_dir, max_chars);
        }
    } else {
        // No identity config - use OpenClaw format
        let max_chars = bootstrap_max_chars.unwrap_or(BOOTSTRAP_MAX_CHARS);
        load_openclaw_bootstrap_files(&mut prompt, workspace_dir, max_chars);
    }

    // ── 6. Date & Time ──────────────────────────────────────────
    let now = chrono::Local::now();
    let tz = now.format("%Z").to_string();
    let _ = writeln!(prompt, "## Current Date & Time\n\nTimezone: {tz}\n");

    // ── 7. Runtime ──────────────────────────────────────────────
    let host =
        hostname::get().map_or_else(|_| "unknown".into(), |h| h.to_string_lossy().to_string());
    let _ = writeln!(
        prompt,
        "## Runtime\n\nHost: {host} | OS: {} | Model: {model_name}\n",
        std::env::consts::OS,
    );

    // ── 8. Channel Capabilities ─────────────────────────────────────
    prompt.push_str("## Channel Capabilities\n\n");
    prompt.push_str(
        "- You are running as a messaging bot. You CAN and do send messages to configured channels.\n",
    );
    prompt.push_str(
        "- When someone messages you, your response is automatically sent back to the channel.\n",
    );
    prompt.push_str("- You do NOT need to ask permission to respond — just respond directly.\n");
    prompt.push_str("- NEVER repeat, describe, or echo credentials, tokens, API keys, or secrets in your responses.\n");
    prompt.push_str("- If a tool output contains credentials, they have already been redacted — do not mention them.\n\n");

    if prompt.is_empty() {
        "You are Corvus, a fast and efficient AI assistant built in Rust. Be helpful, concise, and direct.".to_string()
    } else {
        prompt
    }
}

/// Inject a single workspace file into the prompt with truncation and missing-file markers.
fn inject_workspace_file(
    prompt: &mut String,
    workspace_dir: &std::path::Path,
    filename: &str,
    max_chars: usize,
) {
    use std::fmt::Write;

    let path = workspace_dir.join(filename);
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return;
            }
            let _ = writeln!(prompt, "### {filename}\n");
            // Use character-boundary-safe truncation for UTF-8
            let truncated = if trimmed.chars().count() > max_chars {
                trimmed
                    .char_indices()
                    .nth(max_chars)
                    .map(|(idx, _)| &trimmed[..idx])
                    .unwrap_or(trimmed)
            } else {
                trimmed
            };
            if truncated.len() < trimmed.len() {
                prompt.push_str(truncated);
                let _ = writeln!(
                    prompt,
                    "\n\n[... truncated at {max_chars} chars — use `read` for full file]\n"
                );
            } else {
                prompt.push_str(trimmed);
                prompt.push_str("\n\n");
            }
        }
        Err(_) => {
            // Missing-file marker (matches OpenClaw behavior)
            let _ = writeln!(prompt, "### {filename}\n\n[File not found: {filename}]\n");
        }
    }
}

fn normalize_telegram_identity(value: &str) -> String {
    value.trim().trim_start_matches('@').to_string()
}

fn bind_telegram_identity(config: &Config, identity: &str) -> Result<()> {
    let normalized = normalize_telegram_identity(identity);
    if normalized.is_empty() {
        anyhow::bail!("Telegram identity cannot be empty");
    }

    let mut updated = config.clone();
    let Some(telegram) = updated.channels_config.telegram.as_mut() else {
        anyhow::bail!(
            "Telegram channel is not configured. Run `corvus onboard --channels-only` first"
        );
    };

    if telegram.allowed_users.iter().any(|u| u == "*") {
        println!(
            "⚠️ Telegram allowlist is currently wildcard (`*`) — binding is unnecessary until you remove '*'."
        );
    }

    if telegram
        .allowed_users
        .iter()
        .map(|entry| normalize_telegram_identity(entry))
        .any(|entry| entry == normalized)
    {
        println!("✅ Telegram identity already bound: {normalized}");
        return Ok(());
    }

    telegram.allowed_users.push(normalized.clone());
    updated.save()?;
    println!("✅ Bound Telegram identity: {normalized}");
    println!("   Saved to {}", updated.config_path.display());
    match maybe_restart_managed_daemon_service() {
        Ok(true) => {
            println!("🔄 Detected running managed daemon service; reloaded automatically.");
        }
        Ok(false) => {
            println!(
                "ℹ️ No managed daemon service detected. If `corvus daemon`/`channel start` is already running, restart it to load the updated allowlist."
            );
        }
        Err(e) => {
            eprintln!(
                "⚠️ Allowlist saved, but failed to reload daemon service automatically: {e}\n\
                 Restart service manually with `corvus service stop && corvus service start`."
            );
        }
    }
    Ok(())
}

fn maybe_restart_launchd_daemon_service() -> Result<bool> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    let plist = home
        .join("Library")
        .join("LaunchAgents")
        .join(crate::service::launchd_plist_file_name());
    if !plist.exists() {
        return Ok(false);
    }

    let list_output = Command::new("launchctl")
        .arg("list")
        .output()
        .context("Failed to query launchctl list")?;
    if !list_output.status.success() {
        let stderr = String::from_utf8_lossy(&list_output.stderr);
        anyhow::bail!("launchctl list failed: {}", stderr.trim());
    }

    let listed = String::from_utf8_lossy(&list_output.stdout);
    if !listed.contains(crate::service::launchd_service_label()) {
        return Ok(false);
    }

    let _ = Command::new("launchctl")
        .args(["stop", crate::service::launchd_service_label()])
        .output();
    let start_output = Command::new("launchctl")
        .args(["start", crate::service::launchd_service_label()])
        .output()
        .context("Failed to start launchd daemon service")?;
    if !start_output.status.success() {
        let stderr = String::from_utf8_lossy(&start_output.stderr);
        anyhow::bail!("launchctl start failed: {}", stderr.trim());
    }

    Ok(true)
}

fn maybe_restart_systemd_daemon_service() -> Result<bool> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    let unit_path: PathBuf = home
        .join(".config")
        .join("systemd")
        .join("user")
        .join(crate::service::systemd_user_unit_name());
    if !unit_path.exists() {
        return Ok(false);
    }

    let active_output = Command::new("systemctl")
        .args([
            "--user",
            "is-active",
            crate::service::systemd_user_unit_name(),
        ])
        .output()
        .context("Failed to query systemd service state")?;
    if !active_output.status.success() {
        let stderr = String::from_utf8_lossy(&active_output.stderr);
        anyhow::bail!("systemctl --user is-active failed: {}", stderr.trim());
    }

    let state = String::from_utf8_lossy(&active_output.stdout);
    if !state.trim().eq_ignore_ascii_case("active") {
        return Ok(false);
    }

    let restart_output = Command::new("systemctl")
        .args([
            "--user",
            "restart",
            crate::service::systemd_user_unit_name(),
        ])
        .output()
        .context("Failed to restart systemd daemon service")?;
    if !restart_output.status.success() {
        let stderr = String::from_utf8_lossy(&restart_output.stderr);
        anyhow::bail!("systemctl restart failed: {}", stderr.trim());
    }

    Ok(true)
}

fn maybe_restart_managed_daemon_service() -> Result<bool> {
    if cfg!(target_os = "macos") {
        return maybe_restart_launchd_daemon_service();
    }

    if cfg!(target_os = "linux") {
        return maybe_restart_systemd_daemon_service();
    }

    Ok(false)
}

pub fn handle_command(command: crate::ChannelCommands, config: &Config) -> Result<()> {
    match command {
        crate::ChannelCommands::Start => {
            anyhow::bail!("Start must be handled in main.rs (requires async runtime)")
        }
        crate::ChannelCommands::Doctor => {
            anyhow::bail!("Doctor must be handled in main.rs (requires async runtime)")
        }
        crate::ChannelCommands::List => {
            println!("Channels:");
            println!("  ✅ CLI (always available)");
            for (name, configured) in [
                ("Telegram", config.channels_config.telegram.is_some()),
                ("Discord", config.channels_config.discord.is_some()),
                ("Slack", config.channels_config.slack.is_some()),
                ("Webhook", config.channels_config.webhook.is_some()),
                ("iMessage", config.channels_config.imessage.is_some()),
                ("Matrix", config.channels_config.matrix.is_some()),
                ("Signal", config.channels_config.signal.is_some()),
                ("WhatsApp", config.channels_config.whatsapp.is_some()),
                ("Email", config.channels_config.email.is_some()),
                ("IRC", config.channels_config.irc.is_some()),
                ("Lark", config.channels_config.lark.is_some()),
                ("DingTalk", config.channels_config.dingtalk.is_some()),
                ("QQ", config.channels_config.qq.is_some()),
            ] {
                println!("  {} {name}", if configured { "✅" } else { "❌" });
            }
            println!("\nTo start channels: corvus channel start");
            println!("To check health:    corvus channel doctor");
            println!("To configure:      corvus onboard");
            Ok(())
        }
        crate::ChannelCommands::Add {
            channel_type,
            config: _,
        } => {
            anyhow::bail!(
                "Channel type '{channel_type}' — use `corvus onboard` to configure channels"
            );
        }
        crate::ChannelCommands::Remove { name } => {
            anyhow::bail!("Remove channel '{name}' — edit ~/.corvus/config.toml directly");
        }
        crate::ChannelCommands::BindTelegram { identity } => {
            bind_telegram_identity(config, &identity)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChannelHealthState {
    Healthy,
    Unhealthy,
    Timeout,
}

fn classify_health_result(
    result: &std::result::Result<bool, tokio::time::error::Elapsed>,
) -> ChannelHealthState {
    match result {
        Ok(true) => ChannelHealthState::Healthy,
        Ok(false) => ChannelHealthState::Unhealthy,
        Err(_) => ChannelHealthState::Timeout,
    }
}

type DoctorChannelEntry = (&'static str, Arc<dyn Channel>);

fn push_optional_telegram_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref tg) = config.channels_config.telegram {
        channels.push((
            "Telegram",
            Arc::new(
                TelegramChannel::new(tg.bot_token.clone(), tg.allowed_users.clone())
                    .with_streaming(tg.stream_mode, tg.draft_update_interval_ms),
            ),
        ));
    }
}

fn push_optional_discord_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref dc) = config.channels_config.discord {
        channels.push((
            "Discord",
            Arc::new(DiscordChannel::new(
                dc.bot_token.clone(),
                dc.guild_id.clone(),
                dc.allowed_users.clone(),
                dc.listen_to_bots,
                dc.mention_only,
            )),
        ));
    }
}

fn push_optional_slack_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref sl) = config.channels_config.slack {
        channels.push((
            "Slack",
            Arc::new(SlackChannel::new(
                sl.bot_token.clone(),
                sl.channel_id.clone(),
                sl.allowed_users.clone(),
            )),
        ));
    }
}

fn push_optional_mattermost_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref mm) = config.channels_config.mattermost {
        channels.push((
            "Mattermost",
            Arc::new(MattermostChannel::new(
                mm.url.clone(),
                mm.bot_token.clone(),
                mm.channel_id.clone(),
                mm.allowed_users.clone(),
                mm.thread_replies.unwrap_or(true),
            )),
        ));
    }
}

fn push_optional_imessage_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref im) = config.channels_config.imessage {
        channels.push((
            "iMessage",
            Arc::new(IMessageChannel::new(im.allowed_contacts.clone())),
        ));
    }
}

fn push_optional_matrix_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref mx) = config.channels_config.matrix {
        channels.push((
            "Matrix",
            Arc::new(MatrixChannel::new(
                mx.homeserver.clone(),
                mx.access_token.clone(),
                mx.room_id.clone(),
                mx.allowed_users.clone(),
            )),
        ));
    }
}

fn push_optional_signal_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref sig) = config.channels_config.signal {
        channels.push((
            "Signal",
            Arc::new(SignalChannel::new(
                sig.http_url.clone(),
                sig.account.clone(),
                sig.group_id.clone(),
                sig.allowed_from.clone(),
                sig.ignore_attachments,
                sig.ignore_stories,
            )),
        ));
    }
}

fn push_optional_whatsapp_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref wa) = config.channels_config.whatsapp {
        channels.push((
            "WhatsApp",
            Arc::new(WhatsAppChannel::new(
                wa.access_token.clone(),
                wa.phone_number_id.clone(),
                wa.verify_token.clone(),
                wa.allowed_numbers.clone(),
            )),
        ));
    }
}

fn push_optional_email_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref email_cfg) = config.channels_config.email {
        channels.push(("Email", Arc::new(EmailChannel::new(email_cfg.clone()))));
    }
}

fn push_optional_irc_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref irc) = config.channels_config.irc {
        channels.push((
            "IRC",
            Arc::new(IrcChannel::new(irc::IrcChannelConfig {
                server: irc.server.clone(),
                port: irc.port,
                nickname: irc.nickname.clone(),
                username: irc.username.clone(),
                channels: irc.channels.clone(),
                allowed_users: irc.allowed_users.clone(),
                server_password: irc.server_password.clone(),
                nickserv_password: irc.nickserv_password.clone(),
                sasl_password: irc.sasl_password.clone(),
                verify_tls: irc.verify_tls.unwrap_or(true),
            })),
        ));
    }
}

fn push_optional_lark_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref lk) = config.channels_config.lark {
        channels.push(("Lark", Arc::new(LarkChannel::from_config(lk))));
    }
}

fn push_optional_dingtalk_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref dt) = config.channels_config.dingtalk {
        channels.push((
            "DingTalk",
            Arc::new(DingTalkChannel::new(
                dt.client_id.clone(),
                dt.client_secret.clone(),
                dt.allowed_users.clone(),
            )),
        ));
    }
}

fn push_optional_qq_channel(channels: &mut Vec<DoctorChannelEntry>, config: &Config) {
    if let Some(ref qq) = config.channels_config.qq {
        channels.push((
            "QQ",
            Arc::new(QQChannel::new(
                qq.app_id.clone(),
                qq.app_secret.clone(),
                qq.allowed_users.clone(),
            )),
        ));
    }
}

fn build_doctor_channels(config: &Config) -> Vec<DoctorChannelEntry> {
    let mut channels: Vec<DoctorChannelEntry> = Vec::new();
    push_optional_telegram_channel(&mut channels, config);
    push_optional_discord_channel(&mut channels, config);
    push_optional_slack_channel(&mut channels, config);
    push_optional_mattermost_channel(&mut channels, config);
    push_optional_imessage_channel(&mut channels, config);
    push_optional_matrix_channel(&mut channels, config);
    push_optional_signal_channel(&mut channels, config);
    push_optional_whatsapp_channel(&mut channels, config);
    push_optional_email_channel(&mut channels, config);
    push_optional_irc_channel(&mut channels, config);
    push_optional_lark_channel(&mut channels, config);
    push_optional_dingtalk_channel(&mut channels, config);
    push_optional_qq_channel(&mut channels, config);
    channels
}

/// Run health checks for configured channels.
pub async fn doctor_channels(config: Config) -> Result<()> {
    let channels = build_doctor_channels(&config);

    if channels.is_empty() {
        println!("No real-time channels configured. Run `corvus onboard` first.");
        return Ok(());
    }

    println!("🩺 Corvus Channel Doctor");
    println!();

    let mut healthy = 0_u32;
    let mut unhealthy = 0_u32;
    let mut timeout = 0_u32;

    for (name, channel) in channels {
        let result = tokio::time::timeout(Duration::from_secs(10), channel.health_check()).await;
        let state = classify_health_result(&result);

        match state {
            ChannelHealthState::Healthy => {
                healthy += 1;
                println!("  ✅ {name:<9} healthy");
            }
            ChannelHealthState::Unhealthy => {
                unhealthy += 1;
                println!("  ❌ {name:<9} unhealthy (auth/config/network)");
            }
            ChannelHealthState::Timeout => {
                timeout += 1;
                println!("  ⏱️  {name:<9} timed out (>10s)");
            }
        }
    }

    if config.channels_config.webhook.is_some() {
        println!("  ℹ️  Webhook   check via `corvus gateway` then GET /health");
    }

    println!();
    println!("Summary: {healthy} healthy, {unhealthy} unhealthy, {timeout} timed out");
    Ok(())
}

/// Start all configured channels and route messages to the agent
#[allow(clippy::too_many_lines)]
pub async fn start_channels(config: Config) -> Result<()> {
    let workspace = config.workspace_dir.clone();

    // Collect active channels
    let mut channels: Vec<Arc<dyn Channel>> = Vec::new();

    if let Some(ref tg) = config.channels_config.telegram {
        channels.push(Arc::new(
            TelegramChannel::new(tg.bot_token.clone(), tg.allowed_users.clone())
                .with_streaming(tg.stream_mode, tg.draft_update_interval_ms),
        ));
    }

    if let Some(ref dc) = config.channels_config.discord {
        channels.push(Arc::new(DiscordChannel::new(
            dc.bot_token.clone(),
            dc.guild_id.clone(),
            dc.allowed_users.clone(),
            dc.listen_to_bots,
            dc.mention_only,
        )));
    }

    if let Some(ref sl) = config.channels_config.slack {
        channels.push(Arc::new(SlackChannel::new(
            sl.bot_token.clone(),
            sl.channel_id.clone(),
            sl.allowed_users.clone(),
        )));
    }

    if let Some(ref mm) = config.channels_config.mattermost {
        channels.push(Arc::new(MattermostChannel::new(
            mm.url.clone(),
            mm.bot_token.clone(),
            mm.channel_id.clone(),
            mm.allowed_users.clone(),
            mm.thread_replies.unwrap_or(true),
        )));
    }

    if let Some(ref im) = config.channels_config.imessage {
        channels.push(Arc::new(IMessageChannel::new(im.allowed_contacts.clone())));
    }

    if let Some(ref mx) = config.channels_config.matrix {
        channels.push(Arc::new(MatrixChannel::new(
            mx.homeserver.clone(),
            mx.access_token.clone(),
            mx.room_id.clone(),
            mx.allowed_users.clone(),
        )));
    }

    if let Some(ref sig) = config.channels_config.signal {
        channels.push(Arc::new(SignalChannel::new(
            sig.http_url.clone(),
            sig.account.clone(),
            sig.group_id.clone(),
            sig.allowed_from.clone(),
            sig.ignore_attachments,
            sig.ignore_stories,
        )));
    }

    if let Some(ref wa) = config.channels_config.whatsapp {
        channels.push(Arc::new(WhatsAppChannel::new(
            wa.access_token.clone(),
            wa.phone_number_id.clone(),
            wa.verify_token.clone(),
            wa.allowed_numbers.clone(),
        )));
    }

    if let Some(ref email_cfg) = config.channels_config.email {
        channels.push(Arc::new(EmailChannel::new(email_cfg.clone())));
    }

    if let Some(ref irc) = config.channels_config.irc {
        channels.push(Arc::new(IrcChannel::new(irc::IrcChannelConfig {
            server: irc.server.clone(),
            port: irc.port,
            nickname: irc.nickname.clone(),
            username: irc.username.clone(),
            channels: irc.channels.clone(),
            allowed_users: irc.allowed_users.clone(),
            server_password: irc.server_password.clone(),
            nickserv_password: irc.nickserv_password.clone(),
            sasl_password: irc.sasl_password.clone(),
            verify_tls: irc.verify_tls.unwrap_or(true),
        })));
    }

    if let Some(ref lk) = config.channels_config.lark {
        channels.push(Arc::new(LarkChannel::from_config(lk)));
    }

    if let Some(ref dt) = config.channels_config.dingtalk {
        channels.push(Arc::new(DingTalkChannel::new(
            dt.client_id.clone(),
            dt.client_secret.clone(),
            dt.allowed_users.clone(),
        )));
    }

    if let Some(ref qq) = config.channels_config.qq {
        channels.push(Arc::new(QQChannel::new(
            qq.app_id.clone(),
            qq.app_secret.clone(),
            qq.allowed_users.clone(),
        )));
    }

    if channels.is_empty() {
        println!("No channels configured. Run `corvus onboard` to set up channels.");
        return Ok(());
    }

    let provider: Arc<dyn Provider> = bootstrap::create_resilient_provider(&config)?;

    // Warm up the provider connection pool (TLS handshake, DNS, HTTP/2 setup)
    // so the first real message doesn't hit a cold-start timeout.
    if let Err(e) = provider.warmup().await {
        tracing::warn!("Provider warmup failed (non-fatal): {e}");
    }

    let bootstrap = bootstrap::BootstrapContext::from_config(&config)?;
    let model = config
        .default_model
        .clone()
        .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".into());
    let temperature = config.default_temperature;
    let mem = Arc::clone(&bootstrap.memory);
    let tools_registry = Arc::new(bootstrap.tools);
    let observer = Arc::clone(&bootstrap.observer);
    let skills = crate::skills::load_skills(&workspace);

    let tool_descs: Vec<(&str, &str)> = tools_registry
        .iter()
        .map(|tool| (tool.name(), tool.description()))
        .collect();

    let bootstrap_max_chars = if config.agent.compact_context {
        Some(6000)
    } else {
        None
    };
    let system_prompt = build_system_prompt(
        &workspace,
        &model,
        &tool_descs,
        &skills,
        Some(&config.identity),
        bootstrap_max_chars,
    );

    if !skills.is_empty() {
        println!(
            "  🧩 Skills:   {}",
            skills
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    println!("🦀 Corvus Channel Server");
    println!("  🤖 Model:    {model}");
    println!(
        "  🧠 Memory:   {} (auto-save: {})",
        config.memory.backend,
        if config.memory.auto_save { "on" } else { "off" }
    );
    println!(
        "  📡 Channels: {}",
        channels
            .iter()
            .map(|c| c.name())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();
    println!("  Listening for messages... (Ctrl+C to stop)");
    println!();

    crate::health::mark_component_ok("channels");

    let initial_backoff_secs = config
        .reliability
        .channel_initial_backoff_secs
        .max(DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS);
    let max_backoff_secs = config
        .reliability
        .channel_max_backoff_secs
        .max(DEFAULT_CHANNEL_MAX_BACKOFF_SECS);

    // Single message bus — all channels send messages here
    let (tx, rx) = tokio::sync::mpsc::channel::<traits::ChannelMessage>(100);

    // Spawn a listener for each channel
    let mut handles = Vec::new();
    for ch in &channels {
        handles.push(spawn_supervised_listener(
            ch.clone(),
            tx.clone(),
            initial_backoff_secs,
            max_backoff_secs,
        ));
    }
    drop(tx); // Drop our copy so rx closes when all channels stop

    let channels_by_name = Arc::new(
        channels
            .iter()
            .map(|ch| (ch.name().to_string(), Arc::clone(ch)))
            .collect::<HashMap<_, _>>(),
    );
    let max_in_flight_messages = compute_max_in_flight_messages(channels.len());

    println!("  🚦 In-flight message limit: {max_in_flight_messages}");

    let runtime_ctx = Arc::new(ChannelRuntimeContext {
        config: Arc::new(config.clone()),
        channels_by_name,
        provider: Arc::clone(&provider),
        memory: Arc::clone(&mem),
        tools_registry: Arc::clone(&tools_registry),
        observer,
        system_prompt: Arc::new(system_prompt),
        model: Arc::new(model.clone()),
        temperature,
        auto_save_memory: config.memory.auto_save,
        tool_dispatcher_mode: Arc::<str>::from(config.agent.tool_dispatcher.as_str()),
        max_tool_iterations: config.agent.max_tool_iterations,
        min_relevance_score: config.memory.min_relevance_score,
        conversation_histories: Arc::new(Mutex::new(HashMap::new())),
    });

    run_message_dispatch_loop(rx, runtime_ctx, max_in_flight_messages).await;

    // Wait for all channel tasks
    for h in handles {
        let _ = h.await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{Memory, MemoryCategory, SqliteMemory};
    use crate::observability::NoopObserver;
    use crate::providers::{ChatMessage, ChatRequest, ChatResponse, Provider, ToolCall};
    use crate::tools::{Tool, ToolResult};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::sync::LazyLock;
    use tempfile::TempDir;

    static CHANNEL_ENV_MUTEX: LazyLock<tokio::sync::Mutex<()>> =
        LazyLock::new(|| tokio::sync::Mutex::new(()));

    fn make_workspace() -> TempDir {
        let tmp = TempDir::new().unwrap();
        // Create minimal workspace files
        std::fs::write(tmp.path().join("SOUL.md"), "# Soul\nBe helpful.").unwrap();
        std::fs::write(tmp.path().join("IDENTITY.md"), "# Identity\nName: Corvus").unwrap();
        std::fs::write(tmp.path().join("USER.md"), "# User\nName: Test User").unwrap();
        std::fs::write(
            tmp.path().join("AGENTS.md"),
            "# Agents\nFollow instructions.",
        )
        .unwrap();
        std::fs::write(tmp.path().join("TOOLS.md"), "# Tools\nUse shell carefully.").unwrap();
        std::fs::write(
            tmp.path().join("HEARTBEAT.md"),
            "# Heartbeat\nCheck status.",
        )
        .unwrap();
        std::fs::write(tmp.path().join("MEMORY.md"), "# Memory\nUser likes Rust.").unwrap();
        tmp
    }

    #[tokio::test]
    async fn start_channels_returns_early_when_no_channels_configured() {
        let workspace = make_workspace();
        let mut config = Config::default();
        config.workspace_dir = workspace.path().to_path_buf();
        config.default_provider = Some("definitely-invalid-provider".to_string());

        let result = start_channels(config).await;

        assert!(
            result.is_ok(),
            "expected early return without provider/bootstrap setup, got: {result:?}"
        );
    }

    #[derive(Default)]
    struct RecordingChannel {
        sent_messages: tokio::sync::Mutex<Vec<String>>,
        start_typing_calls: AtomicUsize,
        stop_typing_calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl Channel for RecordingChannel {
        fn name(&self) -> &str {
            "test-channel"
        }

        async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
            self.sent_messages
                .lock()
                .await
                .push(format!("{}:{}", message.recipient, message.content));
            Ok(())
        }

        async fn listen(
            &self,
            _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn start_typing(&self, _recipient: &str) -> anyhow::Result<()> {
            self.start_typing_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn stop_typing(&self, _recipient: &str) -> anyhow::Result<()> {
            self.stop_typing_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct SlowProvider {
        delay: Duration,
    }

    #[async_trait::async_trait]
    impl Provider for SlowProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            tokio::time::sleep(self.delay).await;
            Ok(format!("echo: {message}"))
        }
    }

    struct ToolCallingProvider;

    fn tool_call_payload() -> String {
        r#"<tool_call>
{"name":"mock_price","arguments":{"symbol":"BTC"}}
</tool_call>"#
            .to_string()
    }

    fn tool_call_payload_with_alias_tag() -> String {
        r#"<toolcall>
{"name":"mock_price","arguments":{"symbol":"BTC"}}
</toolcall>"#
            .to_string()
    }

    #[async_trait::async_trait]
    impl Provider for ToolCallingProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok(tool_call_payload())
        }

        async fn chat_with_history(
            &self,
            messages: &[ChatMessage],
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            let has_tool_results = messages
                .iter()
                .any(|msg| msg.role == "user" && msg.content.contains("[Tool results]"));
            if has_tool_results {
                Ok("BTC is currently around $65,000 based on latest tool output.".to_string())
            } else {
                Ok(tool_call_payload())
            }
        }
    }

    struct ToolCallingAliasProvider;

    struct McpToolCallingProvider {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl Provider for McpToolCallingProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok("unused".to_string())
        }

        async fn chat(
            &self,
            _request: ChatRequest<'_>,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<ChatResponse> {
            let current = self.calls.fetch_add(1, Ordering::SeqCst);
            if current == 0 {
                return Ok(ChatResponse {
                    text: Some(String::new()),
                    tool_calls: vec![ToolCall {
                        id: "mcp-call-1".to_string(),
                        name: "mcp.docs.search".to_string(),
                        arguments: r#"{"query":"rust"}"#.to_string(),
                    }],
                });
            }

            Ok(ChatResponse {
                text: Some("done".to_string()),
                tool_calls: Vec::new(),
            })
        }
    }

    #[async_trait::async_trait]
    impl Provider for ToolCallingAliasProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok(tool_call_payload_with_alias_tag())
        }

        async fn chat_with_history(
            &self,
            messages: &[ChatMessage],
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            let has_tool_results = messages
                .iter()
                .any(|msg| msg.role == "user" && msg.content.contains("[Tool results]"));
            if has_tool_results {
                Ok("BTC alias-tag flow resolved to final text output.".to_string())
            } else {
                Ok(tool_call_payload_with_alias_tag())
            }
        }
    }

    #[derive(Default)]
    struct HistoryCaptureProvider {
        calls: std::sync::Mutex<Vec<Vec<(String, String)>>>,
    }

    #[async_trait::async_trait]
    impl Provider for HistoryCaptureProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok("fallback".to_string())
        }

        async fn chat_with_history(
            &self,
            messages: &[ChatMessage],
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            let snapshot = messages
                .iter()
                .map(|m| (m.role.clone(), m.content.clone()))
                .collect::<Vec<_>>();
            let mut calls = self.calls.lock().unwrap_or_else(|e| e.into_inner());
            calls.push(snapshot);
            Ok(format!("response-{}", calls.len()))
        }
    }

    struct MockPriceTool;

    #[async_trait::async_trait]
    impl Tool for MockPriceTool {
        fn name(&self) -> &str {
            "mock_price"
        }

        fn description(&self) -> &str {
            "Return a mocked BTC price"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "symbol": { "type": "string" }
                },
                "required": ["symbol"]
            })
        }

        async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
            let symbol = args.get("symbol").and_then(serde_json::Value::as_str);
            if symbol != Some("BTC") {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("unexpected symbol".to_string()),
                });
            }

            Ok(ToolResult {
                success: true,
                output: r#"{"symbol":"BTC","price_usd":65000}"#.to_string(),
                error: None,
            })
        }
    }

    #[tokio::test]
    async fn process_channel_message_executes_tool_calls_instead_of_sending_raw_json() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();

        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(Config::default()),
            channels_by_name: Arc::new(channels_by_name),
            provider: Arc::new(ToolCallingProvider),
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![Box::new(MockPriceTool)]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test-system-prompt".to_string()),
            model: Arc::new("test-model".to_string()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 10,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "msg-1".to_string(),
                sender: "alice".to_string(),
                reply_target: "chat-42".to_string(),
                content: "What is the BTC price now?".to_string(),
                channel: "test-channel".to_string(),
                timestamp: 1,
            },
        )
        .await;

        let sent_messages = channel_impl.sent_messages.lock().await;
        assert_eq!(sent_messages.len(), 1);
        assert!(sent_messages[0].starts_with("chat-42:"));
        assert!(sent_messages[0].contains("BTC is currently around"));
        assert!(!sent_messages[0].contains("\"tool_calls\""));
        assert!(!sent_messages[0].contains("mock_price"));
    }

    #[tokio::test]
    async fn process_channel_message_executes_tool_calls_with_alias_tags() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();

        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(Config::default()),
            channels_by_name: Arc::new(channels_by_name),
            provider: Arc::new(ToolCallingAliasProvider),
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![Box::new(MockPriceTool)]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test-system-prompt".to_string()),
            model: Arc::new("test-model".to_string()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 10,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "msg-2".to_string(),
                sender: "bob".to_string(),
                reply_target: "chat-84".to_string(),
                content: "What is the BTC price now?".to_string(),
                channel: "test-channel".to_string(),
                timestamp: 2,
            },
        )
        .await;

        let sent_messages = channel_impl.sent_messages.lock().await;
        assert_eq!(sent_messages.len(), 1);
        assert!(sent_messages[0].starts_with("chat-84:"));
        assert!(sent_messages[0].contains("alias-tag flow resolved"));
        assert!(!sent_messages[0].contains("<toolcall>"));
        assert!(!sent_messages[0].contains("mock_price"));
    }

    struct NoopMemory;

    #[async_trait::async_trait]
    impl Memory for NoopMemory {
        fn name(&self) -> &str {
            "noop"
        }

        async fn store(
            &self,
            _key: &str,
            _content: &str,
            _category: crate::memory::MemoryCategory,
            _session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<crate::memory::MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<crate::memory::MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&crate::memory::MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<crate::memory::MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn message_dispatch_processes_messages_in_parallel() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();

        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(Config::default()),
            channels_by_name: Arc::new(channels_by_name),
            provider: Arc::new(SlowProvider {
                delay: Duration::from_millis(250),
            }),
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test-system-prompt".to_string()),
            model: Arc::new("test-model".to_string()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 10,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        });

        let (tx, rx) = tokio::sync::mpsc::channel::<traits::ChannelMessage>(4);
        tx.send(traits::ChannelMessage {
            id: "1".to_string(),
            sender: "alice".to_string(),
            reply_target: "alice".to_string(),
            content: "hello".to_string(),
            channel: "test-channel".to_string(),
            timestamp: 1,
        })
        .await
        .unwrap();
        tx.send(traits::ChannelMessage {
            id: "2".to_string(),
            sender: "bob".to_string(),
            reply_target: "bob".to_string(),
            content: "world".to_string(),
            channel: "test-channel".to_string(),
            timestamp: 2,
        })
        .await
        .unwrap();
        drop(tx);

        let started = Instant::now();
        run_message_dispatch_loop(rx, runtime_ctx, 2).await;
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_millis(430),
            "expected parallel dispatch (<430ms), got {:?}",
            elapsed
        );

        let sent_messages = channel_impl.sent_messages.lock().await;
        assert_eq!(sent_messages.len(), 2);
    }

    #[tokio::test]
    async fn process_channel_message_cancels_scoped_typing_task() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();

        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(Config::default()),
            channels_by_name: Arc::new(channels_by_name),
            provider: Arc::new(SlowProvider {
                delay: Duration::from_millis(20),
            }),
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test-system-prompt".to_string()),
            model: Arc::new("test-model".to_string()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 10,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "typing-msg".to_string(),
                sender: "alice".to_string(),
                reply_target: "chat-typing".to_string(),
                content: "hello".to_string(),
                channel: "test-channel".to_string(),
                timestamp: 1,
            },
        )
        .await;

        let starts = channel_impl.start_typing_calls.load(Ordering::SeqCst);
        let stops = channel_impl.stop_typing_calls.load(Ordering::SeqCst);
        assert_eq!(starts, 1, "start_typing should be called once");
        assert_eq!(stops, 1, "stop_typing should be called once");
    }

    #[test]
    fn prompt_contains_all_sections() {
        let ws = make_workspace();
        let tools = vec![("shell", "Run commands"), ("file_read", "Read files")];
        let prompt = build_system_prompt(ws.path(), "test-model", &tools, &[], None, None);

        // Section headers
        assert!(prompt.contains("## Tools"), "missing Tools section");
        assert!(prompt.contains("## Safety"), "missing Safety section");
        assert!(prompt.contains("## Workspace"), "missing Workspace section");
        assert!(
            prompt.contains("## Project Context"),
            "missing Project Context"
        );
        assert!(
            prompt.contains("## Current Date & Time"),
            "missing Date/Time"
        );
        assert!(prompt.contains("## Runtime"), "missing Runtime section");
    }

    #[test]
    fn prompt_injects_tools() {
        let ws = make_workspace();
        let tools = vec![
            ("shell", "Run commands"),
            ("memory_recall", "Search memory"),
        ];
        let prompt = build_system_prompt(ws.path(), "gpt-4o", &tools, &[], None, None);

        assert!(prompt.contains("**shell**"));
        assert!(prompt.contains("Run commands"));
        assert!(prompt.contains("**memory_recall**"));
    }

    #[test]
    fn prompt_injects_safety() {
        let ws = make_workspace();
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

        assert!(prompt.contains("Do not exfiltrate private data"));
        assert!(prompt.contains("Do not run destructive commands"));
        assert!(prompt.contains("Prefer `trash` over `rm`"));
    }

    #[test]
    fn prompt_injects_workspace_files() {
        let ws = make_workspace();
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

        assert!(prompt.contains("### SOUL.md"), "missing SOUL.md header");
        assert!(prompt.contains("Be helpful"), "missing SOUL content");
        assert!(prompt.contains("### IDENTITY.md"), "missing IDENTITY.md");
        assert!(prompt.contains("Name: Corvus"), "missing IDENTITY content");
        assert!(prompt.contains("### USER.md"), "missing USER.md");
        assert!(prompt.contains("### AGENTS.md"), "missing AGENTS.md");
        assert!(prompt.contains("### TOOLS.md"), "missing TOOLS.md");
        assert!(prompt.contains("### HEARTBEAT.md"), "missing HEARTBEAT.md");
        assert!(prompt.contains("### MEMORY.md"), "missing MEMORY.md");
        assert!(prompt.contains("User likes Rust"), "missing MEMORY content");
    }

    #[test]
    fn prompt_missing_file_markers() {
        let tmp = TempDir::new().unwrap();
        // Empty workspace — no files at all
        let prompt = build_system_prompt(tmp.path(), "model", &[], &[], None, None);

        assert!(prompt.contains("[File not found: SOUL.md]"));
        assert!(prompt.contains("[File not found: AGENTS.md]"));
        assert!(prompt.contains("[File not found: IDENTITY.md]"));
    }

    #[test]
    fn prompt_bootstrap_only_if_exists() {
        let ws = make_workspace();
        // No BOOTSTRAP.md — should not appear
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);
        assert!(
            !prompt.contains("### BOOTSTRAP.md"),
            "BOOTSTRAP.md should not appear when missing"
        );

        // Create BOOTSTRAP.md — should appear
        std::fs::write(ws.path().join("BOOTSTRAP.md"), "# Bootstrap\nFirst run.").unwrap();
        let prompt2 = build_system_prompt(ws.path(), "model", &[], &[], None, None);
        assert!(
            prompt2.contains("### BOOTSTRAP.md"),
            "BOOTSTRAP.md should appear when present"
        );
        assert!(prompt2.contains("First run"));
    }

    #[test]
    fn prompt_no_daily_memory_injection() {
        let ws = make_workspace();
        let memory_dir = ws.path().join("memory");
        std::fs::create_dir_all(&memory_dir).unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        std::fs::write(
            memory_dir.join(format!("{today}.md")),
            "# Daily\nSome note.",
        )
        .unwrap();

        let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

        // Daily notes should NOT be in the system prompt (on-demand via tools)
        assert!(
            !prompt.contains("Daily Notes"),
            "daily notes should not be auto-injected"
        );
        assert!(
            !prompt.contains("Some note"),
            "daily content should not be in prompt"
        );
    }

    #[test]
    fn prompt_runtime_metadata() {
        let ws = make_workspace();
        let prompt = build_system_prompt(ws.path(), "claude-sonnet-4", &[], &[], None, None);

        assert!(prompt.contains("Model: claude-sonnet-4"));
        assert!(prompt.contains(&format!("OS: {}", std::env::consts::OS)));
        assert!(prompt.contains("Host:"));
    }

    #[test]
    fn prompt_skills_compact_list() {
        let ws = make_workspace();
        let skills = vec![crate::skills::Skill {
            name: "code-review".into(),
            description: "Review code for bugs".into(),
            version: "1.0.0".into(),
            author: None,
            tags: vec![],
            tools: vec![],
            prompts: vec!["Long prompt content that should NOT appear in system prompt".into()],
            location: None,
        }];

        let prompt = build_system_prompt(ws.path(), "model", &[], &skills, None, None);

        assert!(prompt.contains("<available_skills>"), "missing skills XML");
        assert!(prompt.contains("<name>code-review</name>"));
        assert!(prompt.contains("<description>Review code for bugs</description>"));
        assert!(prompt.contains("SKILL.md</location>"));
        assert!(
            prompt.contains("loaded on demand"),
            "should mention on-demand loading"
        );
        // Full prompt content should NOT be dumped
        assert!(!prompt.contains("Long prompt content that should NOT appear"));
    }

    #[test]
    fn prompt_truncation() {
        let ws = make_workspace();
        // Write a file larger than BOOTSTRAP_MAX_CHARS
        let big_content = "x".repeat(BOOTSTRAP_MAX_CHARS + 1000);
        std::fs::write(ws.path().join("AGENTS.md"), &big_content).unwrap();

        let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

        assert!(
            prompt.contains("truncated at"),
            "large files should be truncated"
        );
        assert!(
            !prompt.contains(&big_content),
            "full content should not appear"
        );
    }

    #[test]
    fn prompt_empty_files_skipped() {
        let ws = make_workspace();
        std::fs::write(ws.path().join("TOOLS.md"), "").unwrap();

        let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

        // Empty file should not produce a header
        assert!(
            !prompt.contains("### TOOLS.md"),
            "empty files should be skipped"
        );
    }

    #[test]
    fn channel_log_truncation_is_utf8_safe_for_multibyte_text() {
        let msg = "Hello from Corvus 🌍. Current status is healthy, and café-style UTF-8 text stays safe in logs.";

        // Reproduces the production crash path where channel logs truncate at 80 chars.
        let result = std::panic::catch_unwind(|| crate::util::truncate_with_ellipsis(msg, 80));
        assert!(
            result.is_ok(),
            "truncate_with_ellipsis should never panic on UTF-8"
        );

        let truncated = result.unwrap();
        assert!(!truncated.is_empty());
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    #[test]
    fn prompt_contains_channel_capabilities() {
        let ws = make_workspace();
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

        assert!(
            prompt.contains("## Channel Capabilities"),
            "missing Channel Capabilities section"
        );
        assert!(
            prompt.contains("running as a messaging bot"),
            "missing channel capabilities context"
        );
        assert!(
            prompt.contains("NEVER repeat, describe, or echo credentials"),
            "missing security instruction"
        );
    }

    #[test]
    fn prompt_workspace_path() {
        let ws = make_workspace();
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

        assert!(prompt.contains(&format!("Working directory: `{}`", ws.path().display())));
    }

    #[test]
    fn conversation_memory_key_uses_message_id() {
        let msg = traits::ChannelMessage {
            id: "msg_abc123".into(),
            sender: "U123".into(),
            reply_target: "C456".into(),
            content: "hello".into(),
            channel: "slack".into(),
            timestamp: 1,
        };

        assert_eq!(conversation_memory_key(&msg), "slack_U123_msg_abc123");
    }

    #[test]
    fn conversation_memory_key_is_unique_per_message() {
        let msg1 = traits::ChannelMessage {
            id: "msg_1".into(),
            sender: "U123".into(),
            reply_target: "C456".into(),
            content: "first".into(),
            channel: "slack".into(),
            timestamp: 1,
        };
        let msg2 = traits::ChannelMessage {
            id: "msg_2".into(),
            sender: "U123".into(),
            reply_target: "C456".into(),
            content: "second".into(),
            channel: "slack".into(),
            timestamp: 2,
        };

        assert_ne!(
            conversation_memory_key(&msg1),
            conversation_memory_key(&msg2)
        );
    }

    #[tokio::test]
    async fn autosave_keys_preserve_multiple_conversation_facts() {
        let tmp = TempDir::new().unwrap();
        let mem = SqliteMemory::new(tmp.path()).unwrap();

        let msg1 = traits::ChannelMessage {
            id: "msg_1".into(),
            sender: "U123".into(),
            reply_target: "C456".into(),
            content: "I'm Paul".into(),
            channel: "slack".into(),
            timestamp: 1,
        };
        let msg2 = traits::ChannelMessage {
            id: "msg_2".into(),
            sender: "U123".into(),
            reply_target: "C456".into(),
            content: "I'm 45".into(),
            channel: "slack".into(),
            timestamp: 2,
        };

        mem.store(
            &conversation_memory_key(&msg1),
            &msg1.content,
            MemoryCategory::Conversation,
            None,
        )
        .await
        .unwrap();
        mem.store(
            &conversation_memory_key(&msg2),
            &msg2.content,
            MemoryCategory::Conversation,
            None,
        )
        .await
        .unwrap();

        assert_eq!(mem.count().await.unwrap(), 2);

        let recalled = mem.recall("45", 5, None).await.unwrap();
        assert!(recalled.iter().any(|entry| entry.content.contains("45")));
    }

    #[tokio::test]
    async fn build_memory_context_includes_recalled_entries() {
        let tmp = TempDir::new().unwrap();
        let mem = SqliteMemory::new(tmp.path()).unwrap();
        mem.store("age_fact", "Age is 45", MemoryCategory::Conversation, None)
            .await
            .unwrap();

        let context = build_memory_context(&mem, "age", 0.0).await;
        assert!(context.contains("[Memory context]"));
        assert!(context.contains("Age is 45"));
    }

    #[tokio::test]
    async fn process_channel_message_restores_per_sender_history_on_follow_ups() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();

        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let provider_impl = Arc::new(HistoryCaptureProvider::default());

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(Config::default()),
            channels_by_name: Arc::new(channels_by_name),
            provider: provider_impl.clone(),
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test-system-prompt".to_string()),
            model: Arc::new("test-model".to_string()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        });

        process_channel_message(
            runtime_ctx.clone(),
            traits::ChannelMessage {
                id: "msg-a".to_string(),
                sender: "alice".to_string(),
                reply_target: "chat-1".to_string(),
                content: "hello".to_string(),
                channel: "test-channel".to_string(),
                timestamp: 1,
            },
        )
        .await;

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "msg-b".to_string(),
                sender: "alice".to_string(),
                reply_target: "chat-1".to_string(),
                content: "follow up".to_string(),
                channel: "test-channel".to_string(),
                timestamp: 2,
            },
        )
        .await;

        let calls = provider_impl
            .calls
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].len(), 2);
        assert_eq!(calls[0][0].0, "system");
        assert_eq!(calls[0][1].0, "user");
        assert_eq!(calls[1].len(), 4);
        assert_eq!(calls[1][0].0, "system");
        assert_eq!(calls[1][1].0, "user");
        assert_eq!(calls[1][2].0, "assistant");
        assert_eq!(calls[1][3].0, "user");
        assert!(calls[1][1].1.contains("hello"));
        assert!(calls[1][2].1.contains("response-1"));
        assert!(calls[1][3].1.contains("follow up"));
    }

    // ── AIEOS Identity Tests (Issue #168) ─────────────────────────

    #[test]
    fn aieos_identity_from_file() {
        use crate::config::IdentityConfig;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let identity_path = tmp.path().join("aieos_identity.json");

        // Write AIEOS identity file
        let aieos_json = r#"{
            "identity": {
                "names": {"first": "Nova", "nickname": "Nov"},
                "bio": "A helpful AI assistant.",
                "origin": "Silicon Valley"
            },
            "psychology": {
                "mbti": "INTJ",
                "moral_compass": ["Be helpful", "Do no harm"]
            },
            "linguistics": {
                "style": "concise",
                "formality": "casual"
            }
        }"#;
        std::fs::write(&identity_path, aieos_json).unwrap();

        // Create identity config pointing to the file
        let config = IdentityConfig {
            format: "aieos".into(),
            aieos_path: Some("aieos_identity.json".into()),
            aieos_inline: None,
        };

        let prompt = build_system_prompt(tmp.path(), "model", &[], &[], Some(&config), None);

        // Should contain AIEOS sections
        assert!(prompt.contains("## Identity"));
        assert!(prompt.contains("**Name:** Nova"));
        assert!(prompt.contains("**Nickname:** Nov"));
        assert!(prompt.contains("**Bio:** A helpful AI assistant."));
        assert!(prompt.contains("**Origin:** Silicon Valley"));

        assert!(prompt.contains("## Personality"));
        assert!(prompt.contains("**MBTI:** INTJ"));
        assert!(prompt.contains("**Moral Compass:**"));
        assert!(prompt.contains("- Be helpful"));

        assert!(prompt.contains("## Communication Style"));
        assert!(prompt.contains("**Style:** concise"));
        assert!(prompt.contains("**Formality Level:** casual"));

        // Should NOT contain OpenClaw bootstrap file headers
        assert!(!prompt.contains("### SOUL.md"));
        assert!(!prompt.contains("### IDENTITY.md"));
        assert!(!prompt.contains("[File not found"));
    }

    #[test]
    fn aieos_identity_from_inline() {
        use crate::config::IdentityConfig;

        let config = IdentityConfig {
            format: "aieos".into(),
            aieos_path: None,
            aieos_inline: Some(r#"{"identity":{"names":{"first":"Claw"}}}"#.into()),
        };

        let prompt = build_system_prompt(
            std::env::temp_dir().as_path(),
            "model",
            &[],
            &[],
            Some(&config),
            None,
        );

        assert!(prompt.contains("**Name:** Claw"));
        assert!(prompt.contains("## Identity"));
    }

    #[test]
    fn aieos_fallback_to_openclaw_on_parse_error() {
        use crate::config::IdentityConfig;

        let config = IdentityConfig {
            format: "aieos".into(),
            aieos_path: Some("nonexistent.json".into()),
            aieos_inline: None,
        };

        let ws = make_workspace();
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], Some(&config), None);

        // Should fall back to OpenClaw format when AIEOS file is not found
        // (Error is logged to stderr with filename, not included in prompt)
        assert!(prompt.contains("### SOUL.md"));
    }

    #[test]
    fn aieos_empty_uses_openclaw() {
        use crate::config::IdentityConfig;

        // Format is "aieos" but neither path nor inline is set
        let config = IdentityConfig {
            format: "aieos".into(),
            aieos_path: None,
            aieos_inline: None,
        };

        let ws = make_workspace();
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], Some(&config), None);

        // Should use OpenClaw format (not configured for AIEOS)
        assert!(prompt.contains("### SOUL.md"));
        assert!(prompt.contains("Be helpful"));
    }

    #[test]
    fn openclaw_format_uses_bootstrap_files() {
        use crate::config::IdentityConfig;

        let config = IdentityConfig {
            format: "openclaw".into(),
            aieos_path: Some("identity.json".into()),
            aieos_inline: None,
        };

        let ws = make_workspace();
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], Some(&config), None);

        // Should use OpenClaw format even if aieos_path is set
        assert!(prompt.contains("### SOUL.md"));
        assert!(prompt.contains("Be helpful"));
        assert!(!prompt.contains("## Identity"));
    }

    #[test]
    fn none_identity_config_uses_openclaw() {
        let ws = make_workspace();
        // Pass None for identity config
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

        // Should use OpenClaw format
        assert!(prompt.contains("### SOUL.md"));
        assert!(prompt.contains("Be helpful"));
    }

    #[test]
    fn classify_health_ok_true() {
        let state = classify_health_result(&Ok(true));
        assert_eq!(state, ChannelHealthState::Healthy);
    }

    #[test]
    fn classify_health_ok_false() {
        let state = classify_health_result(&Ok(false));
        assert_eq!(state, ChannelHealthState::Unhealthy);
    }

    #[tokio::test]
    async fn classify_health_timeout() {
        let result = tokio::time::timeout(Duration::from_millis(1), async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            true
        })
        .await;
        let state = classify_health_result(&result);
        assert_eq!(state, ChannelHealthState::Timeout);
    }

    struct AlwaysFailChannel {
        name: &'static str,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Channel for AlwaysFailChannel {
        fn name(&self) -> &str {
            self.name
        }

        async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
            Ok(())
        }

        async fn listen(
            &self,
            _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
        ) -> anyhow::Result<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            anyhow::bail!("listen boom")
        }
    }

    #[tokio::test]
    async fn supervised_listener_marks_error_and_restarts_on_failures() {
        let calls = Arc::new(AtomicUsize::new(0));
        let channel: Arc<dyn Channel> = Arc::new(AlwaysFailChannel {
            name: "test-supervised-fail",
            calls: Arc::clone(&calls),
        });

        let (tx, rx) = tokio::sync::mpsc::channel::<traits::ChannelMessage>(1);
        let handle = spawn_supervised_listener(channel, tx, 1, 1);

        tokio::time::sleep(Duration::from_millis(80)).await;
        drop(rx);
        handle.abort();
        let _ = handle.await;

        let snapshot = crate::health::snapshot_json();
        let component = &snapshot["components"]["channel:test-supervised-fail"];
        assert_eq!(component["status"], "error");
        assert!(component["restart_count"].as_u64().unwrap_or(0) >= 1);
        assert!(component["last_error"]
            .as_str()
            .unwrap_or("")
            .contains("listen boom"));
        assert!(calls.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn loop_event_mapping_keeps_session_prefix() {
        let mapped = map_loop_event_to_channel_content(
            "session-123",
            &crate::agent::unified_loop::LoopEvent::LLMProgress("thinking".to_string()),
        )
        .unwrap();

        assert!(mapped.starts_with("[session:session-123]"));
        assert!(mapped.contains("thinking"));
    }

    #[test]
    fn loop_event_mapping_surfaces_approval_request() {
        let mapped = map_loop_event_to_channel_content(
            "session-123",
            &crate::agent::unified_loop::LoopEvent::ApprovalRequired("shell".to_string()),
        )
        .unwrap();

        assert!(mapped.contains("Approval required"));
        assert!(mapped.contains("shell"));
    }

    #[tokio::test]
    async fn channel_tool_loop_returns_structured_denial_for_mcp_calls() {
        let provider = McpToolCallingProvider {
            calls: AtomicUsize::new(0),
        };
        let mut history = vec![
            ConversationMessage::Chat(ChatMessage::system("system")),
            ConversationMessage::Chat(ChatMessage::user("query")),
        ];

        let response = run_unified_channel_tool_loop(
            &provider,
            &[],
            &mut history,
            ChannelLoopParams {
                model: "test-model",
                temperature: 0.0,
                max_tool_iterations: 2,
                dispatcher_mode: "native",
                delta_tx: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(response, "done");

        let mut has_structured_denial = false;
        for message in &history {
            if let ConversationMessage::ToolResults(results) = message {
                for result in results {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result.content) {
                        if parsed["code"] == "approval_required"
                            && parsed["tool"] == "mcp.docs.search"
                        {
                            has_structured_denial = true;
                        }
                    }
                }
            }
        }

        assert!(has_structured_denial);
    }

    #[test]
    fn timeout_abort_text_includes_session_and_abort_semantics() {
        let text = channel_timeout_abort_text("chan-1");
        assert!(text.contains("[session:chan-1]"));
        assert!(text.contains("timed out"));
        assert!(text.contains("aborted"));
    }

    #[tokio::test]
    async fn process_channel_message_blocks_on_approval_by_default() {
        let _env_lock = CHANNEL_ENV_MUTEX.lock().await;
        std::env::remove_var("CORVUS_UNIFIED_APPROVE");

        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();

        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(Config::default()),
            channels_by_name: Arc::new(channels_by_name),
            provider: Arc::new(SlowProvider {
                delay: Duration::from_millis(1),
            }),
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test-system-prompt".to_string()),
            model: Arc::new("test-model".to_string()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "approval-1".to_string(),
                sender: "alice".to_string(),
                reply_target: "chat-1".to_string(),
                content: "needs-approval".to_string(),
                channel: "test-channel".to_string(),
                timestamp: 1,
            },
        )
        .await;

        let sent_messages = channel_impl.sent_messages.lock().await;
        assert_eq!(sent_messages.len(), 1);
        assert!(sent_messages[0].contains("request blocked"));
        assert!(sent_messages[0].contains("[session:test-channel-approval-1]"));
    }

    #[tokio::test]
    async fn process_channel_message_unblocks_on_approval_override() {
        let _env_lock = CHANNEL_ENV_MUTEX.lock().await;
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();

        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(Config::default()),
            channels_by_name: Arc::new(channels_by_name),
            provider: Arc::new(SlowProvider {
                delay: Duration::from_millis(1),
            }),
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test-system-prompt".to_string()),
            model: Arc::new("test-model".to_string()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        });

        // RAII guard to ensure env var is removed even if process_channel_message panics
        struct EnvVarGuard(&'static str);
        impl Drop for EnvVarGuard {
            fn drop(&mut self) {
                std::env::remove_var(self.0);
            }
        }
        std::env::set_var("CORVUS_UNIFIED_APPROVE", "1");
        let _guard = EnvVarGuard("CORVUS_UNIFIED_APPROVE");

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "approval-2".to_string(),
                sender: "alice".to_string(),
                reply_target: "chat-1".to_string(),
                content: "needs-approval".to_string(),
                channel: "test-channel".to_string(),
                timestamp: 1,
            },
        )
        .await;
        // Guard drops here and removes the env var

        let sent_messages = channel_impl.sent_messages.lock().await;
        assert_eq!(sent_messages.len(), 1);
        assert!(!sent_messages[0].contains("request blocked"));
    }

    #[test]
    fn update_visibility_gate_follows_policy_flags() {
        let mut config = Config::default();
        config.updates.enabled = true;
        config.updates.channel_visibility_enabled = true;
        assert!(update_visibility_enabled(&config));

        config.updates.channel_visibility_enabled = false;
        assert!(!update_visibility_enabled(&config));

        config.updates.enabled = false;
        config.updates.channel_visibility_enabled = true;
        assert!(!update_visibility_enabled(&config));
    }
}
