use crate::conductor::TaskId;
use anyhow::Result;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WorkspaceManager {
    workspace_root: PathBuf,
}

impl WorkspaceManager {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    pub fn create_workspace(&self, task_id: &TaskId, hint: Option<&str>) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.workspace_root)?;

        let leaf = hint
            .map(sanitize_workspace_leaf)
            .transpose()?
            .unwrap_or_else(|| task_id.as_str().to_string());

        let workspace = self.workspace_root.join(leaf);
        if !is_within_root(&self.workspace_root, &workspace) {
            anyhow::bail!("workspace path escapes workspace root");
        }

        std::fs::create_dir_all(&workspace)?;
        Ok(workspace)
    }

    pub fn is_within_workspace(&self, task_workspace: &Path, candidate: &Path) -> bool {
        is_within_root(task_workspace, candidate)
    }
}

pub fn sanitize_workspace_leaf(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        anyhow::bail!("workspace hint must not be empty");
    }

    let mut sanitized = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }

    if sanitized.contains("..") {
        anyhow::bail!("workspace hint must not include traversal segments");
    }
    if sanitized.len() > 128 {
        anyhow::bail!("workspace hint exceeds max length of 128");
    }
    if sanitized.is_empty() {
        anyhow::bail!("workspace hint sanitized to empty value");
    }

    Ok(sanitized)
}

fn is_within_root(root: &Path, candidate: &Path) -> bool {
    let mut normalized = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }

    normalized.starts_with(root)
}
