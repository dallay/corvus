use std::any::TypeId;

use async_trait::async_trait;
use corvus::channels::{self, Channel as RuntimeChannel, SendMessage};
use corvus::memory::{self, Memory as RuntimeMemory, MemoryCategory};
use corvus::security::{self, Sandbox as RuntimeSandbox};

struct DummySandbox;

impl corvus_traits::security::Sandbox for DummySandbox {
    fn wrap_command(&self, _cmd: &mut std::process::Command) -> std::io::Result<()> {
        Ok(())
    }

    fn is_available(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "dummy"
    }

    fn description(&self) -> &str {
        "dummy sandbox"
    }
}

struct DummyChannel;

#[async_trait]
impl corvus_traits::channels::Channel for DummyChannel {
    fn name(&self) -> &str {
        "dummy"
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<corvus_traits::channels::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

struct DummyMemory;

#[async_trait]
impl corvus_traits::memory::Memory for DummyMemory {
    fn name(&self) -> &str {
        "dummy"
    }

    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<corvus_traits::memory::MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn get(&self, _key: &str) -> anyhow::Result<Option<corvus_traits::memory::MemoryEntry>> {
        Ok(None)
    }

    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<corvus_traits::memory::MemoryEntry>> {
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

#[test]
fn legacy_paths_match_extracted_trait_identities() {
    assert_eq!(
        TypeId::of::<&dyn RuntimeSandbox>(),
        TypeId::of::<&dyn corvus_traits::security::Sandbox>()
    );
    assert_eq!(
        TypeId::of::<&dyn security::traits::Sandbox>(),
        TypeId::of::<&dyn corvus_traits::security::Sandbox>()
    );

    assert_eq!(
        TypeId::of::<&dyn RuntimeChannel>(),
        TypeId::of::<&dyn corvus_traits::channels::Channel>()
    );
    assert_eq!(
        TypeId::of::<&dyn channels::traits::Channel>(),
        TypeId::of::<&dyn corvus_traits::channels::Channel>()
    );

    assert_eq!(
        TypeId::of::<&dyn RuntimeMemory>(),
        TypeId::of::<&dyn corvus_traits::memory::Memory>()
    );
    assert_eq!(
        TypeId::of::<&dyn memory::traits::Memory>(),
        TypeId::of::<&dyn corvus_traits::memory::Memory>()
    );
}

#[test]
fn legacy_paths_accept_trait_objects_from_extracted_crate() {
    let sandbox = DummySandbox;
    let sandbox_ref: &dyn RuntimeSandbox = &sandbox;
    let sandbox_traits_ref: &dyn security::traits::Sandbox = sandbox_ref;
    let sandbox_new_ref: &dyn corvus_traits::security::Sandbox = sandbox_traits_ref;
    assert_eq!(sandbox_new_ref.name(), "dummy");

    let channel = DummyChannel;
    let channel_ref: &dyn RuntimeChannel = &channel;
    let channel_traits_ref: &dyn channels::traits::Channel = channel_ref;
    let channel_new_ref: &dyn corvus_traits::channels::Channel = channel_traits_ref;
    assert_eq!(channel_new_ref.name(), "dummy");

    let memory = DummyMemory;
    let memory_ref: &dyn RuntimeMemory = &memory;
    let memory_traits_ref: &dyn memory::traits::Memory = memory_ref;
    let memory_new_ref: &dyn corvus_traits::memory::Memory = memory_traits_ref;
    assert_eq!(memory_new_ref.name(), "dummy");
}
