use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Trust tier for a skill, derived from its origin.
/// Never stored independently — always re-derived from SkillSource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillTrust {
    /// From official Corvus skills repo (Phase 2 — no skills qualify yet)
    Official,
    /// Created by user in workspace, or symlinked from local path
    Local,
    /// Installed from any external git source
    ThirdParty,
}

impl SkillTrust {
    /// Returns the string representation used in lockfile and prompt XML.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Local => "local",
            Self::ThirdParty => "third-party",
        }
    }
}

// Default must be Local — the safe trust level for unknown skills.
// Cannot use #[derive(Default)] because the first variant (Official) is not the default.
#[allow(clippy::derivable_impls)]
impl Default for SkillTrust {
    fn default() -> Self {
        Self::Local
    }
}

/// Where a skill was installed from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillSource {
    /// From the official Corvus skills registry (Phase 2)
    Official { repo: String, path: String },
    /// User-created in local workspace
    Local,
    /// Symlinked from a local path
    LinkedLocal { target: PathBuf },
    /// Cloned from a git repository
    GitRepo { url: String },
    /// Discovered via SkillForge
    Discovered { source: String, repo: String },
}

/// Origin metadata for an installed skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOrigin {
    pub source: SkillSource,
    pub installed_at: Option<String>, // ISO 8601
    pub pinned_ref: Option<String>,   // git commit SHA
    pub content_hash: Option<String>, // "sha256:<hex>"
}

impl Default for SkillOrigin {
    fn default() -> Self {
        Self {
            source: SkillSource::Local,
            installed_at: None,
            pinned_ref: None,
            content_hash: None,
        }
    }
}

/// Derive trust from source — the core security invariant.
impl From<&SkillSource> for SkillTrust {
    fn from(source: &SkillSource) -> Self {
        match source {
            SkillSource::Official { .. } => SkillTrust::Official,
            SkillSource::Local | SkillSource::LinkedLocal { .. } => SkillTrust::Local,
            SkillSource::GitRepo { .. } | SkillSource::Discovered { .. } => SkillTrust::ThirdParty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_source_maps_to_official_trust() {
        let source = SkillSource::Official {
            repo: "corvus-skills".to_string(),
            path: "coding/rust".to_string(),
        };
        assert_eq!(SkillTrust::from(&source), SkillTrust::Official);
    }

    #[test]
    fn local_source_maps_to_local_trust() {
        let source = SkillSource::Local;
        assert_eq!(SkillTrust::from(&source), SkillTrust::Local);
    }

    #[test]
    fn linked_local_source_maps_to_local_trust() {
        let source = SkillSource::LinkedLocal {
            target: PathBuf::from("/home/user/my-skill"),
        };
        assert_eq!(SkillTrust::from(&source), SkillTrust::Local);
    }

    #[test]
    fn git_repo_source_maps_to_third_party_trust() {
        let source = SkillSource::GitRepo {
            url: "https://github.com/someone/skill.git".to_string(),
        };
        assert_eq!(SkillTrust::from(&source), SkillTrust::ThirdParty);
    }

    #[test]
    fn discovered_source_maps_to_third_party_trust() {
        let source = SkillSource::Discovered {
            source: "skillforge".to_string(),
            repo: "https://github.com/someone/skill.git".to_string(),
        };
        assert_eq!(SkillTrust::from(&source), SkillTrust::ThirdParty);
    }

    #[test]
    fn trust_ordering_official_less_than_local_less_than_third_party() {
        assert!(SkillTrust::Official < SkillTrust::Local);
        assert!(SkillTrust::Local < SkillTrust::ThirdParty);
        assert!(SkillTrust::Official < SkillTrust::ThirdParty);
    }

    #[test]
    fn as_str_returns_correct_representations() {
        assert_eq!(SkillTrust::Official.as_str(), "official");
        assert_eq!(SkillTrust::Local.as_str(), "local");
        assert_eq!(SkillTrust::ThirdParty.as_str(), "third-party");
    }

    #[test]
    fn skill_origin_default_is_local_with_none_fields() {
        let origin = SkillOrigin::default();
        assert!(matches!(origin.source, SkillSource::Local));
        assert!(origin.installed_at.is_none());
        assert!(origin.pinned_ref.is_none());
        assert!(origin.content_hash.is_none());
    }

    #[test]
    fn privilege_escalation_prevention_git_repo_always_third_party() {
        // Even if the URL looks like an official repo, GitRepo source always maps
        // to ThirdParty trust — trust is derived from source variant, not content.
        let source = SkillSource::GitRepo {
            url: "https://github.com/corvus-official/skills.git".to_string(),
        };
        assert_eq!(SkillTrust::from(&source), SkillTrust::ThirdParty);

        let source = SkillSource::GitRepo {
            url: "https://official.corvus.dev/skills.git".to_string(),
        };
        assert_eq!(SkillTrust::from(&source), SkillTrust::ThirdParty);
    }
}
