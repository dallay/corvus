use crate::conductor::ConductorEventEnvelope;
use std::sync::LazyLock;
use tokio::sync::broadcast;

static EVENT_BUS: LazyLock<broadcast::Sender<String>> = LazyLock::new(|| {
    let (sender, _receiver) = broadcast::channel(512);
    sender
});

pub fn publish(event: &ConductorEventEnvelope) {
    if let Ok(serialized) = serde_json::to_string(event) {
        let _ = EVENT_BUS.send(serialized);
    }
}

pub fn subscribe() -> broadcast::Receiver<String> {
    EVENT_BUS.subscribe()
}
