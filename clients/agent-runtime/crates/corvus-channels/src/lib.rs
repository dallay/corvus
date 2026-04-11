//! Corvus Channels Registry
//!
//! Re-exports channel types and provides registry functions.

pub use corvus_traits::channels::{Channel, ChannelMessage, ContentPart, SendMessage};

/// Information about a channel.
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub name: &'static str,
    pub display_name: &'static str,
}
