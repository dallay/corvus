//! Lockfile management for installed skills.
//! The lockfile (`skills.lock`) is advisory — corrupt or missing files
//! never block skill loading. See AD2 in the design document.

use super::trust::{SkillOrigin, SkillSource, SkillTrust};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

const LOCKFILE_NAME: &str = "skills.lock";

/// Top-level lockfile structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillsLockfile {
    #[serde(default)]
    pub skills: BTreeMap<String, LockEntry>,
}

/// Per-skill lock entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEntry {
    pub trust: String,
    pub source: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default, rename = "ref")]
    pub pinned_ref: Option<String>,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub installed_at: Option<String>,
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
}

/// Read the lockfile from the workspace directory.
/// Returns an empty lockfile on missing/corrupt file (advisory model).
pub fn read_lockfile(workspace_dir: &Path) -> SkillsLockfile {
    let path = workspace_dir.join(LOCKFILE_NAME);
    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<SkillsLockfile>(&content) {
            Ok(lockfile) => lockfile,
            Err(err) => {
                tracing::warn!("corrupt skills lockfile at {}: {err}", path.display(),);
                SkillsLockfile::default()
            }
        },
        Err(_) => SkillsLockfile::default(),
    }
}

/// Write or update a single lock entry.
/// Reads existing lockfile, merges, writes back as pretty TOML.
pub fn write_lock_entry(workspace_dir: &Path, name: &str, entry: LockEntry) -> Result<()> {
    let path = workspace_dir.join(LOCKFILE_NAME);
    let mut lockfile = read_lockfile(workspace_dir);
    lockfile.skills.insert(name.to_string(), entry);
    let content = toml::to_string_pretty(&lockfile)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Remove a lock entry (on skill uninstall).
pub fn remove_lock_entry(workspace_dir: &Path, name: &str) -> Result<()> {
    let path = workspace_dir.join(LOCKFILE_NAME);
    let mut lockfile = read_lockfile(workspace_dir);
    lockfile.skills.remove(name);
    let content = toml::to_string_pretty(&lockfile)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Compute SHA-256 hash of content, returning `"sha256:<64-char-hex>"`.
pub fn compute_content_hash(content: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(content);
    format!("sha256:{}", hex::encode(hash))
}

/// Build a `LockEntry` from install-time metadata.
pub fn build_lock_entry(
    trust: SkillTrust,
    source: &str,
    pinned_ref: Option<String>,
    content_hash: Option<String>,
    allowed_tools: Option<Vec<String>>,
    path: Option<String>,
) -> LockEntry {
    LockEntry {
        trust: trust.as_str().to_string(),
        source: source.to_string(),
        path,
        pinned_ref,
        content_hash,
        installed_at: Some(chrono::Utc::now().to_rfc3339()),
        allowed_tools,
    }
}

/// Convert a `LockEntry` back to a `SkillOrigin` for loading.
pub fn lock_entry_to_origin(entry: &LockEntry) -> SkillOrigin {
    let source = if entry.source.starts_with("official:") {
        let repo = entry.source.strip_prefix("official:").unwrap().to_string();
        SkillSource::Official {
            repo,
            path: entry.path.clone().unwrap_or_default(),
        }
    } else if entry.source == "local" {
        SkillSource::Local
    } else if entry.source.starts_with("https://") || entry.source.starts_with("http://") {
        SkillSource::GitRepo {
            url: entry.source.clone(),
        }
    } else {
        SkillSource::Local // safe default
    };
    SkillOrigin {
        source,
        installed_at: entry.installed_at.clone(),
        pinned_ref: entry.pinned_ref.clone(),
        content_hash: entry.content_hash.clone(),
    }
}

#[derive(Debug, Default)]
pub struct RepairSummary {
    pub added: u32,
    pub removed: u32,
    pub updated: u32,
    pub unchanged: u32,
}

pub fn repair_lockfile(workspace_dir: &Path) -> Result<RepairSummary> {
    let skills_path = workspace_dir.join("skills");
    let mut current = read_lockfile(workspace_dir);
    let mut summary = RepairSummary::default();

    // Collect skills on disk
    let mut on_disk: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    if skills_path.exists() {
        for entry in std::fs::read_dir(&skills_path)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let skill_dir = entry.path();
            let skill_md = skill_dir.join("SKILL.md");
            let skill_toml = skill_dir.join("SKILL.toml");

            if !skill_md.exists() && !skill_toml.exists() {
                continue; // Not a valid skill directory
            }

            on_disk.insert(name.clone());

            // Compute current hash
            let current_hash = if skill_md.exists() {
                let content = std::fs::read(&skill_md).ok();
                content.map(|c| compute_content_hash(&c))
            } else {
                None
            };

            if let Some(existing) = current.skills.get_mut(&name) {
                // Check if hash matches
                if existing.content_hash == current_hash {
                    summary.unchanged += 1;
                } else {
                    existing.content_hash = current_hash;
                    existing.installed_at = Some(chrono::Utc::now().to_rfc3339());
                    summary.updated += 1;
                }
            } else {
                // New entry — default to Local trust
                let entry = LockEntry {
                    trust: "local".to_string(),
                    source: "local".to_string(),
                    path: None,
                    pinned_ref: None,
                    content_hash: current_hash,
                    installed_at: Some(chrono::Utc::now().to_rfc3339()),
                    allowed_tools: None,
                };
                current.skills.insert(name, entry);
                summary.added += 1;
            }
        }
    }

    // Remove orphaned entries
    let orphaned: Vec<String> = current
        .skills
        .keys()
        .filter(|k| !on_disk.contains(k.as_str()))
        .cloned()
        .collect();
    for name in &orphaned {
        current.skills.remove(name);
        summary.removed += 1;
    }

    // Write repaired lockfile
    let path = workspace_dir.join(LOCKFILE_NAME);
    let content = toml::to_string_pretty(&current)?;
    std::fs::write(&path, content)?;

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialization_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let entry = LockEntry {
            trust: "third-party".to_string(),
            source: "https://github.com/someone/skill.git".to_string(),
            path: None,
            pinned_ref: Some("abc123".to_string()),
            content_hash: Some("sha256:deadbeef".to_string()),
            installed_at: Some("2026-01-01T00:00:00Z".to_string()),
            allowed_tools: Some(vec!["Read".to_string(), "Grep".to_string()]),
        };

        write_lock_entry(dir.path(), "test-skill", entry.clone()).unwrap();
        let lockfile = read_lockfile(dir.path());

        let loaded = lockfile.skills.get("test-skill").unwrap();
        assert_eq!(loaded.trust, "third-party");
        assert_eq!(loaded.source, "https://github.com/someone/skill.git");
        assert_eq!(loaded.pinned_ref.as_deref(), Some("abc123"));
        assert_eq!(loaded.content_hash.as_deref(), Some("sha256:deadbeef"));
        assert_eq!(loaded.installed_at.as_deref(), Some("2026-01-01T00:00:00Z"));
        assert_eq!(loaded.allowed_tools.as_ref().unwrap(), &["Read", "Grep"],);
    }

    #[test]
    fn read_lockfile_missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let lockfile = read_lockfile(dir.path());
        assert!(lockfile.skills.is_empty());
    }

    #[test]
    fn read_lockfile_corrupt_toml_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("skills.lock"),
            "this is {{{{ not valid toml !!!!",
        )
        .unwrap();
        let lockfile = read_lockfile(dir.path());
        assert!(lockfile.skills.is_empty());
    }

    #[test]
    fn write_lock_entry_creates_and_updates() {
        let dir = tempfile::tempdir().unwrap();

        let entry1 = LockEntry {
            trust: "local".to_string(),
            source: "local".to_string(),
            path: None,
            pinned_ref: None,
            content_hash: None,
            installed_at: None,
            allowed_tools: None,
        };
        write_lock_entry(dir.path(), "skill-a", entry1).unwrap();

        let entry2 = LockEntry {
            trust: "third-party".to_string(),
            source: "https://example.com/skill.git".to_string(),
            path: None,
            pinned_ref: None,
            content_hash: None,
            installed_at: None,
            allowed_tools: None,
        };
        write_lock_entry(dir.path(), "skill-b", entry2).unwrap();

        let lockfile = read_lockfile(dir.path());
        assert_eq!(lockfile.skills.len(), 2);
        assert_eq!(lockfile.skills["skill-a"].trust, "local");
        assert_eq!(lockfile.skills["skill-b"].trust, "third-party");

        // Update existing entry
        let updated = LockEntry {
            trust: "local".to_string(),
            source: "local".to_string(),
            path: None,
            pinned_ref: Some("newref".to_string()),
            content_hash: None,
            installed_at: None,
            allowed_tools: None,
        };
        write_lock_entry(dir.path(), "skill-a", updated).unwrap();

        let lockfile = read_lockfile(dir.path());
        assert_eq!(lockfile.skills.len(), 2);
        assert_eq!(
            lockfile.skills["skill-a"].pinned_ref.as_deref(),
            Some("newref"),
        );
    }

    #[test]
    fn remove_lock_entry_removes_correct_entry() {
        let dir = tempfile::tempdir().unwrap();

        let entry = LockEntry {
            trust: "local".to_string(),
            source: "local".to_string(),
            path: None,
            pinned_ref: None,
            content_hash: None,
            installed_at: None,
            allowed_tools: None,
        };
        write_lock_entry(dir.path(), "keep-me", entry.clone()).unwrap();
        write_lock_entry(dir.path(), "remove-me", entry).unwrap();

        let lockfile = read_lockfile(dir.path());
        assert_eq!(lockfile.skills.len(), 2);

        remove_lock_entry(dir.path(), "remove-me").unwrap();

        let lockfile = read_lockfile(dir.path());
        assert_eq!(lockfile.skills.len(), 1);
        assert!(lockfile.skills.contains_key("keep-me"));
        assert!(!lockfile.skills.contains_key("remove-me"));
    }

    #[test]
    fn compute_content_hash_correct_sha256() {
        let hash = compute_content_hash(b"hello world");
        assert!(hash.starts_with("sha256:"));
        // SHA-256 of "hello world" is a known value
        assert_eq!(
            hash,
            "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
        );
        // Verify format: "sha256:" + 64 hex chars
        let hex_part = hash.strip_prefix("sha256:").unwrap();
        assert_eq!(hex_part.len(), 64);
    }

    #[test]
    fn lock_entry_to_origin_local() {
        let entry = LockEntry {
            trust: "local".to_string(),
            source: "local".to_string(),
            path: None,
            pinned_ref: None,
            content_hash: Some("sha256:abc123".to_string()),
            installed_at: Some("2026-01-01T00:00:00Z".to_string()),
            allowed_tools: None,
        };
        let origin = lock_entry_to_origin(&entry);
        assert!(matches!(origin.source, SkillSource::Local));
        assert_eq!(origin.installed_at.as_deref(), Some("2026-01-01T00:00:00Z"),);
        assert_eq!(origin.content_hash.as_deref(), Some("sha256:abc123"));
    }

    #[test]
    fn lock_entry_to_origin_git_repo() {
        let entry = LockEntry {
            trust: "third-party".to_string(),
            source: "https://github.com/someone/skill.git".to_string(),
            path: None,
            pinned_ref: Some("deadbeef".to_string()),
            content_hash: None,
            installed_at: None,
            allowed_tools: None,
        };
        let origin = lock_entry_to_origin(&entry);
        match &origin.source {
            SkillSource::GitRepo { url } => {
                assert_eq!(url, "https://github.com/someone/skill.git");
            }
            other => panic!("expected GitRepo, got {other:?}"),
        }
        assert_eq!(origin.pinned_ref.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn lock_entry_to_origin_official() {
        let entry = LockEntry {
            trust: "official".to_string(),
            source: "official:dallay/corvus-skills".to_string(),
            path: Some("skills/git-expert".to_string()),
            pinned_ref: Some("abc123".to_string()),
            content_hash: Some("sha256:cafe".to_string()),
            installed_at: Some("2026-03-01T00:00:00Z".to_string()),
            allowed_tools: None,
        };
        let origin = lock_entry_to_origin(&entry);
        match &origin.source {
            SkillSource::Official { repo, path } => {
                assert_eq!(repo, "dallay/corvus-skills");
                assert_eq!(path, "skills/git-expert");
            }
            other => panic!("expected Official, got {other:?}"),
        }
        assert_eq!(origin.pinned_ref.as_deref(), Some("abc123"));
        assert_eq!(origin.content_hash.as_deref(), Some("sha256:cafe"));
    }

    #[test]
    fn lock_entry_to_origin_official_without_path_defaults_empty() {
        let entry = LockEntry {
            trust: "official".to_string(),
            source: "official:dallay/corvus-skills".to_string(),
            path: None,
            pinned_ref: None,
            content_hash: None,
            installed_at: None,
            allowed_tools: None,
        };
        let origin = lock_entry_to_origin(&entry);
        match &origin.source {
            SkillSource::Official { repo, path } => {
                assert_eq!(repo, "dallay/corvus-skills");
                assert_eq!(path, "");
            }
            other => panic!("expected Official, got {other:?}"),
        }
    }

    #[test]
    fn lock_entry_to_origin_unknown_source_defaults_to_local() {
        let entry = LockEntry {
            trust: "local".to_string(),
            source: "something-unexpected".to_string(),
            path: None,
            pinned_ref: None,
            content_hash: None,
            installed_at: None,
            allowed_tools: None,
        };
        let origin = lock_entry_to_origin(&entry);
        assert!(matches!(origin.source, SkillSource::Local));
    }

    #[test]
    fn build_lock_entry_with_official_source_and_path() {
        let entry = build_lock_entry(
            SkillTrust::Official,
            "official:dallay/corvus-skills",
            Some("deadbeef".to_string()),
            Some("sha256:abc123".to_string()),
            None,
            Some("skills/git-expert".to_string()),
        );
        assert_eq!(entry.trust, "official");
        assert_eq!(entry.source, "official:dallay/corvus-skills");
        assert_eq!(entry.path.as_deref(), Some("skills/git-expert"));
        assert_eq!(entry.pinned_ref.as_deref(), Some("deadbeef"));
        assert_eq!(entry.content_hash.as_deref(), Some("sha256:abc123"));
        assert!(entry.installed_at.is_some());
    }

    #[test]
    fn build_lock_entry_populates_all_fields() {
        let entry = build_lock_entry(
            SkillTrust::ThirdParty,
            "https://github.com/someone/skill.git",
            Some("abc123".to_string()),
            Some("sha256:beef".to_string()),
            Some(vec!["Read".to_string()]),
            None,
        );
        assert_eq!(entry.trust, "third-party");
        assert_eq!(entry.source, "https://github.com/someone/skill.git");
        assert_eq!(entry.pinned_ref.as_deref(), Some("abc123"));
        assert_eq!(entry.content_hash.as_deref(), Some("sha256:beef"));
        assert!(entry.installed_at.is_some());
        // Verify installed_at is a valid ISO 8601 timestamp
        let ts = entry.installed_at.as_ref().unwrap();
        assert!(ts.contains('T'), "expected ISO 8601 format, got: {ts}");
        assert_eq!(entry.allowed_tools.as_ref().unwrap(), &["Read"],);
    }
}
