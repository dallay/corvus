use crate::channels::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use futures_util::future;
use std::sync::Arc;
use tokio::time::Duration;

struct PendingChannel;

#[async_trait]
impl Channel for PendingChannel {
    fn name(&self) -> &str {
        "test-pending"
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        future::pending::<()>().await;
        Ok(())
    }
}

#[tokio::test]
async fn long_running_listener_refreshes_health_without_restart() {
    let channel = Arc::new(PendingChannel);
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let component = format!("channel:{}", channel.name());

    assert!(channel.health_check().await);
    assert!(channel
        .send(&SendMessage::new("ping", "tester"))
        .await
        .is_ok());

    crate::health::clear_component(&component);
    crate::health::mark_component_ok(&component);

    let mut health_interval =
        tokio::time::interval(Duration::from_secs(super::CHANNEL_HEALTH_TICK_SECS));
    let mut listen_task = Box::pin(channel.listen(tx));

    let mut last_ok_values = Vec::new();
    for _ in 0..3 {
        tokio::time::sleep(Duration::from_secs(super::CHANNEL_HEALTH_TICK_SECS)).await;
        tokio::select! {
            res = &mut listen_task => {
                panic!("listener resolved unexpectedly: {res:?}");
            }
            _ = health_interval.tick() => {
                crate::health::mark_component_ok(&component);
                let snapshot = crate::health::snapshot();
                let entry = snapshot
                    .components
                    .get(&component)
                    .expect("component should be present");
                last_ok_values.push(entry.last_ok.clone());
            }
        }
    }

    assert_eq!(last_ok_values.len(), 3);
    assert!(last_ok_values.iter().all(|value| value.is_some()));
    assert!(
        last_ok_values.windows(2).all(|pair| pair[0] != pair[1]),
        "last_ok should refresh while listener is pending"
    );
}
