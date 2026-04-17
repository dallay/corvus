//! Corvus channel registry surfaces for manifest composition.

pub mod factory;
pub mod registry;

pub use corvus_traits::channels::{Channel, ChannelMessage, ContentPart, SendMessage};
pub use factory::{select_channel, ChannelFactorySelection};
pub use registry::{
    channel_availability, list_channels, resolve_channel_key, CapabilityAvailability,
    ChannelDescriptor,
};
