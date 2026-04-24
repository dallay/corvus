use bytes::Bytes;
use futures_util::stream;
use futures_util::{Stream, StreamExt};
use std::collections::VecDeque;
use std::convert::Infallible;

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
    stream::unfold(
        (
            OpenAiSseParser::default(),
            upstream,
            VecDeque::<String>::new(),
            false,
        ),
        |(mut parser, mut upstream, mut pending, terminated)| async move {
            if terminated {
                return None;
            }

            loop {
                if let Some(payload) = pending.pop_front() {
                    let is_done = payload == STREAM_DONE_SENTINEL;
                    let event = axum::response::sse::Event::default().data(payload);
                    return Some((Ok(event), (parser, upstream, pending, is_done)));
                }

                match upstream.next().await {
                    Some(Ok(chunk)) => match parser.push(&chunk) {
                        Ok(payloads) => {
                            pending.extend(payloads);
                        }
                        Err(_) => return None,
                    },
                    Some(Err(_)) | None => return None,
                }
            }
        },
    )
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
