pub(crate) mod discovery;
pub(crate) mod index;
pub(crate) mod sqlite;
pub(crate) mod trigram;

// Public API re-exports
pub use discovery::{DiscoveredFile, DiscoveryRules};
pub use index::{LoadedIndex, RefreshAction, WorkspaceTrigramIndex};

#[cfg(test)]
mod tests;
