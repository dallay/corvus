//! Tool sandboxing for third-party skill tools.
//! Restricts filesystem access to the skill's own directory.

use std::path::{Path, PathBuf};

/// Sandbox violation types.
#[derive(Debug, PartialEq)]
pub enum SandboxViolation {
    /// Path contains `../` traversal sequence
    TraversalSequence { path: String },
    /// Resolved path escapes the skill directory
    PathEscape { path: String, skill_dir: PathBuf },
    /// Symlink target escapes the skill directory
    SymlinkEscape {
        path: String,
        target: PathBuf,
        skill_dir: PathBuf,
    },
}

impl std::fmt::Display for SandboxViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TraversalSequence { path } => {
                write!(f, "path traversal detected in '{path}'")
            }
            Self::PathEscape { path, skill_dir } => {
                write!(
                    f,
                    "path '{path}' escapes skill directory '{}'",
                    skill_dir.display()
                )
            }
            Self::SymlinkEscape {
                path,
                target,
                skill_dir,
            } => {
                write!(
                    f,
                    "symlink '{path}' targets '{}' outside skill directory '{}'",
                    target.display(),
                    skill_dir.display()
                )
            }
        }
    }
}

/// Sandbox policy for a tool.
#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    /// Whether sandboxing is enabled for this tool.
    pub enabled: bool,
    /// The skill directory (working directory for sandboxed tools).
    pub skill_dir: PathBuf,
}

/// Build sandbox policy based on trust tier.
pub fn build_policy(trust: super::trust::SkillTrust, skill_dir: &Path) -> SandboxPolicy {
    SandboxPolicy {
        enabled: trust == super::trust::SkillTrust::ThirdParty,
        skill_dir: skill_dir.to_path_buf(),
    }
}

/// Validate that tool path arguments don't escape the skill directory.
/// Returns Ok(()) if all paths are safe, or Err with the first violation found.
pub fn validate_tool_paths(args: &[&str], skill_dir: &Path) -> Result<(), SandboxViolation> {
    for arg in args {
        // Check for traversal via path components
        let check_path = std::path::Path::new(arg);
        for component in check_path.components() {
            if matches!(component, std::path::Component::ParentDir) {
                return Err(SandboxViolation::TraversalSequence {
                    path: arg.to_string(),
                });
            }
        }

        // Check if path resolves within skill directory
        let path = if Path::new(arg).is_absolute() {
            PathBuf::from(arg)
        } else {
            skill_dir.join(arg)
        };

        // Canonicalize if the path exists (handles symlinks)
        if path.exists() {
            if let Ok(canonical) = path.canonicalize() {
                let canonical_skill = skill_dir
                    .canonicalize()
                    .unwrap_or_else(|_| skill_dir.to_path_buf());
                if !canonical.starts_with(&canonical_skill) {
                    // Check if it's a symlink specifically
                    if path.is_symlink() {
                        return Err(SandboxViolation::SymlinkEscape {
                            path: arg.to_string(),
                            target: canonical,
                            skill_dir: canonical_skill,
                        });
                    }
                    return Err(SandboxViolation::PathEscape {
                        path: arg.to_string(),
                        skill_dir: canonical_skill,
                    });
                }
            } else {
                // Existing path that can't be canonicalized — deny by default
                tracing::warn!(
                    "cannot canonicalize existing path '{}' — denying access",
                    arg
                );
                return Err(SandboxViolation::PathEscape {
                    path: arg.to_string(),
                    skill_dir: skill_dir
                        .canonicalize()
                        .unwrap_or_else(|_| skill_dir.to_path_buf()),
                });
            }
        } else {
            // Path doesn't exist yet — check the logical path
            // Normalize by checking if it logically escapes
            if path.is_absolute() {
                let canonical_skill = skill_dir
                    .canonicalize()
                    .unwrap_or_else(|_| skill_dir.to_path_buf());
                if !path.starts_with(&canonical_skill) && !path.starts_with(skill_dir) {
                    return Err(SandboxViolation::PathEscape {
                        path: arg.to_string(),
                        skill_dir: canonical_skill,
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn traversal_sequence_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let result = validate_tool_paths(&["../etc/passwd"], dir.path());
        assert!(matches!(
            result,
            Err(SandboxViolation::TraversalSequence { .. })
        ));
    }

    #[test]
    fn valid_relative_path_allowed() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("file.txt"), "test").unwrap();
        let result = validate_tool_paths(&["file.txt"], dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn absolute_path_outside_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let result = validate_tool_paths(&["/etc/passwd"], dir.path());
        assert!(matches!(result, Err(SandboxViolation::PathEscape { .. })));
    }

    #[test]
    fn build_policy_thirdparty_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let policy = build_policy(super::super::trust::SkillTrust::ThirdParty, dir.path());
        assert!(policy.enabled);
    }

    #[test]
    fn build_policy_local_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let policy = build_policy(super::super::trust::SkillTrust::Local, dir.path());
        assert!(!policy.enabled);
    }

    #[test]
    fn build_policy_official_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let policy = build_policy(super::super::trust::SkillTrust::Official, dir.path());
        assert!(!policy.enabled);
    }

    #[test]
    #[cfg(unix)]
    fn symlink_escape_rejected() {
        let skill_dir = tempfile::tempdir().unwrap();
        let outside_dir = tempfile::tempdir().unwrap();
        let outside_file = outside_dir.path().join("secret.txt");
        std::fs::write(&outside_file, "secret").unwrap();
        std::os::unix::fs::symlink(&outside_file, skill_dir.path().join("escape-link")).unwrap();
        let result = validate_tool_paths(&["escape-link"], skill_dir.path());
        assert!(
            matches!(result, Err(SandboxViolation::SymlinkEscape { .. })),
            "expected SymlinkEscape, got: {result:?}"
        );
    }
}
