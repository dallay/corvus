use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum ToolCallEventKind {
    Started,
    Finished,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ToolCallEvent {
    pub kind: ToolCallEventKind,
    pub request_id: String,
    pub tool_name: String,
    pub timestamp: String,
    pub duration_ms: Option<u64>,
    pub status: Option<String>,
    pub redacted_args: Option<serde_json::Value>,
    pub redacted_output: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<ToolCallEvent>,
    dropped: Arc<AtomicU64>,
}

impl EventBus {
    pub fn new(buffer: usize) -> Self {
        let buffer = buffer.max(1);
        let (sender, _) = broadcast::channel(buffer);
        Self {
            sender,
            dropped: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn publish(&self, event: ToolCallEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> EventStream {
        EventStream {
            receiver: self.sender.subscribe(),
            dropped: self.dropped.clone(),
        }
    }

    pub fn drop_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

pub struct EventStream {
    receiver: broadcast::Receiver<ToolCallEvent>,
    dropped: Arc<AtomicU64>,
}

impl EventStream {
    pub async fn recv(&mut self) -> Option<ToolCallEvent> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => return Some(event),
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    self.dropped.fetch_add(count as u64, Ordering::Relaxed);
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }

    pub fn try_recv(&mut self) -> Option<ToolCallEvent> {
        match self.receiver.try_recv() {
            Ok(event) => Some(event),
            Err(broadcast::error::TryRecvError::Lagged(count)) => {
                self.dropped.fetch_add(count as u64, Ordering::Relaxed);
                None
            }
            Err(broadcast::error::TryRecvError::Empty) => None,
            Err(broadcast::error::TryRecvError::Closed) => None,
        }
    }

    pub fn drop_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}
