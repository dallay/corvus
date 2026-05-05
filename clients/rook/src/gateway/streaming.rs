use bytes::Bytes;
use futures_util::stream;
use futures_util::{Stream, StreamExt};
use std::collections::VecDeque;
use std::convert::Infallible;
use std::future::Future;

use crate::gateway::types::STREAM_DONE_SENTINEL;

#[derive(Debug, PartialEq, Eq)]
pub enum OpenAiSseParseError {
    MalformedFrame { frame: String },
    DuplicateDoneSentinel,
}

#[derive(Default)]
pub struct OpenAiSseParser {
    buffer: Vec<u8>,
    seen_done: bool,
}

impl OpenAiSseParser {
    pub fn push(&mut self, chunk: &Bytes) -> Result<Vec<String>, OpenAiSseParseError> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();

        while let Some(boundary) = self.buffer.windows(2).position(|window| window == b"\n\n") {
            let frame = self.buffer.drain(..boundary + 2).collect::<Vec<u8>>();
            let frame = String::from_utf8_lossy(&frame[..frame.len() - 2]).to_string();
            if frame.trim().is_empty() {
                continue;
            }

            let payload = parse_frame(&frame)?;
            if payload == STREAM_DONE_SENTINEL {
                if self.seen_done {
                    return Err(OpenAiSseParseError::DuplicateDoneSentinel);
                }
                self.seen_done = true;
            }
            events.push(payload);
        }

        Ok(events)
    }
}

fn parse_frame(frame: &str) -> Result<String, OpenAiSseParseError> {
    let mut data_lines = Vec::new();

    for line in frame.lines() {
        if let Some(payload) = line.strip_prefix("data: ") {
            data_lines.push(payload.to_string());
        } else {
            return Err(OpenAiSseParseError::MalformedFrame {
                frame: frame.to_string(),
            });
        }
    }

    if data_lines.is_empty() {
        return Err(OpenAiSseParseError::MalformedFrame {
            frame: frame.to_string(),
        });
    }

    Ok(data_lines.join("\n"))
}

pub fn normalize_openai_sse_bytes(body: &[u8]) -> (String, bool) {
    let text = String::from_utf8_lossy(body);
    let mut rendered = String::new();
    let mut seen_done = false;

    for frame in text.split("\n\n") {
        let trimmed = frame.trim();
        if trimmed.is_empty() {
            continue;
        }

        let payload = match parse_frame(trimmed) {
            Ok(payload) => payload,
            Err(_) => return (rendered, false),
        };

        if payload == STREAM_DONE_SENTINEL {
            if seen_done {
                return (rendered, false);
            }
            seen_done = true;
            rendered.push_str("data: ");
            rendered.push_str(STREAM_DONE_SENTINEL);
            rendered.push_str("\n\n");
            break;
        }

        rendered.push_str("data: ");
        rendered.push_str(&payload);
        rendered.push_str("\n\n");
    }

    (rendered, seen_done)
}

pub fn text_event_stream(
    body: String,
) -> impl Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>> {
    let events = body
        .split("\n\n")
        .filter_map(|frame| {
            let payload = frame.strip_prefix("data: ")?;
            Some(Ok(axum::response::sse::Event::default().data(payload)))
        })
        .collect::<Vec<_>>();
    stream::iter(events)
}

pub fn upstream_event_stream<S, E>(
    upstream: S,
) -> impl Stream<Item = Result<axum::response::sse::Event, Infallible>>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
{
    upstream_event_stream_with_completion(upstream, || async {})
}

pub fn upstream_event_stream_with_completion<S, E, F, Fut>(
    upstream: S,
    on_complete: F,
) -> impl Stream<Item = Result<axum::response::sse::Event, Infallible>>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    F: FnOnce() -> Fut + Unpin,
    Fut: Future<Output = ()>,
{
    let state = UpstreamEventStreamState::new(upstream, on_complete);
    stream::unfold(state, next_upstream_event)
}

struct UpstreamEventStreamState<S, F> {
    parser: OpenAiSseParser,
    upstream: S,
    pending: VecDeque<String>,
    terminated: bool,
    on_complete: Option<F>,
}

impl<S, F> UpstreamEventStreamState<S, F> {
    fn new(upstream: S, on_complete: F) -> Self {
        Self {
            parser: OpenAiSseParser::default(),
            upstream,
            pending: VecDeque::new(),
            terminated: false,
            on_complete: Some(on_complete),
        }
    }
}

async fn next_upstream_event<S, E, F, Fut>(
    mut state: UpstreamEventStreamState<S, F>,
) -> Option<(
    Result<axum::response::sse::Event, Infallible>,
    UpstreamEventStreamState<S, F>,
)>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    F: FnOnce() -> Fut + Unpin,
    Fut: Future<Output = ()>,
{
    if state.terminated {
        run_completion_once(state.on_complete.take()).await;
        return None;
    }

    next_pending_or_upstream_event(&mut state)
        .await
        .map(|event| (Ok(event), state))
}

async fn next_pending_or_upstream_event<S, E, F>(
    state: &mut UpstreamEventStreamState<S, F>,
) -> Option<axum::response::sse::Event>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
{
    loop {
        if let Some(next) = pop_pending_event(&mut state.pending) {
            let (event, is_done) = next;
            state.terminated = is_done;
            return Some(event);
        }

        if !extend_pending_from_upstream(&mut state.parser, &mut state.upstream, &mut state.pending)
            .await
        {
            return None;
        }
    }
}

async fn run_completion_once<F, Fut>(on_complete: Option<F>)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    if let Some(on_complete) = on_complete {
        on_complete().await;
    }
}

fn pop_pending_event(pending: &mut VecDeque<String>) -> Option<(axum::response::sse::Event, bool)> {
    let payload = pending.pop_front()?;
    let is_done = payload == STREAM_DONE_SENTINEL;
    let event = axum::response::sse::Event::default().data(payload);
    Some((event, is_done))
}

async fn extend_pending_from_upstream<S, E>(
    parser: &mut OpenAiSseParser,
    upstream: &mut S,
    pending: &mut VecDeque<String>,
) -> bool
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
{
    match upstream.next().await {
        Some(Ok(chunk)) => match parser.push(&chunk) {
            Ok(payloads) => {
                pending.extend(payloads);
                true
            }
            Err(_) => false,
        },
        Some(Err(_)) | None => false,
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::{OpenAiSseParseError, OpenAiSseParser};

    #[test]
    fn parser_reconstructs_ordered_events_across_split_boundaries() {
        let mut parser = OpenAiSseParser::default();

        let first = parser
            .push(&Bytes::from_static(b"data: {\"id\":\"chunk-1\"}"))
            .unwrap();
        assert!(first.is_empty());

        let second = parser
            .push(&Bytes::from_static(
                b"\n\ndata: {\"id\":\"chunk-2\"}\n\ndata: [DONE]\n\n",
            ))
            .unwrap();

        assert_eq!(
            second,
            vec![
                "{\"id\":\"chunk-1\"}".to_string(),
                "{\"id\":\"chunk-2\"}".to_string(),
                "[DONE]".to_string(),
            ]
        );
    }

    #[test]
    fn parser_rejects_malformed_frames() {
        let mut parser = OpenAiSseParser::default();

        let error = parser
            .push(&Bytes::from_static(b"event: message\n\n"))
            .unwrap_err();

        assert!(matches!(error, OpenAiSseParseError::MalformedFrame { .. }));
    }

    #[test]
    fn parser_allows_done_once_and_rejects_duplicates() {
        let mut parser = OpenAiSseParser::default();

        let first = parser
            .push(&Bytes::from_static(b"data: [DONE]\n\n"))
            .unwrap();
        assert_eq!(first, vec!["[DONE]".to_string()]);

        let error = parser
            .push(&Bytes::from_static(b"data: [DONE]\n\n"))
            .unwrap_err();

        assert!(matches!(error, OpenAiSseParseError::DuplicateDoneSentinel));
    }
}
