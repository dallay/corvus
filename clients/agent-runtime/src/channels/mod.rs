pub mod audio_media;
pub mod cli;
pub mod dingtalk;
pub mod discord;
pub mod email_channel;
pub mod imessage;
pub mod irc;
pub mod lark;
pub mod matrix;
pub mod mattermost;
pub mod media;
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
use crate::agent::prompt::{
    render_datetime_section, render_project_context_section, render_runtime_section,
    render_safety_section, render_skills_section, render_workspace_section,
    COMPACT_CONTEXT_BOOTSTRAP_MAX_CHARS,
};
use crate::bootstrap;
use crate::config::Config;
use crate::memory::Memory;
use crate::observability::Observer;
use crate::providers::{ChatMessage, ChatRequest, ConversationMessage, Provider};
use crate::tools::Tool;
use crate::transcription::traits::Transcriber;
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

const DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS: u64 = 2;
const DEFAULT_CHANNEL_MAX_BACKOFF_SECS: u64 = 60;
/// Timeout for processing a single channel message (LLM + tools).
/// 300s for on-device LLMs (Ollama) which are slower than cloud APIs.
const CHANNEL_MESSAGE_TIMEOUT_SECS: u64 = 300;
#[cfg(test)]
const CHANNEL_HEALTH_TICK_SECS: u64 = 1;
#[cfg(not(test))]
const CHANNEL_HEALTH_TICK_SECS: u64 = 60;
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
    transcriber: Option<Arc<dyn Transcriber>>,
}

/// Shared handle for enqueuing messages into the channel runtime
/// pipeline. `Clone + Send + Sync` so it can be shared between
/// channel listeners and the gateway module.
#[derive(Clone)]
pub struct ChannelRuntimeHandle {
    tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
}

impl ChannelRuntimeHandle {
    /// Create a new handle from an mpsc sender.
    pub fn new(tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>) -> Self {
        Self { tx }
    }

    /// Enqueue a canonical message into the processing pipeline.
    ///
    /// Returns an error if the receiver has been dropped or the
    /// channel buffer is full (backpressure).
    pub fn enqueue(&self, msg: traits::ChannelMessage) -> Result<()> {
        self.tx
            .try_send(msg)
            .map_err(|e| anyhow::anyhow!("failed to enqueue channel message: {e}"))
    }

    /// Obtain a sender clone for use with channel listeners.
    pub(crate) fn sender(&self) -> tokio::sync::mpsc::Sender<traits::ChannelMessage> {
        self.tx.clone()
    }
}

/// RAII guard ensuring staged image temp files are cleaned up on
/// all exit paths (success, error, timeout, early return).
struct StagedImageGuard(Vec<media::StagedImage>);

impl Drop for StagedImageGuard {
    fn drop(&mut self) {
        for img in &self.0 {
            img.cleanup();
        }
    }
}

/// RAII guard ensuring staged audio temp files are cleaned up on
/// all exit paths (success, error, timeout, early return).
struct StagedAudioGuard(Vec<audio_media::StagedAudio>);

impl Drop for StagedAudioGuard {
    fn drop(&mut self) {
        for audio in &self.0 {
            audio.cleanup();
        }
    }
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

#[derive(Debug)]
struct ResolvedImageRoute {
    selector: String,
    provider: String,
    model: String,
}

fn resolve_image_route(config: &Config) -> Result<ResolvedImageRoute, media::ImageRejectionReason> {
    let hint = config
        .multimodal
        .vision_model_hint
        .as_deref()
        .map(str::trim)
        .filter(|hint| !hint.is_empty())
        .ok_or(media::ImageRejectionReason::MissingVisionRoute)?;

    let route = config
        .model_routes
        .iter()
        .find(|route| route.hint == hint)
        .ok_or(media::ImageRejectionReason::MissingVisionRoute)?;

    if !route.allow_image_input {
        return Err(media::ImageRejectionReason::RouteNotImageCapable);
    }

    Ok(ResolvedImageRoute {
        selector: format!("hint:{hint}"),
        provider: route.provider.clone(),
        model: route.model.clone(),
    })
}

#[allow(clippy::too_many_arguments)]
fn rejection_to_ingress_reason(
    r: &media::ImageRejectionReason,
) -> crate::observability::ImageIngressReason {
    use crate::observability::ImageIngressReason;
    match r {
        media::ImageRejectionReason::Disabled => ImageIngressReason::Disabled,
        media::ImageRejectionReason::ChannelNotAllowed => ImageIngressReason::ChannelNotAllowed,
        media::ImageRejectionReason::MissingVisionRoute => ImageIngressReason::MissingVisionRoute,
        media::ImageRejectionReason::RouteNotImageCapable => {
            ImageIngressReason::RouteNotImageCapable
        }
        media::ImageRejectionReason::FetchFailed => ImageIngressReason::FetchFailed,
        media::ImageRejectionReason::MimeRejected => ImageIngressReason::MimeRejected,
        media::ImageRejectionReason::Oversize => ImageIngressReason::Oversize,
        media::ImageRejectionReason::TooManyImages => ImageIngressReason::TooManyImages,
        media::ImageRejectionReason::ProviderError => ImageIngressReason::ProviderError,
        media::ImageRejectionReason::ChannelNotSupported => ImageIngressReason::ChannelNotSupported,
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_image_ingress(
    observer: &dyn Observer,
    channel: &str,
    provider: Option<String>,
    model: Option<String>,
    outcome: crate::observability::ImageIngressOutcome,
    reason: Option<media::ImageRejectionReason>,
    image_count: usize,
    mime_type: Option<String>,
    byte_len: Option<u64>,
) {
    observer.on_image_ingress(&crate::observability::ImageIngressEvent {
        channel: channel.to_string(),
        provider,
        model,
        outcome,
        reason: reason.as_ref().map(rejection_to_ingress_reason),
        image_count,
        mime_type,
        byte_len,
    });
}

fn execution_model_for_turn<'a>(
    default_model: &'a str,
    image_route: Option<&'a ResolvedImageRoute>,
) -> &'a str {
    image_route
        .map(|route| route.selector.as_str())
        .unwrap_or(default_model)
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
            let mut health_interval =
                tokio::time::interval(Duration::from_secs(CHANNEL_HEALTH_TICK_SECS));
            let mut listen_task = Box::pin(ch.listen(tx.clone()));

            let result = loop {
                tokio::select! {
                    res = &mut listen_task => break res,
                    () = tx.closed() => break Ok(()),
                    _ = health_interval.tick() => {
                        crate::health::mark_component_ok(&component);
                    }
                }
            };

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
    images: &'a [media::StagedImage],
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
                    images: params.images,
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

async fn process_channel_message(ctx: Arc<ChannelRuntimeContext>, mut msg: traits::ChannelMessage) {
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

    let session_id = channel_session_id(&msg);
    let started_at = Instant::now();

    // ── Audio pipeline (before memory enrichment) ────────
    let audio_history_metas = if msg.has_audio_parts() {
        if gate_audio_config(&ctx, &msg, &session_id, target_channel.as_ref())
            .await
            .is_err()
        {
            return;
        }

        let audio_guard =
            match gate_and_stage_audio(&ctx, &msg, &session_id, target_channel.as_ref()).await {
                Ok(guard) => guard,
                Err(()) => return,
            };

        let transcriptions = match transcribe_audio(
            &ctx,
            &audio_guard.0,
            &session_id,
            target_channel.as_ref(),
            &msg,
        )
        .await
        {
            Ok(t) => t,
            Err(()) => return,
        };

        // Emit admitted event
        for (audio, tx) in audio_guard.0.iter().zip(transcriptions.iter()) {
            emit_audio_ingress(
                ctx.observer.as_ref(),
                &msg.channel,
                crate::observability::AudioIngressOutcome::Admitted,
                None,
                Some(audio.mime_type.as_str().to_string()),
                Some(audio.byte_len),
                audio.duration_secs,
                tx.processing_ms,
            );
        }

        // audio_guard drops at end of this block, cleaning up temp files
        inject_transcription(&mut msg, &audio_guard.0, &transcriptions)
    } else {
        Vec::new()
    };

    let user_text = extract_user_text(&msg);
    let enriched_message = enrich_with_memory(&ctx, &msg, &user_text).await;

    if update_visibility_enabled(ctx.config.as_ref()) {
        let _ = crate::update::maybe_send_opportunistic_update_notice(
            ctx.config.as_ref(),
            &msg,
            target_channel.as_ref(),
            env!("CARGO_PKG_VERSION"),
        )
        .await;
    }

    if handle_canonical_blocking_outcome(
        target_channel.as_ref(),
        &session_id,
        &msg.reply_target,
        &user_text,
    )
    .await
    .is_some()
    {
        return;
    }

    // ── Image pipeline gating ────────────────────────────
    let image_route_metadata =
        match gate_multimodal_config(&ctx, &msg, &session_id, target_channel.as_ref()).await {
            Ok(route) => route,
            Err(()) => return,
        };

    let staged_guard = match gate_and_stage_images(
        &ctx,
        &msg,
        &session_id,
        target_channel.as_ref(),
        image_route_metadata.as_ref(),
    )
    .await
    {
        Ok(guard) => guard,
        Err(()) => return,
    };

    emit_image_provider_outcome(
        &ctx,
        &msg,
        image_route_metadata.as_ref(),
        &staged_guard.0,
        crate::observability::ImageIngressOutcome::Admitted,
        None,
    );

    // ── Provider dispatch ────────────────────────────────
    println!("  ⏳ Processing message...");

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

    let effective_model =
        execution_model_for_turn(ctx.model.as_str(), image_route_metadata.as_ref()).to_string();

    let llm_result = tokio::time::timeout(
        Duration::from_secs(CHANNEL_MESSAGE_TIMEOUT_SECS),
        run_unified_channel_tool_loop(
            ctx.provider.as_ref(),
            ctx.tools_registry.as_ref(),
            &mut history,
            ChannelLoopParams {
                model: &effective_model,
                temperature: ctx.temperature,
                max_tool_iterations: ctx.max_tool_iterations,
                dispatcher_mode: &ctx.tool_dispatcher_mode,
                delta_tx,
                images: staged_guard.0.as_slice(),
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
            emit_image_provider_outcome(
                &ctx,
                &msg,
                image_route_metadata.as_ref(),
                &staged_guard.0,
                crate::observability::ImageIngressOutcome::ProviderSent,
                None,
            );
            handle_successful_response(
                ctx.as_ref(),
                &history_key,
                &enriched_message,
                &effective_model,
                response,
                started_at,
                response_ctx,
                &staged_guard.0,
                &msg,
                audio_history_metas,
            )
            .await;
        }
        Ok(Err(e)) => {
            emit_image_provider_outcome(
                &ctx,
                &msg,
                image_route_metadata.as_ref(),
                &staged_guard.0,
                crate::observability::ImageIngressOutcome::ProviderError,
                Some(media::ImageRejectionReason::ProviderError),
            );
            handle_llm_error(e, started_at, response_ctx).await;
        }
        Err(_) => {
            emit_image_provider_outcome(
                &ctx,
                &msg,
                image_route_metadata.as_ref(),
                &staged_guard.0,
                crate::observability::ImageIngressOutcome::ProviderError,
                Some(media::ImageRejectionReason::ProviderError),
            );
            handle_timeout(session_id, started_at, response_ctx).await;
        }
    }
}

/// Extract the user-visible text from a channel message, preferring canonical
/// content before recomputing projection from multimodal parts.
fn extract_user_text(msg: &traits::ChannelMessage) -> String {
    if !msg.content.is_empty() {
        return msg.content.clone();
    }

    if msg.parts.is_empty() {
        return String::new();
    }

    msg.text_projection()
}

/// Build memory context, auto-save conversation, and return the enriched message.
async fn enrich_with_memory(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    user_text: &str,
) -> String {
    let memory_context = if user_text.trim().is_empty() {
        String::new()
    } else {
        build_memory_context(ctx.memory.as_ref(), user_text, ctx.min_relevance_score).await
    };

    if ctx.auto_save_memory && !user_text.trim().is_empty() {
        let autosave_key = conversation_memory_key(msg);
        let _ = ctx
            .memory
            .store(
                &autosave_key,
                user_text,
                crate::memory::MemoryCategory::Conversation,
                None,
            )
            .await;
    }

    if memory_context.is_empty() {
        user_text.to_string()
    } else {
        format!("{memory_context}{user_text}")
    }
}

/// Send an image rejection: emit observability event and notify user.
async fn reject_image_turn(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
    route: Option<&ResolvedImageRoute>,
    reason: media::ImageRejectionReason,
    image_count: usize,
    rejection_text: String,
) {
    emit_image_ingress(
        ctx.observer.as_ref(),
        &msg.channel,
        route.map(|r| r.provider.clone()),
        route.map(|r| r.model.clone()),
        crate::observability::ImageIngressOutcome::Rejected,
        Some(reason),
        image_count,
        None,
        None,
    );
    if let Some(ch) = target_channel {
        let _ = ch
            .send(&SendMessage::new(rejection_text, &msg.reply_target))
            .await;
    }
}

/// Gate multimodal configuration: check enabled, allowed channels, vision route.
/// Returns `Ok(Some(route))` if images are allowed, `Ok(None)` if no images,
/// `Err(())` if rejected (response already sent to channel).
async fn gate_multimodal_config(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    session_id: &str,
    target_channel: Option<&Arc<dyn Channel>>,
) -> Result<Option<ResolvedImageRoute>, ()> {
    if !msg.has_image_parts() {
        return Ok(None);
    }

    let mm = &ctx.config.multimodal;
    let img_count = msg.image_parts().len();

    if !mm.enabled {
        reject_image_turn(
            ctx,
            msg,
            target_channel,
            None,
            media::ImageRejectionReason::Disabled,
            img_count,
            format!("[session:{session_id}] ⚠️ Image input is currently disabled."),
        )
        .await;
        return Err(());
    }

    if !mm.allowed_channels.contains(&msg.channel) {
        reject_image_turn(
            ctx,
            msg,
            target_channel,
            None,
            media::ImageRejectionReason::ChannelNotAllowed,
            img_count,
            format!("[session:{session_id}] ⚠️ Image input is not enabled for this channel."),
        )
        .await;
        return Err(());
    }

    match resolve_image_route(ctx.config.as_ref()) {
        Ok(route) => Ok(Some(route)),
        Err(reason) => {
            let text = image_route_rejection_text(session_id, &reason);
            reject_image_turn(ctx, msg, target_channel, None, reason, img_count, text).await;
            Err(())
        }
    }
}

/// Format rejection text for image route resolution failures.
fn image_route_rejection_text(session_id: &str, reason: &media::ImageRejectionReason) -> String {
    match reason {
        media::ImageRejectionReason::MissingVisionRoute => {
            format!("[session:{session_id}] ⚠️ Image input is not configured with a vision route.")
        }
        media::ImageRejectionReason::RouteNotImageCapable => format!(
            "[session:{session_id}] ⚠️ The configured vision route does not allow image input."
        ),
        _ => format!("[session:{session_id}] ⚠️ Image input is not available for this request."),
    }
}

/// Gate image count, stage images, and verify staging succeeded.
/// Returns `Ok(guard)` on success, `Err(())` if rejected (response already sent).
async fn gate_and_stage_images(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    session_id: &str,
    target_channel: Option<&Arc<dyn Channel>>,
    route: Option<&ResolvedImageRoute>,
) -> Result<StagedImageGuard, ()> {
    if !msg.has_image_parts() {
        return Ok(StagedImageGuard(Vec::new()));
    }

    let image_count = msg.image_parts().len();

    if media::validate_image_count(image_count).is_err() {
        reject_image_turn(
            ctx,
            msg,
            target_channel,
            route,
            media::ImageRejectionReason::TooManyImages,
            image_count,
            format!(
                "[session:{session_id}] ⚠️ Too many images \
                 ({image_count}). Maximum {} per message.",
                media::MAX_IMAGES_PER_TURN,
            ),
        )
        .await;
        return Err(());
    }

    let staged = match stage_channel_images(ctx.config.as_ref(), msg).await {
        Ok(s) => s,
        Err(reason) => {
            let text = staging_rejection_text(session_id, &reason);
            reject_image_turn(ctx, msg, target_channel, route, reason, image_count, text).await;
            return Err(());
        }
    };
    let guard = StagedImageGuard(staged);

    // Fail-closed: reject image turns on channels that have
    // not yet implemented fetch/staging.
    if guard.0.is_empty() {
        reject_image_turn(
            ctx,
            msg,
            target_channel,
            route,
            media::ImageRejectionReason::ChannelNotSupported,
            image_count,
            format!(
                "[session:{session_id}] ⚠️ Image input is not \
                 yet supported for this channel."
            ),
        )
        .await;
        return Err(());
    }

    Ok(guard)
}

/// Format rejection text for image staging failures.
fn staging_rejection_text(session_id: &str, reason: &media::ImageRejectionReason) -> String {
    match reason {
        media::ImageRejectionReason::FetchFailed => format!(
            "[session:{session_id}] ⚠️ I couldn't download that image safely. Please try again."
        ),
        media::ImageRejectionReason::MimeRejected => {
            format!("[session:{session_id}] ⚠️ That image format is not supported.")
        }
        media::ImageRejectionReason::Oversize => {
            format!("[session:{session_id}] ⚠️ That image is too large to process.")
        }
        _ => format!("[session:{session_id}] ⚠️ Image input is not available for this request."),
    }
}

/// Convert an `Instant` elapsed time to milliseconds as `u64`.
fn elapsed_ms(start: &std::time::Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Convert an `f64` duration in seconds to milliseconds as `u64`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn duration_f64_to_ms(secs: f64) -> u64 {
    (secs * 1000.0).clamp(0.0, u64::MAX as f64) as u64
}

/// Build a transcriber from config when audio is enabled.
/// Build a transcriber from config. Returns None if audio is disabled.
/// This is pub(crate) so the gateway can reuse it.
pub(crate) fn build_transcriber(config: &Config) -> Option<Arc<dyn Transcriber>> {
    if !config.audio.enabled {
        return None;
    }
    let ac = &config.audio;
    Some(Arc::new(
        crate::transcription::whisper_cli::WhisperCliTranscriber::new(
            ac.whisper_binary.clone(),
            &ac.transcription_model,
            ac.transcription_language.clone(),
            ac.transcription_timeout_secs,
            ac.max_concurrent_transcriptions,
        ),
    ))
}

// ── Audio pipeline helpers ──────────────────────────────────────

fn audio_rejection_to_ingress_reason(
    r: &audio_media::AudioRejectionReason,
) -> crate::observability::AudioIngressReason {
    use crate::observability::AudioIngressReason;
    match r {
        audio_media::AudioRejectionReason::Disabled => AudioIngressReason::Disabled,
        audio_media::AudioRejectionReason::ChannelNotAllowed => {
            AudioIngressReason::ChannelNotAllowed
        }
        audio_media::AudioRejectionReason::FetchFailed => AudioIngressReason::FetchFailed,
        audio_media::AudioRejectionReason::MimeRejected => AudioIngressReason::MimeRejected,
        audio_media::AudioRejectionReason::Oversize => AudioIngressReason::Oversize,
        audio_media::AudioRejectionReason::TooLong => AudioIngressReason::TooLong,
        audio_media::AudioRejectionReason::Corrupted => AudioIngressReason::Corrupted,
        audio_media::AudioRejectionReason::TranscriptionFailed => {
            AudioIngressReason::TranscriptionFailed
        }
        audio_media::AudioRejectionReason::NoSpeechDetected => AudioIngressReason::NoSpeechDetected,
        audio_media::AudioRejectionReason::TranscriberUnavailable => {
            AudioIngressReason::TranscriberUnavailable
        }
        audio_media::AudioRejectionReason::MultipleAudioParts => {
            AudioIngressReason::MultipleAudioParts
        }
        audio_media::AudioRejectionReason::SystemError => AudioIngressReason::SystemError,
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_audio_ingress(
    observer: &dyn Observer,
    channel: &str,
    outcome: crate::observability::AudioIngressOutcome,
    reason: Option<&audio_media::AudioRejectionReason>,
    mime_type: Option<String>,
    byte_len: Option<u64>,
    duration_secs: Option<f64>,
    transcription_duration_ms: Option<u64>,
) {
    observer.on_audio_ingress(&crate::observability::AudioIngressEvent {
        channel: channel.to_string(),
        outcome,
        reason: reason.map(audio_rejection_to_ingress_reason),
        mime_type,
        byte_len,
        duration_secs,
        transcription_duration_ms,
    });
}

/// Map an `AudioRejectionReason` to a user-facing error message.
fn audio_rejection_user_text(
    session_id: &str,
    reason: &audio_media::AudioRejectionReason,
    config: &Config,
) -> String {
    let body = match reason {
        audio_media::AudioRejectionReason::Disabled => {
            "Audio input is currently disabled.".to_string()
        }
        audio_media::AudioRejectionReason::ChannelNotAllowed => {
            "Audio input is not enabled for this channel.".to_string()
        }
        audio_media::AudioRejectionReason::FetchFailed => {
            "I couldn't download that audio safely. Please try again.".to_string()
        }
        audio_media::AudioRejectionReason::MimeRejected => {
            "That audio format is not supported. Supported formats: OGG, MP3, WAV, M4A.".to_string()
        }
        audio_media::AudioRejectionReason::Oversize => {
            let max_mb = config.audio.max_audio_bytes / (1024 * 1024);
            format!("That audio file is too large to process. Maximum size: {max_mb} MB.")
        }
        audio_media::AudioRejectionReason::TooLong => {
            let secs = config.audio.max_audio_duration_secs;
            if secs >= 60 && secs.is_multiple_of(60) {
                let mins = secs / 60;
                format!(
                    "That audio is too long to process. Maximum duration: {mins} minute{}.",
                    if mins == 1 { "" } else { "s" }
                )
            } else if secs >= 60 {
                let mins = secs / 60;
                let rem = secs % 60;
                format!("That audio is too long to process. Maximum duration: {mins} minute{} {rem} second{}.", if mins == 1 { "" } else { "s" }, if rem == 1 { "" } else { "s" })
            } else {
                format!(
                    "That audio is too long to process. Maximum duration: {secs} second{}.",
                    if secs == 1 { "" } else { "s" }
                )
            }
        }
        audio_media::AudioRejectionReason::Corrupted => {
            "That audio file appears to be corrupted and cannot be processed.".to_string()
        }
        audio_media::AudioRejectionReason::TranscriberUnavailable => {
            "Audio transcription is not available on this agent. \
             Please send text instead."
                .to_string()
        }
        audio_media::AudioRejectionReason::TranscriptionFailed => {
            "Audio transcription failed. Please try again or send text instead.".to_string()
        }
        audio_media::AudioRejectionReason::NoSpeechDetected => {
            "No speech was detected in that audio. \
             Please try again with a clearer recording."
                .to_string()
        }
        audio_media::AudioRejectionReason::MultipleAudioParts => {
            "Only one audio file per message is supported.".to_string()
        }
        audio_media::AudioRejectionReason::SystemError => {
            "An internal error occurred processing your audio. Please try again.".to_string()
        }
    };
    format!("[session:{session_id}] ⚠️ {body}")
}

/// Send an audio rejection: emit observability event and notify user.
async fn reject_audio_turn(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
    reason: audio_media::AudioRejectionReason,
    session_id: &str,
) {
    emit_audio_ingress(
        ctx.observer.as_ref(),
        &msg.channel,
        crate::observability::AudioIngressOutcome::Rejected,
        Some(&reason),
        None,
        None,
        None,
        None,
    );
    let text = audio_rejection_user_text(session_id, &reason, ctx.config.as_ref());
    if let Some(ch) = target_channel {
        let _ = ch.send(&SendMessage::new(text, &msg.reply_target)).await;
    }
}

/// Gate audio configuration: check enabled and allowed channels.
/// Returns `Ok(())` if audio should be processed, `Err(())` if rejected.
async fn gate_audio_config(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    session_id: &str,
    target_channel: Option<&Arc<dyn Channel>>,
) -> Result<(), ()> {
    if !msg.has_audio_parts() {
        return Ok(());
    }

    let audio_cfg = &ctx.config.audio;
    if !audio_cfg.enabled {
        reject_audio_turn(
            ctx,
            msg,
            target_channel,
            audio_media::AudioRejectionReason::Disabled,
            session_id,
        )
        .await;
        return Err(());
    }

    if !audio_cfg.allowed_channels.contains(&msg.channel) {
        reject_audio_turn(
            ctx,
            msg,
            target_channel,
            audio_media::AudioRejectionReason::ChannelNotAllowed,
            session_id,
        )
        .await;
        return Err(());
    }

    // Check transcriber availability
    if ctx.transcriber.is_none() {
        reject_audio_turn(
            ctx,
            msg,
            target_channel,
            audio_media::AudioRejectionReason::TranscriberUnavailable,
            session_id,
        )
        .await;
        return Err(());
    }

    Ok(())
}

/// Fetch, validate, and stage audio from channel. Returns staged audio
/// wrapped in RAII guard, or `Err(())` if rejected (response sent).
async fn gate_and_stage_audio(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    session_id: &str,
    target_channel: Option<&Arc<dyn Channel>>,
) -> Result<StagedAudioGuard, ()> {
    if !msg.has_audio_parts() {
        return Ok(StagedAudioGuard(Vec::new()));
    }

    // Spec: max 1 audio per message
    let audio_parts = msg.audio_parts();
    if audio_parts.len() > 1 {
        reject_audio_turn(
            ctx,
            msg,
            target_channel,
            audio_media::AudioRejectionReason::MultipleAudioParts,
            session_id,
        )
        .await;
        return Err(());
    }

    let staged = match stage_channel_audio(ctx.config.as_ref(), msg).await {
        Ok(s) => s,
        Err(reason) => {
            reject_audio_turn(ctx, msg, target_channel, reason, session_id).await;
            return Err(());
        }
    };

    if staged.is_empty() {
        reject_audio_turn(
            ctx,
            msg,
            target_channel,
            audio_media::AudioRejectionReason::FetchFailed,
            session_id,
        )
        .await;
        return Err(());
    }

    Ok(StagedAudioGuard(staged))
}

/// Dispatch audio staging to the appropriate channel implementation.
async fn stage_channel_audio(
    config: &Config,
    msg: &traits::ChannelMessage,
) -> Result<Vec<audio_media::StagedAudio>, audio_media::AudioRejectionReason> {
    let max_bytes = config.audio.max_audio_bytes;
    let max_duration_secs = config.audio.max_audio_duration_secs;
    let mut staged = Vec::with_capacity(msg.audio_parts().len());

    for part in msg.audio_parts() {
        let traits::ContentPart::Audio {
            channel_handle,
            declared_mime,
            declared_duration_secs,
            declared_bytes,
            ..
        } = part
        else {
            continue;
        };

        let audio = match msg.channel.as_str() {
            "telegram" => {
                build_telegram_channel(config)
                    .ok_or(audio_media::AudioRejectionReason::FetchFailed)?
                    .fetch_and_stage_audio(
                        channel_handle,
                        declared_mime.as_deref(),
                        *declared_duration_secs,
                        *declared_bytes,
                        max_bytes,
                        max_duration_secs,
                    )
                    .await?
            }
            // "gateway" and "cli" handle audio entirely pre-pipeline (Option A
            // architecture): they transcribe audio before injecting a plain-text
            // ChannelMessage into the agent flow. Raw audio is never routed through
            // this function for those channels.
            _ => {
                return Err(audio_media::AudioRejectionReason::ChannelNotAllowed);
            }
        };

        staged.push(audio);
    }

    Ok(staged)
}

/// Transcribe staged audio files. Returns transcription results or
/// `Err(())` if transcription failed (response sent to channel).
async fn transcribe_audio(
    ctx: &ChannelRuntimeContext,
    staged: &[audio_media::StagedAudio],
    session_id: &str,
    target_channel: Option<&Arc<dyn Channel>>,
    msg: &traits::ChannelMessage,
) -> Result<Vec<crate::transcription::traits::TranscriptionResult>, ()> {
    let transcriber = match ctx.transcriber.as_ref() {
        Some(t) => t,
        None => {
            reject_audio_turn(
                ctx,
                msg,
                target_channel,
                audio_media::AudioRejectionReason::TranscriberUnavailable,
                session_id,
            )
            .await;
            return Err(());
        }
    };

    let mut results = Vec::with_capacity(staged.len());
    for audio in staged {
        let start = std::time::Instant::now();
        match transcriber.transcribe(audio).await {
            Ok(mut result) => {
                let processing_ms = elapsed_ms(&start);
                result.processing_ms = Some(processing_ms);
                // Empty transcription guard (REQ-14)
                if result.text.trim().is_empty() {
                    emit_audio_ingress(
                        ctx.observer.as_ref(),
                        &msg.channel,
                        crate::observability::AudioIngressOutcome::Rejected,
                        Some(&audio_media::AudioRejectionReason::NoSpeechDetected),
                        Some(audio.mime_type.as_str().to_string()),
                        Some(audio.byte_len),
                        audio.duration_secs,
                        Some(processing_ms),
                    );
                    let text = audio_rejection_user_text(
                        session_id,
                        &audio_media::AudioRejectionReason::NoSpeechDetected,
                        ctx.config.as_ref(),
                    );
                    if let Some(ch) = target_channel {
                        let _ = ch.send(&SendMessage::new(text, &msg.reply_target)).await;
                    }
                    return Err(());
                }
                results.push(result);
            }
            Err(reason) => {
                let processing_ms = elapsed_ms(&start);
                emit_audio_ingress(
                    ctx.observer.as_ref(),
                    &msg.channel,
                    crate::observability::AudioIngressOutcome::Rejected,
                    Some(&reason),
                    Some(audio.mime_type.as_str().to_string()),
                    Some(audio.byte_len),
                    audio.duration_secs,
                    Some(processing_ms),
                );
                let text = audio_rejection_user_text(session_id, &reason, ctx.config.as_ref());
                if let Some(ch) = target_channel {
                    let _ = ch.send(&SendMessage::new(text, &msg.reply_target)).await;
                }
                return Err(());
            }
        }
    }

    Ok(results)
}

/// Replace `ContentPart::Audio` with `ContentPart::Text` containing
/// the transcription. Build `AudioHistoryMeta` for conversation history.
fn inject_transcription(
    msg: &mut traits::ChannelMessage,
    staged: &[audio_media::StagedAudio],
    transcriptions: &[crate::transcription::traits::TranscriptionResult],
) -> Vec<audio_media::AudioHistoryMeta> {
    let mut history_metas = Vec::with_capacity(staged.len());
    let mut tx_idx = 0;

    msg.parts = msg
        .parts
        .iter()
        .map(|part| {
            if let traits::ContentPart::Audio { caption_text, .. } = part {
                if tx_idx < transcriptions.len() && tx_idx < staged.len() {
                    let transcription = &transcriptions[tx_idx];
                    let audio = &staged[tx_idx];
                    let trimmed = transcription.text.trim().to_string();

                    let meta = audio_media::AudioHistoryMeta::from_staged(
                        audio,
                        &trimmed,
                        caption_text.as_deref(),
                    );
                    history_metas.push(meta);

                    let injected_text = if caption_text.is_some() {
                        format!("[Audio transcription]: {trimmed}")
                    } else {
                        format!("[Voice message transcription]: {trimmed}")
                    };

                    tx_idx += 1;
                    traits::ContentPart::Text {
                        text: injected_text,
                    }
                } else {
                    part.clone()
                }
            } else {
                part.clone()
            }
        })
        .collect();

    // Update the legacy content field with the transcription
    msg.content = msg.text_projection();

    history_metas
}

/// Emit image ingress event for provider-level outcomes (admitted, sent, error).
fn emit_image_provider_outcome(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    route: Option<&ResolvedImageRoute>,
    staged: &[media::StagedImage],
    outcome: crate::observability::ImageIngressOutcome,
    reason: Option<media::ImageRejectionReason>,
) {
    let Some(route) = route else { return };
    let (count, mime, bytes) = if reason.is_none() {
        match staged.first() {
            Some(first) => (
                staged.len(),
                Some(first.mime_type.as_str().to_string()),
                Some(first.byte_len),
            ),
            None => return,
        }
    } else {
        (msg.image_parts().len(), None, None)
    };
    emit_image_ingress(
        ctx.observer.as_ref(),
        &msg.channel,
        Some(route.provider.clone()),
        Some(route.model.clone()),
        outcome,
        reason,
        count,
        mime,
        bytes,
    );
}

async fn stage_channel_images(
    config: &Config,
    msg: &traits::ChannelMessage,
) -> Result<Vec<media::StagedImage>, media::ImageRejectionReason> {
    let max_bytes = config
        .multimodal
        .max_image_bytes
        .unwrap_or(media::MAX_IMAGE_BYTES);
    let mut staged = Vec::with_capacity(msg.image_parts().len());

    for part in msg.image_parts() {
        let traits::ContentPart::Image {
            channel_handle,
            declared_mime,
            ..
        } = part
        else {
            continue;
        };

        let image = match msg.channel.as_str() {
            "telegram" => {
                build_telegram_channel(config)
                    .ok_or(media::ImageRejectionReason::FetchFailed)?
                    .fetch_and_stage_image(channel_handle, declared_mime.as_deref(), max_bytes)
                    .await?
            }
            "whatsapp" => {
                build_whatsapp_channel(config)
                    .ok_or(media::ImageRejectionReason::FetchFailed)?
                    .fetch_and_stage_image(channel_handle, declared_mime.as_deref(), max_bytes)
                    .await?
            }
            "discord" => {
                build_discord_channel(config)
                    .ok_or(media::ImageRejectionReason::FetchFailed)?
                    .fetch_and_stage_image(channel_handle, declared_mime.as_deref(), max_bytes)
                    .await?
            }
            _ => return Ok(Vec::new()),
        };

        staged.push(image);
    }

    Ok(staged)
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

    // Inject image/audio context from prior turns into outbound messages
    // without modifying stored history.
    for turn in prior_turns {
        let has_media_meta = turn.image_metadata.is_some() || turn.audio_metadata.is_some();
        if has_media_meta {
            let mut augmented_content = String::new();
            if let Some(ref meta_list) = turn.image_metadata {
                for meta in meta_list {
                    augmented_content.push_str(&meta.to_context_string());
                    augmented_content.push('\n');
                }
            }
            if let Some(ref meta_list) = turn.audio_metadata {
                for meta in meta_list {
                    augmented_content.push_str(&meta.to_context_string());
                    augmented_content.push('\n');
                }
            }
            augmented_content.push_str(&turn.content);
            history.push(ConversationMessage::Chat(ChatMessage::user(
                augmented_content,
            )));
        } else {
            history.push(ConversationMessage::Chat(turn));
        }
    }

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
    let canonical = crate::pre_execution::evaluate(session_id.to_string(), content).await;

    if let Some(blocking) = crate::pre_execution::classify_blocking(&canonical) {
        if let Some(ch) = channel {
            let text = match blocking {
                crate::pre_execution::BlockingOutcome::ApprovalRequired { tool } => {
                    format!(
                        "[session:{session_id}] approval required for `{tool}`; request blocked"
                    )
                }
                crate::pre_execution::BlockingOutcome::TimeoutAborted => {
                    channel_timeout_abort_text(session_id)
                }
                crate::pre_execution::BlockingOutcome::Fallback { response } => {
                    format!("[session:{session_id}] {response}")
                }
            };
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

#[allow(clippy::too_many_arguments)]
async fn handle_successful_response(
    ctx: &ChannelRuntimeContext,
    history_key: &str,
    enriched_message: &str,
    effective_model: &str,
    mut response: String,
    started_at: Instant,
    response_ctx: ResponseContext<'_>,
    staged_images: &[media::StagedImage],
    original_msg: &traits::ChannelMessage,
    audio_history_metas: Vec<audio_media::AudioHistoryMeta>,
) {
    response = enforce_strict_memory_validation(
        ctx.memory.as_ref(),
        ctx.provider.as_ref(),
        effective_model,
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

        // Build history turn with image/audio metadata if present
        if !audio_history_metas.is_empty() {
            if staged_images.is_empty() {
                turns.push(ChatMessage::user_with_audio(
                    enriched_message,
                    audio_history_metas,
                ));
            } else {
                // Mixed media: both audio and images in the same turn
                let caption = original_msg.parts.iter().find_map(|p| match p {
                    traits::ContentPart::Image { caption_text, .. } => caption_text.clone(),
                    _ => None,
                });
                let img_meta: Vec<media::ImageHistoryMeta> = staged_images
                    .iter()
                    .map(|img| media::ImageHistoryMeta::from_staged(img, caption.clone()))
                    .collect();
                turns.push(ChatMessage::user_with_media(
                    enriched_message,
                    img_meta,
                    audio_history_metas,
                ));
            }
        } else if !staged_images.is_empty() {
            let caption = original_msg.parts.iter().find_map(|p| match p {
                traits::ContentPart::Image { caption_text, .. }
                | traits::ContentPart::Audio { caption_text, .. } => caption_text.clone(),
                traits::ContentPart::Text { .. } => None,
            });
            let meta: Vec<media::ImageHistoryMeta> = staged_images
                .iter()
                .map(|img| media::ImageHistoryMeta::from_staged(img, caption.clone()))
                .collect();
            turns.push(ChatMessage::user_with_images(enriched_message, meta));
        } else {
            turns.push(ChatMessage::user(enriched_message));
        }

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
            process_channel_message(worker_ctx, msg).await;
            drop(permit); // Ensure semaphore permit lives until task completes.
        });

        while let Some(result) = workers.try_join_next() {
            log_worker_join_result(result);
        }
    }

    while let Some(result) = workers.join_next().await {
        log_worker_join_result(result);
    }
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
    prompt.push_str(&render_safety_section());
    prompt.push_str("\n\n");

    // ── 3. Skills (compact list — load on-demand) ───────────────
    let skills_section = render_skills_section(workspace_dir, skills);
    if !skills_section.is_empty() {
        prompt.push_str(&skills_section);
        prompt.push_str("\n\n");
    }

    // ── 4. Workspace ────────────────────────────────────────────
    prompt.push_str(&render_workspace_section(workspace_dir));
    prompt.push_str("\n\n");

    // ── 5. Bootstrap files (injected into context) ──────────────
    prompt.push_str(&render_project_context_section(
        workspace_dir,
        identity_config,
        bootstrap_max_chars,
    ));
    prompt.push_str("\n\n");

    // ── 6. Date & Time ──────────────────────────────────────────
    prompt.push_str(&render_datetime_section());
    prompt.push_str("\n\n");

    // ── 7. Runtime ──────────────────────────────────────────────
    prompt.push_str(&render_runtime_section(model_name));
    prompt.push_str("\n\n");

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

type ConfiguredChannelEntry = (&'static str, &'static str, Arc<dyn Channel>);
type ChannelBuilder = fn(&Config) -> Option<Arc<dyn Channel>>;

struct ChannelRegistryEntry {
    key: &'static str,
    display_name: &'static str,
    build: ChannelBuilder,
}

pub(crate) fn build_telegram_channel(config: &Config) -> Option<Arc<TelegramChannel>> {
    config.channels_config.telegram.as_ref().map(|tg| {
        Arc::new(
            TelegramChannel::new(tg.bot_token.clone(), tg.allowed_users.clone())
                .with_streaming(tg.stream_mode, tg.draft_update_interval_ms),
        )
    })
}

fn build_discord_channel(config: &Config) -> Option<Arc<DiscordChannel>> {
    config.channels_config.discord.as_ref().map(|dc| {
        Arc::new(DiscordChannel::new(
            dc.bot_token.clone(),
            dc.guild_id.clone(),
            dc.allowed_users.clone(),
            dc.listen_to_bots,
            dc.mention_only,
        ))
    })
}

fn build_slack_channel(config: &Config) -> Option<Arc<SlackChannel>> {
    config.channels_config.slack.as_ref().map(|sl| {
        Arc::new(SlackChannel::new(
            sl.bot_token.clone(),
            sl.channel_id.clone(),
            sl.allowed_users.clone(),
        ))
    })
}

fn build_mattermost_channel(config: &Config) -> Option<Arc<MattermostChannel>> {
    config.channels_config.mattermost.as_ref().map(|mm| {
        Arc::new(MattermostChannel::new(
            mm.url.clone(),
            mm.bot_token.clone(),
            mm.channel_id.clone(),
            mm.allowed_users.clone(),
            mm.thread_replies.unwrap_or(true),
        ))
    })
}

fn build_imessage_channel(config: &Config) -> Option<Arc<IMessageChannel>> {
    config
        .channels_config
        .imessage
        .as_ref()
        .map(|im| Arc::new(IMessageChannel::new(im.allowed_contacts.clone())))
}

fn build_matrix_channel(config: &Config) -> Option<Arc<MatrixChannel>> {
    config.channels_config.matrix.as_ref().map(|mx| {
        Arc::new(MatrixChannel::new(
            mx.homeserver.clone(),
            mx.access_token.clone(),
            mx.room_id.clone(),
            mx.allowed_users.clone(),
        ))
    })
}

fn build_signal_channel(config: &Config) -> Option<Arc<SignalChannel>> {
    config.channels_config.signal.as_ref().map(|sig| {
        Arc::new(SignalChannel::new(
            sig.http_url.clone(),
            sig.account.clone(),
            sig.group_id.clone(),
            sig.allowed_from.clone(),
            sig.ignore_attachments,
            sig.ignore_stories,
        ))
    })
}

pub(crate) fn build_whatsapp_channel(config: &Config) -> Option<Arc<WhatsAppChannel>> {
    config.channels_config.whatsapp.as_ref().map(|wa| {
        Arc::new(WhatsAppChannel::new(
            wa.access_token.clone(),
            wa.phone_number_id.clone(),
            wa.verify_token.clone(),
            wa.allowed_numbers.clone(),
        ))
    })
}

fn build_email_channel(config: &Config) -> Option<Arc<EmailChannel>> {
    config
        .channels_config
        .email
        .as_ref()
        .map(|cfg| Arc::new(EmailChannel::new(cfg.clone())))
}

fn build_irc_channel(config: &Config) -> Option<Arc<IrcChannel>> {
    config.channels_config.irc.as_ref().map(|cfg| {
        Arc::new(IrcChannel::new(irc::IrcChannelConfig {
            server: cfg.server.clone(),
            port: cfg.port,
            nickname: cfg.nickname.clone(),
            username: cfg.username.clone(),
            channels: cfg.channels.clone(),
            allowed_users: cfg.allowed_users.clone(),
            server_password: cfg.server_password.clone(),
            nickserv_password: cfg.nickserv_password.clone(),
            sasl_password: cfg.sasl_password.clone(),
            verify_tls: cfg.verify_tls.unwrap_or(true),
        }))
    })
}

fn build_lark_channel(config: &Config) -> Option<Arc<LarkChannel>> {
    config
        .channels_config
        .lark
        .as_ref()
        .map(|cfg| Arc::new(LarkChannel::from_config(cfg)))
}

fn build_dingtalk_channel(config: &Config) -> Option<Arc<DingTalkChannel>> {
    config.channels_config.dingtalk.as_ref().map(|dt| {
        Arc::new(DingTalkChannel::new(
            dt.client_id.clone(),
            dt.client_secret.clone(),
            dt.allowed_users.clone(),
        ))
    })
}

fn build_qq_channel(config: &Config) -> Option<Arc<QQChannel>> {
    config.channels_config.qq.as_ref().map(|qq| {
        Arc::new(QQChannel::new(
            qq.app_id.clone(),
            qq.app_secret.clone(),
            qq.allowed_users.clone(),
        ))
    })
}

fn build_telegram_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_telegram_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

fn build_discord_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_discord_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

fn build_slack_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_slack_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

fn build_mattermost_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_mattermost_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

fn build_imessage_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_imessage_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

fn build_matrix_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_matrix_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

fn build_signal_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_signal_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

fn build_whatsapp_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_whatsapp_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

fn build_email_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_email_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

fn build_irc_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_irc_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

fn build_lark_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_lark_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

fn build_dingtalk_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_dingtalk_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

fn build_qq_channel_dyn(config: &Config) -> Option<Arc<dyn Channel>> {
    build_qq_channel(config).map(|channel| channel as Arc<dyn Channel>)
}

const CHANNEL_REGISTRY: &[ChannelRegistryEntry] = &[
    ChannelRegistryEntry {
        key: "telegram",
        display_name: "Telegram",
        build: build_telegram_channel_dyn,
    },
    ChannelRegistryEntry {
        key: "discord",
        display_name: "Discord",
        build: build_discord_channel_dyn,
    },
    ChannelRegistryEntry {
        key: "slack",
        display_name: "Slack",
        build: build_slack_channel_dyn,
    },
    ChannelRegistryEntry {
        key: "mattermost",
        display_name: "Mattermost",
        build: build_mattermost_channel_dyn,
    },
    ChannelRegistryEntry {
        key: "imessage",
        display_name: "iMessage",
        build: build_imessage_channel_dyn,
    },
    ChannelRegistryEntry {
        key: "matrix",
        display_name: "Matrix",
        build: build_matrix_channel_dyn,
    },
    ChannelRegistryEntry {
        key: "signal",
        display_name: "Signal",
        build: build_signal_channel_dyn,
    },
    ChannelRegistryEntry {
        key: "whatsapp",
        display_name: "WhatsApp",
        build: build_whatsapp_channel_dyn,
    },
    ChannelRegistryEntry {
        key: "email",
        display_name: "Email",
        build: build_email_channel_dyn,
    },
    ChannelRegistryEntry {
        key: "irc",
        display_name: "IRC",
        build: build_irc_channel_dyn,
    },
    ChannelRegistryEntry {
        key: "lark",
        display_name: "Lark",
        build: build_lark_channel_dyn,
    },
    ChannelRegistryEntry {
        key: "dingtalk",
        display_name: "DingTalk",
        build: build_dingtalk_channel_dyn,
    },
    ChannelRegistryEntry {
        key: "qq",
        display_name: "QQ",
        build: build_qq_channel_dyn,
    },
];

fn configured_channel_entries(config: &Config) -> Vec<ConfiguredChannelEntry> {
    CHANNEL_REGISTRY
        .iter()
        .filter_map(|entry| {
            (entry.build)(config).map(|channel| (entry.key, entry.display_name, channel))
        })
        .collect()
}

fn build_doctor_channels(config: &Config) -> Vec<DoctorChannelEntry> {
    configured_channel_entries(config)
        .into_iter()
        .map(|(_key, display_name, channel)| (display_name, channel))
        .collect()
}

pub(crate) fn build_channel(config: &Config, channel_name: &str) -> Option<Arc<dyn Channel>> {
    let channel_name = channel_name.to_ascii_lowercase();
    CHANNEL_REGISTRY
        .iter()
        .find(|entry| entry.key == channel_name.as_str())
        .and_then(|entry| (entry.build)(config))
}

pub(crate) fn is_supported_channel(channel_name: &str) -> bool {
    let channel_name = channel_name.to_ascii_lowercase();
    CHANNEL_REGISTRY
        .iter()
        .any(|entry| entry.key == channel_name.as_str())
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
    let channels: Vec<Arc<dyn Channel>> = configured_channel_entries(&config)
        .into_iter()
        .map(|(_, _, channel)| channel)
        .collect();

    if channels.is_empty() {
        println!("No channels configured. Run `corvus onboard` to set up channels.");
        return Ok(());
    }

    let model = config
        .default_model
        .clone()
        .unwrap_or_else(|| bootstrap::DEFAULT_MODEL.into());
    let provider: Arc<dyn Provider> =
        Arc::from(bootstrap::create_routed_provider(&config, &model)?);

    // Warm up the provider connection pool (TLS handshake, DNS, HTTP/2 setup)
    // so the first real message doesn't hit a cold-start timeout.
    if let Err(e) = provider.warmup().await {
        tracing::warn!("Provider warmup failed (non-fatal): {e}");
    }

    let bootstrap = bootstrap::BootstrapContext::from_config(&config)?;
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
        Some(COMPACT_CONTEXT_BOOTSTRAP_MAX_CHARS)
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
    let runtime_handle = ChannelRuntimeHandle::new(tx);

    // Spawn a listener for each channel
    let mut handles = Vec::new();
    for ch in &channels {
        handles.push(spawn_supervised_listener(
            ch.clone(),
            runtime_handle.sender(),
            initial_backoff_secs,
            max_backoff_secs,
        ));
    }
    // Drop our copy so rx closes when all channels stop
    drop(runtime_handle);

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
        transcriber: build_transcriber(&config),
    });

    run_message_dispatch_loop(rx, runtime_ctx, max_in_flight_messages).await;

    // Wait for all channel tasks
    for h in handles {
        let _ = h.await;
    }

    Ok(())
}

pub(crate) fn spawn_runtime_handle(config: &Config) -> Result<Option<ChannelRuntimeHandle>> {
    let channels: Vec<Arc<dyn Channel>> = configured_channel_entries(config)
        .into_iter()
        .map(|(_, _, channel)| channel)
        .collect();

    if channels.is_empty() {
        return Ok(None);
    }

    let workspace = config.workspace_dir.clone();
    let model = config
        .default_model
        .clone()
        .unwrap_or_else(|| bootstrap::DEFAULT_MODEL.into());
    let provider: Arc<dyn Provider> = Arc::from(bootstrap::create_routed_provider(config, &model)?);
    {
        let p = Arc::clone(&provider);
        tokio::spawn(async move {
            if let Err(e) = p.warmup().await {
                tracing::debug!("Channel provider warmup failed (non-fatal): {e}");
            }
        });
    }
    let bootstrap = bootstrap::BootstrapContext::from_config(config)?;
    let tools_registry = Arc::new(bootstrap.tools);
    let skills = crate::skills::load_skills(&workspace);
    let tool_descs: Vec<(&str, &str)> = tools_registry
        .iter()
        .map(|tool| (tool.name(), tool.description()))
        .collect();
    let bootstrap_max_chars = if config.agent.compact_context {
        Some(COMPACT_CONTEXT_BOOTSTRAP_MAX_CHARS)
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

    let channels_by_name = Arc::new(
        channels
            .iter()
            .map(|ch| (ch.name().to_string(), Arc::clone(ch)))
            .collect::<HashMap<_, _>>(),
    );
    let runtime_ctx = Arc::new(ChannelRuntimeContext {
        config: Arc::new(config.clone()),
        channels_by_name,
        provider,
        memory: bootstrap.memory,
        tools_registry,
        observer: bootstrap.observer,
        system_prompt: Arc::new(system_prompt),
        model: Arc::new(model),
        temperature: config.default_temperature,
        auto_save_memory: config.memory.auto_save,
        tool_dispatcher_mode: Arc::<str>::from(config.agent.tool_dispatcher.as_str()),
        max_tool_iterations: config.agent.max_tool_iterations,
        min_relevance_score: config.memory.min_relevance_score,
        conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        transcriber: build_transcriber(config),
    });

    let (tx, rx) = tokio::sync::mpsc::channel::<traits::ChannelMessage>(100);
    let runtime_handle = ChannelRuntimeHandle::new(tx);
    let max_in_flight_messages = compute_max_in_flight_messages(channels.len());

    tokio::spawn(run_message_dispatch_loop(
        rx,
        runtime_ctx,
        max_in_flight_messages,
    ));

    Ok(Some(runtime_handle))
}

#[cfg(test)]
#[path = "tests/health.rs"]
mod channel_health_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::prompt::DEFAULT_BOOTSTRAP_MAX_CHARS;
    use crate::config::{SlackConfig, StreamMode, TelegramConfig};
    use crate::memory::{Memory, MemoryCategory, SqliteMemory};
    use crate::observability::{ImageIngressEvent, ImageIngressOutcome, NoopObserver, Observer};
    use crate::providers::traits::ProviderCapabilities;
    use crate::providers::{ChatMessage, ChatRequest, ChatResponse, Provider, ToolCall};
    use crate::tools::{Tool, ToolResult};
    use crate::transcription::whisper_cli::WhisperCliTranscriber;
    use std::collections::HashMap;
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
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

    #[test]
    fn centralized_channel_factory_reuses_registry_for_named_lookup() {
        let mut config = Config::default();
        config.channels_config.telegram = Some(TelegramConfig {
            bot_token: "telegram-token".into(),
            allowed_users: vec!["*".into()],
            stream_mode: StreamMode::default(),
            draft_update_interval_ms: 250,
        });
        config.channels_config.slack = Some(SlackConfig {
            bot_token: "slack-token".into(),
            app_token: None,
            channel_id: Some("C123".into()),
            allowed_users: vec!["U123".into()],
        });

        let entries = configured_channel_entries(&config);
        let names: Vec<&str> = entries.iter().map(|(key, _, _)| *key).collect();

        assert_eq!(names, vec!["telegram", "slack"]);
        assert_eq!(
            build_channel(&config, "telegram").unwrap().name(),
            "telegram"
        );
        assert_eq!(
            build_channel(&config, "Telegram").unwrap().name(),
            "telegram"
        );
        assert_eq!(build_channel(&config, "slack").unwrap().name(), "slack");
        assert_eq!(build_channel(&config, "SLACK").unwrap().name(), "slack");
        assert!(build_channel(&config, "discord").is_none());
        assert!(is_supported_channel("Telegram"));
        assert!(is_supported_channel("SLACK"));
        assert!(is_supported_channel("dIsCoRd"));
        assert!(!is_supported_channel("pagerduty"));
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

    #[derive(Default)]
    struct RecordingObserver {
        image_events: std::sync::Mutex<Vec<ImageIngressEvent>>,
    }

    impl Observer for RecordingObserver {
        fn record_event(&self, _event: &crate::observability::ObserverEvent) {}

        fn record_metric(&self, _metric: &crate::observability::ObserverMetric) {}

        fn on_image_ingress(&self, event: &ImageIngressEvent) {
            self.image_events
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(event.clone());
        }

        fn name(&self) -> &str {
            "recording-observer"
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    fn make_multimodal_test_config(channel: &str) -> Config {
        Config {
            multimodal: crate::config::MultimodalConfig {
                enabled: true,
                allowed_channels: vec![channel.to_string()],
                vision_model_hint: Some("vision".into()),
                max_image_bytes: None,
            },
            model_routes: vec![crate::config::ModelRouteConfig {
                hint: "vision".into(),
                provider: "test-provider".into(),
                model: "test-vision-model".into(),
                api_key: None,
                allow_image_input: true,
            }],
            ..Config::default()
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

    #[derive(Default)]
    struct ImageAwareProvider {
        calls: AtomicUsize,
        image_counts: std::sync::Mutex<Vec<usize>>,
        models: std::sync::Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl Provider for ImageAwareProvider {
        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                image_input: true,
                image_transport_forms: vec![media::ImageTransportForm::InlineBytes],
                ..Default::default()
            }
        }

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
            request: ChatRequest<'_>,
            model: &str,
            _temperature: f64,
        ) -> anyhow::Result<ChatResponse> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.image_counts
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(request.images.len());
            self.models
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(model.to_string());
            Ok(ChatResponse {
                text: Some("image-ok".to_string()),
                tool_calls: Vec::new(),
            })
        }
    }

    #[async_trait::async_trait]
    impl Provider for Arc<ImageAwareProvider> {
        fn capabilities(&self) -> ProviderCapabilities {
            self.as_ref().capabilities()
        }

        async fn chat_with_system(
            &self,
            system_prompt: Option<&str>,
            message: &str,
            model: &str,
            temperature: f64,
        ) -> anyhow::Result<String> {
            self.as_ref()
                .chat_with_system(system_prompt, message, model, temperature)
                .await
        }

        async fn chat(
            &self,
            request: ChatRequest<'_>,
            model: &str,
            temperature: f64,
        ) -> anyhow::Result<ChatResponse> {
            self.as_ref().chat(request, model, temperature).await
        }
    }

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
                    structured: None,
                });
            }

            Ok(ToolResult {
                success: true,
                output: r#"{"symbol":"BTC","price_usd":65000}"#.to_string(),
                error: None,
                structured: None,
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
            config: Arc::new(make_multimodal_test_config("test-channel")),
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
            transcriber: None,
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
                parts: vec![],
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
            config: Arc::new(make_multimodal_test_config("test-channel")),
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
            transcriber: None,
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
                parts: vec![],
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

    /// Memory backend that records whether `store` or `recall` were called.
    #[derive(Default)]
    struct RecordingMemory {
        store_count: std::sync::atomic::AtomicUsize,
        recall_count: std::sync::atomic::AtomicUsize,
    }

    #[async_trait::async_trait]
    impl Memory for RecordingMemory {
        fn name(&self) -> &str {
            "recording"
        }

        async fn store(
            &self,
            _key: &str,
            _content: &str,
            _category: crate::memory::MemoryCategory,
            _session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            self.store_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<crate::memory::MemoryEntry>> {
            self.recall_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
            config: Arc::new(make_multimodal_test_config("test-channel")),
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
            transcriber: None,
        });

        let (tx, rx) = tokio::sync::mpsc::channel::<traits::ChannelMessage>(4);
        tx.send(traits::ChannelMessage {
            id: "1".to_string(),
            sender: "alice".to_string(),
            reply_target: "alice".to_string(),
            content: "hello".to_string(),
            channel: "test-channel".to_string(),
            timestamp: 1,
            parts: vec![],
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
            parts: vec![],
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
            config: Arc::new(make_multimodal_test_config("test-channel")),
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
            transcriber: None,
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
                parts: vec![],
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
            trust: crate::skills::trust::SkillTrust::Local,
            origin: crate::skills::trust::SkillOrigin::default(),
            allowed_tools: Vec::new(),
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
        // Write a file larger than DEFAULT_BOOTSTRAP_MAX_CHARS
        let big_content = "x".repeat(DEFAULT_BOOTSTRAP_MAX_CHARS + 1000);
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
            parts: vec![],
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
            parts: vec![],
        };
        let msg2 = traits::ChannelMessage {
            id: "msg_2".into(),
            sender: "U123".into(),
            reply_target: "C456".into(),
            content: "second".into(),
            channel: "slack".into(),
            timestamp: 2,
            parts: vec![],
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
            parts: vec![],
        };
        let msg2 = traits::ChannelMessage {
            id: "msg_2".into(),
            sender: "U123".into(),
            reply_target: "C456".into(),
            content: "I'm 45".into(),
            channel: "slack".into(),
            timestamp: 2,
            parts: vec![],
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
            config: Arc::new(make_multimodal_test_config("test-channel")),
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
            transcriber: None,
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
                parts: vec![],
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
                parts: vec![],
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
                images: &[],
            },
        )
        .await
        .unwrap();

        assert_eq!(response, "done");
        assert!(history_contains_structured_denial(
            &history,
            "approval_required",
            "mcp.docs.search"
        ));
    }

    /// Check whether conversation history contains a tool result with the given code and tool name.
    fn history_contains_structured_denial(
        history: &[ConversationMessage],
        expected_code: &str,
        expected_tool: &str,
    ) -> bool {
        history.iter().any(|message| {
            if let ConversationMessage::ToolResults(results) = message {
                results.iter().any(|result| {
                    serde_json::from_str::<serde_json::Value>(&result.content)
                        .ok()
                        .is_some_and(|parsed| {
                            parsed["code"] == expected_code && parsed["tool"] == expected_tool
                        })
                })
            } else {
                false
            }
        })
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
            config: Arc::new(make_multimodal_test_config("test-channel")),
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
            transcriber: None,
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
                parts: vec![],
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
            transcriber: None,
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
                parts: vec![],
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

    // ── ChannelRuntimeHandle (Task 1.3) ──────────────────────

    #[test]
    fn channel_runtime_handle_is_clone_send_sync() {
        fn assert_traits<T: Clone + Send + Sync>() {}
        assert_traits::<ChannelRuntimeHandle>();
    }

    #[tokio::test]
    async fn channel_runtime_handle_enqueue_delivers_message() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<traits::ChannelMessage>(4);
        let handle = ChannelRuntimeHandle::new(tx);

        let msg = traits::ChannelMessage {
            id: "h-1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: "hello".into(),
            channel: "test".into(),
            timestamp: 1,
            parts: vec![],
        };

        handle.enqueue(msg).unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, "h-1");
        assert_eq!(received.content, "hello");
    }

    #[tokio::test]
    async fn channel_runtime_handle_enqueue_fails_when_closed() {
        let (tx, rx) = tokio::sync::mpsc::channel::<traits::ChannelMessage>(1);
        let handle = ChannelRuntimeHandle::new(tx);
        drop(rx);

        let msg = traits::ChannelMessage {
            id: "h-2".into(),
            sender: "bob".into(),
            reply_target: "bob".into(),
            content: "bye".into(),
            channel: "test".into(),
            timestamp: 2,
            parts: vec![],
        };

        assert!(handle.enqueue(msg).is_err());
    }

    // ── Image turn gating (Task 1.4) ─────────────────────────

    fn make_image_part(handle: &str) -> traits::ContentPart {
        traits::ContentPart::Image {
            channel_handle: handle.into(),
            source_channel: "test".into(),
            declared_mime: Some("image/jpeg".into()),
            caption_text: None,
            file_name: None,
            declared_bytes: None,
        }
    }

    #[tokio::test]
    async fn process_rejects_too_many_images() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(make_multimodal_test_config("test-channel")),
            channels_by_name: Arc::new(channels_by_name),
            provider: Arc::new(SlowProvider {
                delay: Duration::from_millis(1),
            }),
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test".into()),
            model: Arc::new("test".into()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber: None,
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "img-many".into(),
                sender: "alice".into(),
                reply_target: "chat-img".into(),
                content: "two photos".into(),
                channel: "test-channel".into(),
                timestamp: 1,
                parts: vec![make_image_part("f1"), make_image_part("f2")],
            },
        )
        .await;

        let sent = channel_impl.sent_messages.lock().await;
        assert_eq!(sent.len(), 1);
        assert!(
            sent[0].contains("Too many images"),
            "expected too-many-images rejection, got: {}",
            sent[0]
        );
    }

    #[tokio::test]
    async fn process_rejects_unstaged_image_turn() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(make_multimodal_test_config("test-channel")),
            channels_by_name: Arc::new(channels_by_name),
            provider: Arc::new(SlowProvider {
                delay: Duration::from_millis(1),
            }),
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test".into()),
            model: Arc::new("test".into()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber: None,
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "img-one".into(),
                sender: "bob".into(),
                reply_target: "chat-img2".into(),
                content: "one photo".into(),
                channel: "test-channel".into(),
                timestamp: 1,
                parts: vec![make_image_part("f1")],
            },
        )
        .await;

        let sent = channel_impl.sent_messages.lock().await;
        assert_eq!(sent.len(), 1);
        assert!(
            sent[0].contains("not yet supported"),
            "expected unsupported rejection, got: {}",
            sent[0]
        );
    }

    #[tokio::test]
    async fn process_rejects_when_multimodal_disabled() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let observer_impl = Arc::new(RecordingObserver::default());
        let observer: Arc<dyn Observer> = observer_impl.clone();
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
            observer,
            system_prompt: Arc::new("test".into()),
            model: Arc::new("test".into()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber: None,
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "img-disabled".into(),
                sender: "alice".into(),
                reply_target: "chat-disabled".into(),
                content: "photo".into(),
                channel: "test-channel".into(),
                timestamp: 1,
                parts: vec![make_image_part("f1")],
            },
        )
        .await;

        let sent = channel_impl.sent_messages.lock().await;
        assert_eq!(sent.len(), 1);
        assert!(sent[0].contains("currently disabled"));

        let events = observer_impl
            .image_events
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].outcome, ImageIngressOutcome::Rejected);
        assert_eq!(
            events[0].reason,
            Some(crate::observability::ImageIngressReason::Disabled)
        );
    }

    #[tokio::test]
    async fn process_rejects_when_channel_not_allowed() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let observer_impl = Arc::new(RecordingObserver::default());
        let observer: Arc<dyn Observer> = observer_impl.clone();
        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(make_multimodal_test_config("telegram")),
            channels_by_name: Arc::new(channels_by_name),
            provider: Arc::new(SlowProvider {
                delay: Duration::from_millis(1),
            }),
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer,
            system_prompt: Arc::new("test".into()),
            model: Arc::new("test".into()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber: None,
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "img-channel-blocked".into(),
                sender: "alice".into(),
                reply_target: "chat-channel-blocked".into(),
                content: "photo".into(),
                channel: "test-channel".into(),
                timestamp: 1,
                parts: vec![make_image_part("f1")],
            },
        )
        .await;

        let sent = channel_impl.sent_messages.lock().await;
        assert_eq!(sent.len(), 1);
        assert!(sent[0].contains("not enabled for this channel"));

        let events = observer_impl
            .image_events
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].outcome, ImageIngressOutcome::Rejected);
        assert_eq!(
            events[0].reason,
            Some(crate::observability::ImageIngressReason::ChannelNotAllowed)
        );
    }

    // ── resolve_image_route unit tests ────────────────────────

    #[test]
    fn resolve_image_route_succeeds_with_valid_config() {
        let config = make_multimodal_test_config("telegram");
        let route = resolve_image_route(&config).unwrap();
        assert_eq!(route.selector, "hint:vision");
        assert_eq!(route.provider, "test-provider");
        assert_eq!(route.model, "test-vision-model");
    }

    #[test]
    fn resolve_image_route_fails_when_hint_missing() {
        let mut config = make_multimodal_test_config("telegram");
        config.multimodal.vision_model_hint = None;
        let err = resolve_image_route(&config).unwrap_err();
        assert!(matches!(
            err,
            media::ImageRejectionReason::MissingVisionRoute
        ));
    }

    #[test]
    fn resolve_image_route_fails_when_hint_empty() {
        let mut config = make_multimodal_test_config("telegram");
        config.multimodal.vision_model_hint = Some("  ".into());
        let err = resolve_image_route(&config).unwrap_err();
        assert!(matches!(
            err,
            media::ImageRejectionReason::MissingVisionRoute
        ));
    }

    #[test]
    fn resolve_image_route_fails_when_no_matching_route() {
        let mut config = make_multimodal_test_config("telegram");
        config.multimodal.vision_model_hint = Some("nonexistent".into());
        let err = resolve_image_route(&config).unwrap_err();
        assert!(matches!(
            err,
            media::ImageRejectionReason::MissingVisionRoute
        ));
    }

    #[test]
    fn resolve_image_route_fails_when_route_not_image_capable() {
        let mut config = make_multimodal_test_config("telegram");
        config.model_routes[0].allow_image_input = false;
        let err = resolve_image_route(&config).unwrap_err();
        assert!(matches!(
            err,
            media::ImageRejectionReason::RouteNotImageCapable
        ));
    }

    // ── rejection_to_ingress_reason mapping tests ───────────

    #[test]
    fn rejection_to_ingress_reason_maps_all_variants() {
        use crate::observability::ImageIngressReason;
        let cases = vec![
            (
                media::ImageRejectionReason::Disabled,
                ImageIngressReason::Disabled,
            ),
            (
                media::ImageRejectionReason::ChannelNotAllowed,
                ImageIngressReason::ChannelNotAllowed,
            ),
            (
                media::ImageRejectionReason::MissingVisionRoute,
                ImageIngressReason::MissingVisionRoute,
            ),
            (
                media::ImageRejectionReason::RouteNotImageCapable,
                ImageIngressReason::RouteNotImageCapable,
            ),
            (
                media::ImageRejectionReason::FetchFailed,
                ImageIngressReason::FetchFailed,
            ),
            (
                media::ImageRejectionReason::MimeRejected,
                ImageIngressReason::MimeRejected,
            ),
            (
                media::ImageRejectionReason::Oversize,
                ImageIngressReason::Oversize,
            ),
            (
                media::ImageRejectionReason::TooManyImages,
                ImageIngressReason::TooManyImages,
            ),
            (
                media::ImageRejectionReason::ProviderError,
                ImageIngressReason::ProviderError,
            ),
            (
                media::ImageRejectionReason::ChannelNotSupported,
                ImageIngressReason::ChannelNotSupported,
            ),
        ];
        for (rejection, expected) in cases {
            assert_eq!(rejection_to_ingress_reason(&rejection), expected);
        }
    }

    // ── channel_delivery_instructions tests ─────────────────

    #[test]
    fn delivery_instructions_telegram_returns_marker_guidance() {
        let instructions = channel_delivery_instructions("telegram");
        assert!(instructions.is_some());
        assert!(instructions.unwrap().contains("[IMAGE:"));
    }

    #[test]
    fn delivery_instructions_unknown_channel_returns_none() {
        assert!(channel_delivery_instructions("discord").is_none());
        assert!(channel_delivery_instructions("whatsapp").is_none());
        assert!(channel_delivery_instructions("slack").is_none());
    }

    // ── StagedImageGuard (Task 1.5) ─────────────────────────

    #[test]
    fn staged_image_guard_cleanup_called_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let tmp = dir.path().join("guard_test.jpg");
        std::fs::write(&tmp, b"fake-image").unwrap();
        assert!(tmp.exists());

        {
            let _guard = StagedImageGuard(vec![media::StagedImage {
                sha256: "abc".into(),
                mime_type: media::AllowedImageMime::Jpeg,
                byte_len: 10,
                temp_path: tmp.clone(),
                transport_form: media::ImageTransportForm::InlineBytes,
                channel_origin: "test".into(),
            }]);
            // guard dropped here
        }

        assert!(!tmp.exists(), "temp file should be removed on guard drop");
    }

    #[tokio::test]
    async fn process_text_only_unaffected_by_image_gating() {
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
            system_prompt: Arc::new("test".into()),
            model: Arc::new("test".into()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber: None,
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "text-only".into(),
                sender: "carol".into(),
                reply_target: "chat-text".into(),
                content: "just text".into(),
                channel: "test-channel".into(),
                timestamp: 1,
                parts: vec![],
            },
        )
        .await;

        let sent = channel_impl.sent_messages.lock().await;
        assert_eq!(sent.len(), 1);
        assert!(
            !sent[0].contains("not yet supported"),
            "text-only should not be rejected"
        );
        assert!(
            !sent[0].contains("Too many images"),
            "text-only should not trigger image rejection"
        );
    }

    #[tokio::test]
    async fn image_only_message_skips_memory_recall_and_store() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let memory_impl = Arc::new(RecordingMemory::default());
        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(make_multimodal_test_config("test-channel")),
            channels_by_name: Arc::new(channels_by_name),
            provider: Arc::new(SlowProvider {
                delay: Duration::from_millis(1),
            }),
            memory: memory_impl.clone(),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test".into()),
            model: Arc::new("test".into()),
            temperature: 0.0,
            auto_save_memory: true,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber: None,
        });

        // Image-only message: content is empty, text_projection is empty,
        // parts has only an image (no text part).
        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "img-only-mem".into(),
                sender: "alice".into(),
                reply_target: "chat-mem".into(),
                content: String::new(),
                channel: "test-channel".into(),
                timestamp: 1,
                parts: vec![make_image_part("f1")],
            },
        )
        .await;

        let stores = memory_impl
            .store_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let recalls = memory_impl
            .recall_count
            .load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(
            stores, 0,
            "image-only message should not autosave to memory"
        );
        assert_eq!(
            recalls, 0,
            "image-only message should not recall from memory"
        );
    }

    #[tokio::test]
    async fn run_unified_channel_tool_loop_forwards_staged_images_to_provider() {
        let provider = ImageAwareProvider::default();
        let history = &mut vec![ConversationMessage::Chat(ChatMessage::user(
            "describe this",
        ))];
        let temp_dir = tempfile::tempdir().unwrap();
        let image_path = temp_dir.path().join("image.jpg");
        std::fs::write(&image_path, b"fake-image").unwrap();
        let staged = vec![media::StagedImage {
            sha256: "hash".into(),
            mime_type: media::AllowedImageMime::Jpeg,
            byte_len: 10,
            temp_path: image_path,
            transport_form: media::ImageTransportForm::InlineBytes,
            channel_origin: "whatsapp:test".into(),
        }];

        let response = run_unified_channel_tool_loop(
            &provider,
            &[],
            history,
            ChannelLoopParams {
                model: "test-model",
                temperature: 0.0,
                max_tool_iterations: 1,
                dispatcher_mode: "xml",
                delta_tx: None,
                images: &staged,
            },
        )
        .await
        .unwrap();

        assert_eq!(response, "image-ok");
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            provider
                .image_counts
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_slice(),
            &[1]
        );
        assert_eq!(
            provider
                .models
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_slice(),
            &["test-model".to_string()]
        );
    }

    #[tokio::test]
    async fn run_unified_channel_tool_loop_uses_hint_model_for_image_execution() {
        let provider = ImageAwareProvider::default();
        let history = &mut vec![ConversationMessage::Chat(ChatMessage::user(
            "describe this",
        ))];
        let temp_dir = tempfile::tempdir().unwrap();
        let image_path = temp_dir.path().join("image.jpg");
        std::fs::write(&image_path, b"fake-image").unwrap();
        let staged = vec![media::StagedImage {
            sha256: "hash".into(),
            mime_type: media::AllowedImageMime::Jpeg,
            byte_len: 10,
            temp_path: image_path,
            transport_form: media::ImageTransportForm::InlineBytes,
            channel_origin: "whatsapp:test".into(),
        }];

        let response = run_unified_channel_tool_loop(
            &provider,
            &[],
            history,
            ChannelLoopParams {
                model: "hint:vision",
                temperature: 0.0,
                max_tool_iterations: 1,
                dispatcher_mode: "xml",
                delta_tx: None,
                images: &staged,
            },
        )
        .await
        .unwrap();

        assert_eq!(response, "image-ok");
        assert_eq!(
            provider
                .models
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_slice(),
            &["hint:vision".to_string()]
        );
    }

    #[tokio::test]
    async fn rejected_image_turn_skips_provider_dispatch() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let provider_impl = Arc::new(ImageAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(Config::default()),
            channels_by_name: Arc::new(channels_by_name),
            provider,
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test".into()),
            model: Arc::new("test".into()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber: None,
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "img-rejected".into(),
                sender: "alice".into(),
                reply_target: "chat-rejected".into(),
                content: "photo".into(),
                channel: "test-channel".into(),
                timestamp: 1,
                parts: vec![make_image_part("f1")],
            },
        )
        .await;

        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 0);
        let sent = channel_impl.sent_messages.lock().await;
        assert_eq!(sent.len(), 1);
        assert!(sent[0].contains("currently disabled"));
    }

    #[test]
    fn image_turn_prefers_vision_route_selector_for_provider_execution() {
        let route = ResolvedImageRoute {
            selector: "hint:vision".into(),
            provider: "test-provider".into(),
            model: "test-vision-model".into(),
        };

        assert_eq!(
            execution_model_for_turn("default-text-model", Some(&route)),
            "hint:vision"
        );
    }

    #[tokio::test]
    async fn process_text_only_turn_keeps_default_model_route() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let provider_impl = Arc::new(ImageAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(make_multimodal_test_config("test-channel")),
            channels_by_name: Arc::new(channels_by_name),
            provider,
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test".into()),
            model: Arc::new("default-text-model".into()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber: None,
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "text-default".into(),
                sender: "alice".into(),
                reply_target: "chat-default".into(),
                content: "hello".into(),
                channel: "test-channel".into(),
                timestamp: 1,
                parts: vec![traits::ContentPart::Text {
                    text: "hello".into(),
                }],
            },
        )
        .await;

        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            provider_impl
                .models
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_slice(),
            &["default-text-model".to_string()]
        );
    }

    #[tokio::test]
    async fn mime_rejection_skips_provider_dispatch() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let provider_impl = Arc::new(ImageAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(make_multimodal_test_config("test-channel")),
            channels_by_name: Arc::new(channels_by_name),
            provider,
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test".into()),
            model: Arc::new("test".into()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber: None,
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "img-mime-rejected".into(),
                sender: "alice".into(),
                reply_target: "chat-mime-rejected".into(),
                content: "photo".into(),
                channel: "test-channel".into(),
                timestamp: 1,
                parts: vec![make_image_part("__reject_mime__")],
            },
        )
        .await;

        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 0);
        let sent = channel_impl.sent_messages.lock().await;
        assert_eq!(sent.len(), 1);
        assert!(!sent[0].is_empty());
    }

    #[tokio::test]
    async fn oversize_rejection_skips_provider_dispatch() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let provider_impl = Arc::new(ImageAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(make_multimodal_test_config("test-channel")),
            channels_by_name: Arc::new(channels_by_name),
            provider,
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(NoopObserver),
            system_prompt: Arc::new("test".into()),
            model: Arc::new("test".into()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber: None,
        });

        process_channel_message(
            runtime_ctx,
            traits::ChannelMessage {
                id: "img-oversize-rejected".into(),
                sender: "alice".into(),
                reply_target: "chat-oversize-rejected".into(),
                content: "photo".into(),
                channel: "test-channel".into(),
                timestamp: 1,
                parts: vec![make_image_part("__reject_oversize__")],
            },
        )
        .await;

        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 0);
        let sent = channel_impl.sent_messages.lock().await;
        assert_eq!(sent.len(), 1);
        assert!(!sent[0].is_empty());
    }

    // ── Audio integration tests (Phase 4) ────────────────────

    /// Mock transcriber that returns a configurable text result.
    /// Used to test the audio pipeline without a real whisper binary.
    struct MockTranscriber {
        response_text: String,
        delay: Duration,
        call_count: AtomicUsize,
    }

    impl MockTranscriber {
        fn new(text: &str) -> Self {
            Self {
                response_text: text.to_string(),
                delay: Duration::from_millis(0),
                call_count: AtomicUsize::new(0),
            }
        }

        fn with_delay(text: &str, delay: Duration) -> Self {
            Self {
                response_text: text.to_string(),
                delay,
                call_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl crate::transcription::traits::Transcriber for MockTranscriber {
        fn name(&self) -> &str {
            "mock-transcriber"
        }

        async fn transcribe(
            &self,
            audio: &audio_media::StagedAudio,
        ) -> Result<
            crate::transcription::traits::TranscriptionResult,
            audio_media::AudioRejectionReason,
        > {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            Ok(crate::transcription::traits::TranscriptionResult {
                text: self.response_text.clone(),
                language: Some("es".into()),
                duration_secs: audio.duration_secs,
                confidence: Some(0.95),
                processing_ms: None,
            })
        }

        async fn health_check(&self) -> Result<(), String> {
            Ok(())
        }
    }

    /// Recording observer that captures audio ingress events.
    #[derive(Default)]
    struct AudioRecordingObserver {
        audio_events: std::sync::Mutex<Vec<crate::observability::AudioIngressEvent>>,
    }

    impl Observer for AudioRecordingObserver {
        fn record_event(&self, _event: &crate::observability::ObserverEvent) {}
        fn record_metric(&self, _metric: &crate::observability::ObserverMetric) {}

        fn on_audio_ingress(&self, event: &crate::observability::AudioIngressEvent) {
            self.audio_events
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(event.clone());
        }

        fn name(&self) -> &str {
            "audio-recording-observer"
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    fn make_audio_test_config(channel: &str) -> Config {
        Config {
            audio: crate::config::AudioConfig {
                enabled: true,
                allowed_channels: vec![channel.to_string()],
                ..crate::config::AudioConfig::default()
            },
            ..Config::default()
        }
    }

    /// Create a staged audio temp file for testing.
    fn make_test_staged_audio(dir: &std::path::Path) -> audio_media::StagedAudio {
        let tmp = dir.join("corvus-tg-aud-testsha256abcdef.ogg");
        // Write valid OGG magic bytes + some padding
        let mut bytes = vec![0u8; 64];
        bytes[0..4].copy_from_slice(b"OggS");
        std::fs::write(&tmp, &bytes).unwrap();

        audio_media::StagedAudio {
            sha256: "testsha256abcdef1234567890abcdef".into(),
            mime_type: audio_media::AllowedAudioMime::OggOpus,
            byte_len: 64,
            duration_secs: Some(5.0),
            temp_path: tmp,
            channel_origin: "telegram".into(),
        }
    }

    fn make_audio_channel_message(parts: Vec<traits::ContentPart>) -> traits::ChannelMessage {
        traits::ChannelMessage {
            id: "audio-test-1".into(),
            sender: "alice".into(),
            reply_target: "chat-audio-test".into(),
            content: String::new(),
            channel: "test-channel".into(),
            timestamp: 1,
            parts,
        }
    }

    #[cfg(unix)]
    fn create_real_whisper_transcriber(
        dir: &TempDir,
        script_name: &str,
        script_body: &str,
        concurrency: usize,
    ) -> Arc<dyn crate::transcription::traits::Transcriber> {
        let model_path = dir.path().join("ggml-base.bin");
        fs::write(&model_path, b"fake-model").unwrap();

        let script_path = dir.path().join(script_name);
        fs::write(&script_path, script_body).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        Arc::new(WhisperCliTranscriber::new_for_tests(
            script_path.display().to_string(),
            model_path,
            "es".into(),
            5,
            concurrency,
        ))
    }

    // ── Task 4.2: Integration test — happy path ─────────────

    #[cfg(unix)]
    #[tokio::test]
    async fn audio_pipeline_happy_path_uses_real_transcriber_and_emits_admitted_event() {
        let tmp = tempfile::tempdir().unwrap();
        let staged = make_test_staged_audio(tmp.path());
        let temp_path = staged.temp_path.clone();
        let transcriber = create_real_whisper_transcriber(
            &tmp,
            "fake-whisper.sh",
            r#"#!/bin/sh
set -eu
printf 'Known mock transcription\n'
"#,
            1,
        );
        let observer = Arc::new(AudioRecordingObserver::default());
        let runtime_ctx = ChannelRuntimeContext {
            config: Arc::new(make_audio_test_config("test-channel")),
            channels_by_name: Arc::new(HashMap::new()),
            provider: Arc::new(SlowProvider {
                delay: Duration::from_millis(10),
            }),
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: observer.clone(),
            system_prompt: Arc::new("test".into()),
            model: Arc::new("test".into()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber: Some(transcriber),
        };

        assert!(temp_path.exists());
        let mut msg = make_audio_channel_message(vec![traits::ContentPart::Audio {
            channel_handle: "file123".into(),
            source_channel: "telegram".into(),
            declared_mime: Some("audio/ogg".into()),
            caption_text: None,
            file_name: None,
            declared_bytes: Some(64),
            declared_duration_secs: Some(5),
        }]);

        let guard = StagedAudioGuard(vec![staged]);
        let transcriptions = transcribe_audio(&runtime_ctx, &guard.0, "session-audio", None, &msg)
            .await
            .expect("mock whisper should succeed");

        for (audio, tx) in guard.0.iter().zip(transcriptions.iter()) {
            emit_audio_ingress(
                runtime_ctx.observer.as_ref(),
                &msg.channel,
                crate::observability::AudioIngressOutcome::Admitted,
                None,
                Some(audio.mime_type.as_str().to_string()),
                Some(audio.byte_len),
                audio.duration_secs,
                tx.processing_ms,
            );
        }

        let history_metas = inject_transcription(&mut msg, &guard.0, &transcriptions);

        assert!(!msg.parts.is_empty());
        let has_text_part = msg.parts.iter().any(|p| {
            if let traits::ContentPart::Text { text } = p {
                text.contains("Known mock transcription")
            } else {
                false
            }
        });
        assert!(has_text_part, "transcription text not found in parts");
        assert!(!msg.has_audio_parts(), "audio parts should be replaced");
        assert_eq!(history_metas.len(), 1);
        assert_eq!(history_metas[0].transcription, "Known mock transcription");
        assert_eq!(history_metas[0].mime, "audio/ogg");
        assert_eq!(history_metas[0].channel_origin, "telegram");

        let events = observer.audio_events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].outcome,
            crate::observability::AudioIngressOutcome::Admitted
        );
        assert_eq!(events[0].reason, None);
        assert_eq!(events[0].mime_type.as_deref(), Some("audio/ogg"));
        drop(events);

        drop(guard);
        assert!(
            !temp_path.exists(),
            "temp file should be cleaned up by guard"
        );
    }

    #[tokio::test]
    async fn audio_pipeline_observability_event_emitted() {
        let observer = Arc::new(AudioRecordingObserver::default());

        // Emit an admitted event (simulating what process_channel_message does)
        emit_audio_ingress(
            observer.as_ref(),
            "telegram",
            crate::observability::AudioIngressOutcome::Admitted,
            None,
            Some("audio/ogg".into()),
            Some(64),
            Some(5.0),
            Some(150),
        );

        let events = observer.audio_events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].outcome,
            crate::observability::AudioIngressOutcome::Admitted
        );
        assert!(events[0].reason.is_none());
        assert_eq!(events[0].mime_type, Some("audio/ogg".into()));
        assert_eq!(events[0].byte_len, Some(64));
        assert_eq!(events[0].duration_secs, Some(5.0));
        assert_eq!(events[0].transcription_duration_ms, Some(150));
    }

    #[tokio::test]
    async fn audio_pipeline_temp_file_cleaned_on_error() {
        let tmp = tempfile::tempdir().unwrap();
        let staged = make_test_staged_audio(tmp.path());
        let temp_path = staged.temp_path.clone();
        assert!(temp_path.exists());

        // Simulate an error path: guard is dropped without transcription
        {
            let _guard = StagedAudioGuard(vec![staged]);
            // Error occurs, guard drops
        }
        assert!(
            !temp_path.exists(),
            "temp file should be cleaned up on error path"
        );
    }

    // ── Task 4.3: Integration test — regression ─────────────

    #[tokio::test]
    async fn text_only_message_unaffected_when_audio_enabled() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();

        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);

        let mock_transcriber: Arc<dyn crate::transcription::traits::Transcriber> =
            Arc::new(MockTranscriber::new("should not be called"));

        let provider = Arc::new(SlowProvider {
            delay: Duration::from_millis(10),
        });

        let runtime_ctx = Arc::new(ChannelRuntimeContext {
            config: Arc::new(make_audio_test_config("test-channel")),
            channels_by_name: Arc::new(channels_by_name),
            provider,
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer: Arc::new(AudioRecordingObserver::default()),
            system_prompt: Arc::new("test".into()),
            model: Arc::new("test".into()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber: Some(mock_transcriber.clone()),
        });

        // Text-only message — audio pipeline should NOT be invoked
        let text_msg = traits::ChannelMessage {
            id: "text-regression-1".into(),
            sender: "alice".into(),
            reply_target: "chat-text-regression".into(),
            content: "hello world".into(),
            channel: "test-channel".into(),
            timestamp: 1,
            parts: vec![traits::ContentPart::Text {
                text: "hello world".into(),
            }],
        };

        assert!(!text_msg.has_audio_parts());

        // Process the message — should go through normal text path
        process_channel_message(runtime_ctx.clone(), text_msg).await;

        // Provider should have been called (text processed normally)
        // and the channel should have received a response
        let sent = channel_impl.sent_messages.lock().await;
        assert!(
            !sent.is_empty(),
            "text message should have been processed and responded to"
        );
    }

    #[tokio::test]
    async fn image_only_message_unaffected_when_audio_enabled() {
        // An image-only message should flow through the image pipeline,
        // not the audio pipeline, even when audio is enabled.
        let msg = traits::ChannelMessage {
            id: "image-regression-1".into(),
            sender: "bob".into(),
            reply_target: "chat-image-regression".into(),
            content: "photo".into(),
            channel: "telegram".into(),
            timestamp: 1,
            parts: vec![traits::ContentPart::Image {
                channel_handle: "photo123".into(),
                source_channel: "telegram".into(),
                declared_mime: Some("image/jpeg".into()),
                caption_text: None,
                file_name: None,
                declared_bytes: None,
            }],
        };

        assert!(
            !msg.has_audio_parts(),
            "image message should have no audio parts"
        );
        assert!(
            msg.parts
                .iter()
                .any(|p| matches!(p, traits::ContentPart::Image { .. })),
            "image part should be present"
        );
    }

    // ── Task 4.4: Integration test — concurrency semaphore ──

    #[cfg(unix)]
    #[tokio::test]
    async fn transcription_semaphore_enforces_serial_execution() {
        let tmp = tempfile::tempdir().unwrap();
        let transcriber = create_real_whisper_transcriber(
            &tmp,
            "delayed-whisper.sh",
            r#"#!/bin/sh
set -eu
sleep 0.1
printf 'Hola mundo\n'
"#,
            1,
        );

        let staged1 = make_test_staged_audio(tmp.path());
        let staged2 = {
            let mut s = make_test_staged_audio(tmp.path());
            let p = tmp.path().join("corvus-tg-aud-testsha256second.ogg");
            let mut bytes = vec![0_u8; 64];
            bytes[0..4].copy_from_slice(b"OggS");
            std::fs::write(&p, &bytes).unwrap();
            s.temp_path = p;
            s.sha256 = "testsha256second1234567890abcdef".into();
            s
        };

        let tx1 = transcriber.clone();
        let tx2 = transcriber.clone();
        let started = std::time::Instant::now();

        let t1 = tokio::spawn(async move { tx1.transcribe(&staged1).await });
        let t2 = tokio::spawn(async move { tx2.transcribe(&staged2).await });

        let (r1, r2) = tokio::join!(t1, t2);
        let elapsed = started.elapsed();

        assert!(r1.unwrap().is_ok());
        assert!(r2.unwrap().is_ok());
        assert!(
            elapsed >= Duration::from_millis(190),
            "expected serial execution (>=190ms), got {:?}",
            elapsed
        );
    }

    // ── audio_rejection_user_text — all 11 variants (coverage) ──

    #[test]
    fn audio_rejection_user_text_disabled() {
        let config = Config::default();
        let text =
            audio_rejection_user_text("s1", &audio_media::AudioRejectionReason::Disabled, &config);
        assert!(text.contains("[session:s1]"));
        assert!(text.contains("Audio input is currently disabled"));
    }

    #[test]
    fn audio_rejection_user_text_channel_not_allowed() {
        let config = Config::default();
        let text = audio_rejection_user_text(
            "s2",
            &audio_media::AudioRejectionReason::ChannelNotAllowed,
            &config,
        );
        assert!(text.contains("not enabled for this channel"));
    }

    #[test]
    fn audio_rejection_user_text_fetch_failed() {
        let config = Config::default();
        let text = audio_rejection_user_text(
            "s3",
            &audio_media::AudioRejectionReason::FetchFailed,
            &config,
        );
        assert!(text.contains("couldn't download"));
    }

    #[test]
    fn audio_rejection_user_text_mime_rejected() {
        let config = Config::default();
        let text = audio_rejection_user_text(
            "s4",
            &audio_media::AudioRejectionReason::MimeRejected,
            &config,
        );
        assert!(text.contains("not supported"));
        assert!(text.contains("OGG"));
    }

    #[test]
    fn audio_rejection_user_text_oversize() {
        let config = Config::default();
        let text =
            audio_rejection_user_text("s5", &audio_media::AudioRejectionReason::Oversize, &config);
        assert!(text.contains("too large"));
        assert!(text.contains("MB"));
    }

    #[test]
    fn audio_rejection_user_text_too_long() {
        let config = Config::default();
        let text =
            audio_rejection_user_text("s6", &audio_media::AudioRejectionReason::TooLong, &config);
        assert!(text.contains("too long"));
        assert!(text.contains("minutes"));
    }

    #[test]
    fn audio_rejection_user_text_corrupted() {
        let config = Config::default();
        let text =
            audio_rejection_user_text("s7", &audio_media::AudioRejectionReason::Corrupted, &config);
        assert!(text.contains("corrupted"));
    }

    #[test]
    fn audio_rejection_user_text_transcriber_unavailable() {
        let config = Config::default();
        let text = audio_rejection_user_text(
            "s8",
            &audio_media::AudioRejectionReason::TranscriberUnavailable,
            &config,
        );
        assert!(text.contains("not available"));
        assert!(text.contains("text instead"));
    }

    #[test]
    fn audio_rejection_user_text_transcription_failed() {
        let config = Config::default();
        let text = audio_rejection_user_text(
            "s9",
            &audio_media::AudioRejectionReason::TranscriptionFailed,
            &config,
        );
        assert!(text.contains("transcription failed"));
    }

    #[test]
    fn audio_rejection_user_text_no_speech_detected() {
        let config = Config::default();
        let text = audio_rejection_user_text(
            "s10",
            &audio_media::AudioRejectionReason::NoSpeechDetected,
            &config,
        );
        assert!(text.contains("No speech was detected"));
    }

    #[test]
    fn audio_rejection_user_text_system_error() {
        let config = Config::default();
        let text = audio_rejection_user_text(
            "s11",
            &audio_media::AudioRejectionReason::SystemError,
            &config,
        );
        assert!(text.contains("internal error"));
    }

    #[test]
    fn audio_rejection_user_text_oversize_uses_config_max() {
        let mut config = Config::default();
        config.audio.max_audio_bytes = 50 * 1024 * 1024;
        let text = audio_rejection_user_text(
            "s-size",
            &audio_media::AudioRejectionReason::Oversize,
            &config,
        );
        assert!(text.contains("50 MB"), "expected 50 MB, got: {text}");
    }

    #[test]
    fn audio_rejection_user_text_too_long_uses_config_max() {
        let mut config = Config::default();
        config.audio.max_audio_duration_secs = 1800;
        let text = audio_rejection_user_text(
            "s-dur",
            &audio_media::AudioRejectionReason::TooLong,
            &config,
        );
        assert!(text.contains("30 minutes"), "expected 30 min, got: {text}");
    }

    // ── audio_rejection_to_ingress_reason — all 12 variants ──

    #[test]
    fn audio_rejection_to_ingress_reason_maps_all_variants() {
        use crate::observability::AudioIngressReason;
        let cases = vec![
            (
                audio_media::AudioRejectionReason::Disabled,
                AudioIngressReason::Disabled,
            ),
            (
                audio_media::AudioRejectionReason::ChannelNotAllowed,
                AudioIngressReason::ChannelNotAllowed,
            ),
            (
                audio_media::AudioRejectionReason::FetchFailed,
                AudioIngressReason::FetchFailed,
            ),
            (
                audio_media::AudioRejectionReason::MimeRejected,
                AudioIngressReason::MimeRejected,
            ),
            (
                audio_media::AudioRejectionReason::Oversize,
                AudioIngressReason::Oversize,
            ),
            (
                audio_media::AudioRejectionReason::TooLong,
                AudioIngressReason::TooLong,
            ),
            (
                audio_media::AudioRejectionReason::Corrupted,
                AudioIngressReason::Corrupted,
            ),
            (
                audio_media::AudioRejectionReason::TranscriptionFailed,
                AudioIngressReason::TranscriptionFailed,
            ),
            (
                audio_media::AudioRejectionReason::NoSpeechDetected,
                AudioIngressReason::NoSpeechDetected,
            ),
            (
                audio_media::AudioRejectionReason::TranscriberUnavailable,
                AudioIngressReason::TranscriberUnavailable,
            ),
            (
                audio_media::AudioRejectionReason::MultipleAudioParts,
                AudioIngressReason::MultipleAudioParts,
            ),
            (
                audio_media::AudioRejectionReason::SystemError,
                AudioIngressReason::SystemError,
            ),
        ];
        for (rejection, expected) in cases {
            assert_eq!(
                audio_rejection_to_ingress_reason(&rejection),
                expected,
                "mismatch for {rejection:?}"
            );
        }
    }

    // ── inject_transcription — caption and multi-part tests ──

    #[test]
    fn inject_transcription_preserves_caption_text() {
        let tmp = tempfile::tempdir().unwrap();
        let staged = make_test_staged_audio(tmp.path());

        let transcriptions = vec![crate::transcription::traits::TranscriptionResult {
            text: "Hola mundo".to_string(),
            language: Some("es".into()),
            duration_secs: Some(5.0),
            confidence: Some(0.9),
            processing_ms: None,
        }];

        let mut msg = make_audio_channel_message(vec![traits::ContentPart::Audio {
            channel_handle: "file123".into(),
            source_channel: "telegram".into(),
            declared_mime: Some("audio/ogg".into()),
            caption_text: Some("translate this".into()),
            file_name: None,
            declared_bytes: Some(64),
            declared_duration_secs: Some(5),
        }]);

        let history_metas =
            inject_transcription(&mut msg, std::slice::from_ref(&staged), &transcriptions);

        let text_part = msg.parts.iter().find_map(|p| {
            if let traits::ContentPart::Text { text } = p {
                Some(text.clone())
            } else {
                None
            }
        });
        assert!(text_part.is_some());
        assert!(
            text_part
                .as_ref()
                .unwrap()
                .contains("[Audio transcription]"),
            "expected '[Audio transcription]' prefix, got: {}",
            text_part.unwrap()
        );

        assert_eq!(history_metas.len(), 1);
        assert_eq!(history_metas[0].caption, Some("translate this".to_string()));
    }

    #[test]
    fn inject_transcription_voice_without_caption() {
        let tmp = tempfile::tempdir().unwrap();
        let staged = make_test_staged_audio(tmp.path());

        let transcriptions = vec![crate::transcription::traits::TranscriptionResult {
            text: "Buenos días".to_string(),
            language: Some("es".into()),
            duration_secs: Some(3.0),
            confidence: None,
            processing_ms: None,
        }];

        let mut msg = make_audio_channel_message(vec![traits::ContentPart::Audio {
            channel_handle: "file456".into(),
            source_channel: "telegram".into(),
            declared_mime: Some("audio/ogg".into()),
            caption_text: None,
            file_name: None,
            declared_bytes: Some(64),
            declared_duration_secs: Some(3),
        }]);

        let history_metas =
            inject_transcription(&mut msg, std::slice::from_ref(&staged), &transcriptions);

        let text_part = msg.parts.iter().find_map(|p| {
            if let traits::ContentPart::Text { text } = p {
                Some(text.clone())
            } else {
                None
            }
        });
        assert!(text_part.is_some());
        assert!(
            text_part
                .as_ref()
                .unwrap()
                .contains("[Voice message transcription]"),
            "expected '[Voice message transcription]' prefix, got: {}",
            text_part.unwrap()
        );

        assert_eq!(history_metas[0].caption, None);
    }

    #[test]
    fn inject_transcription_updates_content_field() {
        let tmp = tempfile::tempdir().unwrap();
        let staged = make_test_staged_audio(tmp.path());

        let transcriptions = vec![crate::transcription::traits::TranscriptionResult {
            text: "Updated content".to_string(),
            language: Some("es".into()),
            duration_secs: Some(2.0),
            confidence: None,
            processing_ms: None,
        }];

        let mut msg = make_audio_channel_message(vec![traits::ContentPart::Audio {
            channel_handle: "file789".into(),
            source_channel: "telegram".into(),
            declared_mime: Some("audio/ogg".into()),
            caption_text: None,
            file_name: None,
            declared_bytes: Some(64),
            declared_duration_secs: Some(2),
        }]);

        assert!(msg.content.is_empty(), "content should start empty");

        inject_transcription(&mut msg, std::slice::from_ref(&staged), &transcriptions);

        assert!(
            !msg.content.is_empty(),
            "content should be updated after injection"
        );
        assert!(msg.content.contains("Updated content"));
    }

    #[test]
    fn inject_transcription_preserves_text_parts() {
        let tmp = tempfile::tempdir().unwrap();
        let staged = make_test_staged_audio(tmp.path());

        let transcriptions = vec![crate::transcription::traits::TranscriptionResult {
            text: "Transcribed text".to_string(),
            language: Some("es".into()),
            duration_secs: Some(5.0),
            confidence: None,
            processing_ms: None,
        }];

        let mut msg = make_audio_channel_message(vec![
            traits::ContentPart::Text {
                text: "Here is my voice note:".into(),
            },
            traits::ContentPart::Audio {
                channel_handle: "file999".into(),
                source_channel: "telegram".into(),
                declared_mime: Some("audio/ogg".into()),
                caption_text: None,
                file_name: None,
                declared_bytes: Some(64),
                declared_duration_secs: Some(5),
            },
        ]);

        inject_transcription(&mut msg, std::slice::from_ref(&staged), &transcriptions);

        assert_eq!(msg.parts.len(), 2, "should still have 2 parts");
        assert!(
            matches!(
                &msg.parts[0],
                traits::ContentPart::Text { text } if text.contains("voice note")
            ),
            "first part should remain unchanged"
        );
        assert!(
            matches!(
                &msg.parts[1],
                traits::ContentPart::Text { text } if text.contains("Transcribed text")
            ),
            "second part should be the injected transcription"
        );
    }

    // ── StagedAudioGuard with multiple files ─────────────────

    #[test]
    fn staged_audio_guard_drop_cleans_up_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = dir.path().join("audio1.ogg");
        let f2 = dir.path().join("audio2.ogg");
        std::fs::write(&f1, b"fake1").unwrap();
        std::fs::write(&f2, b"fake2").unwrap();
        assert!(f1.exists());
        assert!(f2.exists());

        let guard = StagedAudioGuard(vec![
            audio_media::StagedAudio {
                sha256: "aaa".into(),
                mime_type: audio_media::AllowedAudioMime::OggOpus,
                byte_len: 5,
                duration_secs: Some(1.0),
                temp_path: f1.clone(),
                channel_origin: "telegram".into(),
            },
            audio_media::StagedAudio {
                sha256: "bbb".into(),
                mime_type: audio_media::AllowedAudioMime::Mp3,
                byte_len: 5,
                duration_secs: Some(2.0),
                temp_path: f2.clone(),
                channel_origin: "telegram".into(),
            },
        ]);
        drop(guard);

        assert!(!f1.exists(), "first temp file should be removed");
        assert!(!f2.exists(), "second temp file should be removed");
    }

    // ── duration_f64_to_ms helper tests ──────────────────────

    #[test]
    fn duration_f64_to_ms_normal_values() {
        assert_eq!(duration_f64_to_ms(1.0), 1000);
        assert_eq!(duration_f64_to_ms(0.5), 500);
        assert_eq!(duration_f64_to_ms(5.123), 5123);
        assert_eq!(duration_f64_to_ms(0.0), 0);
    }

    #[test]
    fn duration_f64_to_ms_negative_clamped_to_zero() {
        assert_eq!(duration_f64_to_ms(-1.0), 0);
        assert_eq!(duration_f64_to_ms(-100.0), 0);
    }

    // ── emit_audio_ingress rejection event test ──────────────

    #[tokio::test]
    async fn audio_pipeline_rejection_event_emitted_with_reason() {
        let observer = Arc::new(AudioRecordingObserver::default());

        emit_audio_ingress(
            observer.as_ref(),
            "telegram",
            crate::observability::AudioIngressOutcome::Rejected,
            Some(&audio_media::AudioRejectionReason::Oversize),
            Some("audio/ogg".into()),
            Some(30_000_000),
            Some(120.0),
            None,
        );

        let events = observer.audio_events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].outcome,
            crate::observability::AudioIngressOutcome::Rejected
        );
        assert_eq!(
            events[0].reason,
            Some(crate::observability::AudioIngressReason::Oversize)
        );
        assert_eq!(events[0].byte_len, Some(30_000_000));
        assert_eq!(events[0].duration_secs, Some(120.0));
        assert!(events[0].transcription_duration_ms.is_none());
    }

    // ── build_transcriber tests ──────────────────────────────

    #[test]
    fn build_transcriber_returns_none_when_audio_disabled() {
        let config = Config::default(); // audio.enabled is false by default
        assert!(build_transcriber(&config).is_none());
    }

    #[test]
    fn build_transcriber_returns_some_when_audio_enabled() {
        let mut config = Config::default();
        config.audio.enabled = true;
        config.audio.allowed_channels = vec!["telegram".into()];
        let transcriber = build_transcriber(&config);
        assert!(
            transcriber.is_some(),
            "should return a transcriber when audio is enabled"
        );
        assert_eq!(transcriber.unwrap().name(), "whisper-cli");
    }

    // ── Audio pipeline runtime-context helper ────────────────

    fn make_audio_runtime_context(
        channel: Arc<dyn Channel>,
        transcriber: Option<Arc<dyn crate::transcription::traits::Transcriber>>,
        observer: Arc<dyn Observer>,
        config: Config,
    ) -> Arc<ChannelRuntimeContext> {
        let mut channels_by_name = HashMap::new();
        channels_by_name.insert(channel.name().to_string(), channel);
        Arc::new(ChannelRuntimeContext {
            config: Arc::new(config),
            channels_by_name: Arc::new(channels_by_name),
            provider: Arc::new(SlowProvider {
                delay: Duration::from_millis(1),
            }),
            memory: Arc::new(NoopMemory),
            tools_registry: Arc::new(vec![]),
            observer,
            system_prompt: Arc::new("test".into()),
            model: Arc::new("test".into()),
            temperature: 0.0,
            auto_save_memory: false,
            tool_dispatcher_mode: Arc::from("xml"),
            max_tool_iterations: 5,
            min_relevance_score: 0.0,
            conversation_histories: Arc::new(Mutex::new(HashMap::new())),
            transcriber,
        })
    }

    // ── gate_audio_config tests ──────────────────────────────

    #[tokio::test]
    async fn gate_audio_config_returns_ok_when_no_audio_parts() {
        let channel: Arc<dyn Channel> = Arc::new(RecordingChannel::default());
        let ctx = make_audio_runtime_context(
            channel.clone(),
            None,
            Arc::new(NoopObserver),
            Config::default(),
        );
        let msg = make_audio_channel_message(vec![traits::ContentPart::Text {
            text: "hello".into(),
        }]);
        let result = gate_audio_config(&ctx, &msg, "s1", Some(&channel)).await;
        assert!(result.is_ok(), "no audio parts should pass through");
    }

    #[tokio::test]
    async fn gate_audio_config_rejects_when_transcriber_unavailable() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let config = make_audio_test_config("test-channel");
        let ctx = make_audio_runtime_context(
            channel.clone(),
            None, // No transcriber
            Arc::new(AudioRecordingObserver::default()),
            config,
        );

        let msg = make_audio_channel_message(vec![traits::ContentPart::Audio {
            channel_handle: "file123".into(),
            source_channel: "telegram".into(),
            declared_mime: Some("audio/ogg".into()),
            caption_text: None,
            file_name: None,
            declared_bytes: Some(64),
            declared_duration_secs: Some(5),
        }]);

        let result = gate_audio_config(&ctx, &msg, "s-tx", Some(&channel)).await;
        assert!(result.is_err(), "should reject when transcriber is None");

        let sent = channel_impl.sent_messages.lock().await;
        assert!(!sent.is_empty(), "rejection message should be sent");
        assert!(
            sent[0].contains("not available"),
            "should mention transcriber unavailable, got: {}",
            sent[0]
        );
    }

    // ── gate_and_stage_audio tests ───────────────────────────

    #[tokio::test]
    async fn gate_and_stage_audio_returns_empty_guard_when_no_audio() {
        let channel: Arc<dyn Channel> = Arc::new(RecordingChannel::default());
        let transcriber: Arc<dyn crate::transcription::traits::Transcriber> =
            Arc::new(MockTranscriber::new("unused"));
        let ctx = make_audio_runtime_context(
            channel.clone(),
            Some(transcriber),
            Arc::new(NoopObserver),
            make_audio_test_config("test-channel"),
        );

        let msg = make_audio_channel_message(vec![traits::ContentPart::Text {
            text: "just text".into(),
        }]);

        let result = gate_and_stage_audio(&ctx, &msg, "s1", Some(&channel)).await;
        assert!(result.is_ok());
        assert!(
            result.unwrap().0.is_empty(),
            "guard should have no staged audio"
        );
    }

    #[tokio::test]
    async fn gate_and_stage_audio_rejects_multiple_audio_parts() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let transcriber: Arc<dyn crate::transcription::traits::Transcriber> =
            Arc::new(MockTranscriber::new("unused"));
        let ctx = make_audio_runtime_context(
            channel.clone(),
            Some(transcriber),
            Arc::new(AudioRecordingObserver::default()),
            make_audio_test_config("test-channel"),
        );

        let msg = make_audio_channel_message(vec![
            traits::ContentPart::Audio {
                channel_handle: "file1".into(),
                source_channel: "telegram".into(),
                declared_mime: Some("audio/ogg".into()),
                caption_text: None,
                file_name: None,
                declared_bytes: Some(64),
                declared_duration_secs: Some(5),
            },
            traits::ContentPart::Audio {
                channel_handle: "file2".into(),
                source_channel: "telegram".into(),
                declared_mime: Some("audio/ogg".into()),
                caption_text: None,
                file_name: None,
                declared_bytes: Some(64),
                declared_duration_secs: Some(3),
            },
        ]);

        let result = gate_and_stage_audio(&ctx, &msg, "s-multi", Some(&channel)).await;
        assert!(result.is_err(), "multiple audio parts should be rejected");

        let sent = channel_impl.sent_messages.lock().await;
        assert!(!sent.is_empty());
        assert!(
            sent[0].contains("one audio"),
            "should mention single audio limit, got: {}",
            sent[0]
        );
    }

    // ── transcribe_audio tests ───────────────────────────────

    /// Mock transcriber that always fails with a configurable reason.
    struct FailingMockTranscriber {
        reason: audio_media::AudioRejectionReason,
    }

    #[async_trait::async_trait]
    impl crate::transcription::traits::Transcriber for FailingMockTranscriber {
        fn name(&self) -> &str {
            "failing-mock-transcriber"
        }

        async fn transcribe(
            &self,
            _audio: &audio_media::StagedAudio,
        ) -> Result<
            crate::transcription::traits::TranscriptionResult,
            audio_media::AudioRejectionReason,
        > {
            Err(self.reason.clone())
        }

        async fn health_check(&self) -> Result<(), String> {
            Err("failing".into())
        }
    }

    #[tokio::test]
    async fn transcribe_audio_rejects_empty_transcription_text() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let transcriber: Arc<dyn crate::transcription::traits::Transcriber> =
            Arc::new(MockTranscriber::new("")); // Empty text
        let observer = Arc::new(AudioRecordingObserver::default());
        let ctx = make_audio_runtime_context(
            channel.clone(),
            Some(transcriber),
            observer.clone(),
            make_audio_test_config("test-channel"),
        );

        let tmp = tempfile::tempdir().unwrap();
        let staged = make_test_staged_audio(tmp.path());

        let msg = make_audio_channel_message(vec![traits::ContentPart::Audio {
            channel_handle: "file123".into(),
            source_channel: "telegram".into(),
            declared_mime: Some("audio/ogg".into()),
            caption_text: None,
            file_name: None,
            declared_bytes: Some(64),
            declared_duration_secs: Some(5),
        }]);

        let result = transcribe_audio(
            &ctx,
            std::slice::from_ref(&staged),
            "s-empty",
            Some(&channel),
            &msg,
        )
        .await;

        assert!(result.is_err(), "empty transcription should be rejected");

        let sent = channel_impl.sent_messages.lock().await;
        assert!(!sent.is_empty());
        assert!(
            sent[0].contains("No speech was detected"),
            "should mention no speech, got: {}",
            sent[0]
        );

        // Verify observability event
        let events = observer.audio_events.lock().unwrap();
        assert!(!events.is_empty());
        assert_eq!(
            events[0].reason,
            Some(crate::observability::AudioIngressReason::NoSpeechDetected)
        );
    }

    #[tokio::test]
    async fn transcribe_audio_rejects_on_transcriber_error() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let transcriber: Arc<dyn crate::transcription::traits::Transcriber> =
            Arc::new(FailingMockTranscriber {
                reason: audio_media::AudioRejectionReason::TranscriptionFailed,
            });
        let observer = Arc::new(AudioRecordingObserver::default());
        let ctx = make_audio_runtime_context(
            channel.clone(),
            Some(transcriber),
            observer.clone(),
            make_audio_test_config("test-channel"),
        );

        let tmp = tempfile::tempdir().unwrap();
        let staged = make_test_staged_audio(tmp.path());

        let msg = make_audio_channel_message(vec![traits::ContentPart::Audio {
            channel_handle: "file123".into(),
            source_channel: "telegram".into(),
            declared_mime: Some("audio/ogg".into()),
            caption_text: None,
            file_name: None,
            declared_bytes: Some(64),
            declared_duration_secs: Some(5),
        }]);

        let result = transcribe_audio(
            &ctx,
            std::slice::from_ref(&staged),
            "s-fail",
            Some(&channel),
            &msg,
        )
        .await;

        assert!(result.is_err(), "transcription error should be rejected");

        let sent = channel_impl.sent_messages.lock().await;
        assert!(!sent.is_empty());
        assert!(
            sent[0].contains("transcription failed"),
            "should mention transcription failure, got: {}",
            sent[0]
        );

        let events = observer.audio_events.lock().unwrap();
        assert!(!events.is_empty());
        assert_eq!(
            events[0].reason,
            Some(crate::observability::AudioIngressReason::TranscriptionFailed)
        );
    }

    #[tokio::test]
    async fn transcribe_audio_rejects_when_no_transcriber() {
        let channel_impl = Arc::new(RecordingChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let ctx = make_audio_runtime_context(
            channel.clone(),
            None, // No transcriber
            Arc::new(AudioRecordingObserver::default()),
            make_audio_test_config("test-channel"),
        );

        let tmp = tempfile::tempdir().unwrap();
        let staged = make_test_staged_audio(tmp.path());

        let msg = make_audio_channel_message(vec![traits::ContentPart::Audio {
            channel_handle: "file123".into(),
            source_channel: "telegram".into(),
            declared_mime: Some("audio/ogg".into()),
            caption_text: None,
            file_name: None,
            declared_bytes: Some(64),
            declared_duration_secs: Some(5),
        }]);

        let result = transcribe_audio(
            &ctx,
            std::slice::from_ref(&staged),
            "s-notx",
            Some(&channel),
            &msg,
        )
        .await;

        assert!(result.is_err(), "should reject when no transcriber");

        let sent = channel_impl.sent_messages.lock().await;
        assert!(!sent.is_empty());
        assert!(sent[0].contains("not available"));
    }

    // ── audio_rejection_user_text — TooLong duration variants ──

    #[test]
    fn audio_rejection_user_text_too_long_sub_minute() {
        let mut config = Config::default();
        config.audio.max_audio_duration_secs = 45;
        let text = audio_rejection_user_text(
            "s-sub",
            &audio_media::AudioRejectionReason::TooLong,
            &config,
        );
        assert!(
            text.contains("45 seconds"),
            "expected '45 seconds', got: {text}"
        );
    }

    #[test]
    fn audio_rejection_user_text_too_long_mixed_minutes_seconds() {
        let mut config = Config::default();
        config.audio.max_audio_duration_secs = 90; // 1 min 30 sec
        let text = audio_rejection_user_text(
            "s-mix",
            &audio_media::AudioRejectionReason::TooLong,
            &config,
        );
        assert!(
            text.contains("1 minute") && text.contains("30 seconds"),
            "expected '1 minute ... 30 seconds', got: {text}"
        );
    }

    #[test]
    fn audio_rejection_user_text_too_long_exact_minutes() {
        let mut config = Config::default();
        config.audio.max_audio_duration_secs = 600; // 10 min exactly
        let text = audio_rejection_user_text(
            "s-exact",
            &audio_media::AudioRejectionReason::TooLong,
            &config,
        );
        assert!(
            text.contains("10 minutes"),
            "expected '10 minutes', got: {text}"
        );
        assert!(
            !text.contains("seconds"),
            "exact minutes should not mention seconds, got: {text}"
        );
    }

    #[test]
    fn audio_rejection_user_text_multiple_audio_parts_variant() {
        let config = Config::default();
        let text = audio_rejection_user_text(
            "s-multi",
            &audio_media::AudioRejectionReason::MultipleAudioParts,
            &config,
        );
        assert!(text.contains("one audio"), "got: {text}");
    }
}
