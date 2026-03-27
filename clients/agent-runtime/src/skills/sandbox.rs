//! Tool sandboxing for third-party skill tools.
//! Restricts filesystem access to the skill's own directory.

use std::hash::{Hash, Hasher};
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
        validate_single_path(arg, skill_dir)?;
    }
    Ok(())
}

/// Validate a single path argument against the sandbox boundary.
fn validate_single_path(arg: &str, skill_dir: &Path) -> Result<(), SandboxViolation> {
    check_traversal_components(arg)?;

    let path = resolve_arg_path(arg, skill_dir);

    // Check for symlinks (including dangling ones) before exists()
    if is_symlink_entry(&path) {
        return check_symlink_target(&path, arg, skill_dir);
    }

    if path.exists() {
        check_existing_path(&path, arg, skill_dir)
    } else {
        check_nonexistent_path(&path, arg, skill_dir)
    }
}

/// Reject paths containing `..` (parent directory) components.
fn check_traversal_components(arg: &str) -> Result<(), SandboxViolation> {
    for component in std::path::Path::new(arg).components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(SandboxViolation::TraversalSequence {
                path: arg.to_string(),
            });
        }
    }
    Ok(())
}

/// Resolve an argument to an absolute path, joining with skill_dir if relative.
fn resolve_arg_path(arg: &str, skill_dir: &Path) -> PathBuf {
    if Path::new(arg).is_absolute() {
        PathBuf::from(arg)
    } else {
        skill_dir.join(arg)
    }
}

/// Canonicalize skill_dir, falling back to the original path on error.
fn canonical_skill(skill_dir: &Path) -> PathBuf {
    skill_dir
        .canonicalize()
        .unwrap_or_else(|_| skill_dir.to_path_buf())
}

/// Check whether the path entry itself is a symlink (without following it).
fn is_symlink_entry(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Validate a symlink target stays within the sandbox.
fn check_symlink_target(path: &Path, arg: &str, skill_dir: &Path) -> Result<(), SandboxViolation> {
    let canonical_skill = canonical_skill(skill_dir);
    match path.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(&canonical_skill) {
                return Err(SandboxViolation::SymlinkEscape {
                    path: arg.to_string(),
                    target: canonical,
                    skill_dir: canonical_skill,
                });
            }
            Ok(())
        }
        Err(_) => Err(SandboxViolation::SymlinkEscape {
            path: arg.to_string(),
            target: PathBuf::from("(dangling)"),
            skill_dir: canonical_skill,
        }),
    }
}

/// Validate an existing (non-symlink) path stays within the sandbox.
fn check_existing_path(path: &Path, arg: &str, skill_dir: &Path) -> Result<(), SandboxViolation> {
    let canonical_skill = canonical_skill(skill_dir);
    let Ok(canonical) = path.canonicalize() else {
        let path_fingerprint = fingerprint_path(arg);
        tracing::warn!(
            path_fingerprint = %path_fingerprint,
            "cannot canonicalize path — denying access"
        );
        return Err(SandboxViolation::PathEscape {
            path: arg.to_string(),
            skill_dir: canonical_skill,
        });
    };
    if canonical.starts_with(&canonical_skill) {
        return Ok(());
    }
    if path.is_symlink() {
        return Err(SandboxViolation::SymlinkEscape {
            path: arg.to_string(),
            target: canonical,
            skill_dir: canonical_skill,
        });
    }
    Err(SandboxViolation::PathEscape {
        path: arg.to_string(),
        skill_dir: canonical_skill,
    })
}

fn fingerprint_path(arg: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    arg.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Validate a nonexistent path by walking ancestors and checking absolute bounds.
fn check_nonexistent_path(
    path: &Path,
    arg: &str,
    skill_dir: &Path,
) -> Result<(), SandboxViolation> {
    let canonical_skill = canonical_skill(skill_dir);
    check_nearest_ancestor(path, arg, skill_dir, &canonical_skill)?;

    // Reject absolute paths outside sandbox
    if path.is_absolute() && !path.starts_with(&canonical_skill) && !path.starts_with(skill_dir) {
        return Err(SandboxViolation::PathEscape {
            path: arg.to_string(),
            skill_dir: canonical_skill,
        });
    }
    Ok(())
}

/// Walk up from `path` to find the nearest existing ancestor and verify it
/// stays within the sandbox (catches symlinked parent directories).
fn check_nearest_ancestor(
    path: &Path,
    arg: &str,
    skill_dir: &Path,
    canonical_skill: &Path,
) -> Result<(), SandboxViolation> {
    let mut ancestor = path.to_path_buf();
    while ancestor != *skill_dir && ancestor.parent().is_some() {
        ancestor = match ancestor.parent() {
            Some(p) => p.to_path_buf(),
            None => break,
        };
        if !ancestor.exists() {
            continue;
        }
        check_ancestor_symlink(&ancestor, arg, canonical_skill)?;
        check_ancestor_escape(&ancestor, arg, canonical_skill)?;
        break; // Only need to check the nearest existing ancestor
    }
    Ok(())
}

/// Check whether an ancestor is a symlink that escapes the sandbox.
fn check_ancestor_symlink(
    ancestor: &Path,
    arg: &str,
    canonical_skill: &Path,
) -> Result<(), SandboxViolation> {
    let is_symlink = ancestor
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    if !is_symlink {
        return Ok(());
    }
    match ancestor.canonicalize() {
        Ok(canonical) if canonical.starts_with(canonical_skill) => Ok(()),
        Ok(canonical) => Err(SandboxViolation::SymlinkEscape {
            path: arg.to_string(),
            target: canonical,
            skill_dir: canonical_skill.to_path_buf(),
        }),
        Err(_) => Err(SandboxViolation::SymlinkEscape {
            path: arg.to_string(),
            target: PathBuf::from("(dangling)"),
            skill_dir: canonical_skill.to_path_buf(),
        }),
    }
}

/// Verify the canonicalized ancestor stays in the sandbox.
fn check_ancestor_escape(
    ancestor: &Path,
    arg: &str,
    canonical_skill: &Path,
) -> Result<(), SandboxViolation> {
    if let Ok(canonical) = ancestor.canonicalize() {
        if !canonical.starts_with(canonical_skill) {
            return Err(SandboxViolation::PathEscape {
                path: arg.to_string(),
                skill_dir: canonical_skill.to_path_buf(),
            });
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
    fn traversal_embedded_in_path_components_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let result = validate_tool_paths(&["foo/bar/.."], dir.path());
        assert!(matches!(
            result,
            Err(SandboxViolation::TraversalSequence { .. })
        ));
    }

    #[test]
    fn relative_path_within_sandbox_subdir_allowed() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("subdir/file.txt"), "ok").unwrap();
        let result = validate_tool_paths(&["subdir/file.txt"], dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn empty_args_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let result = validate_tool_paths(&[], dir.path());
        assert!(result.is_ok());
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

    #[test]
    #[cfg(unix)]
    fn dangling_symlink_rejected() {
        let skill_dir = tempfile::tempdir().unwrap();
        // Create a symlink to a non-existent target
        std::os::unix::fs::symlink("/nonexistent/target", skill_dir.path().join("dangling"))
            .unwrap();
        let result = validate_tool_paths(&["dangling"], skill_dir.path());
        assert!(
            matches!(result, Err(SandboxViolation::SymlinkEscape { .. })),
            "expected SymlinkEscape for dangling symlink, got: {result:?}"
        );
    }

    #[test]
    #[cfg(unix)]
    fn symlinked_parent_directory_escape_rejected() {
        let skill_dir = tempfile::tempdir().unwrap();
        let outside_dir = tempfile::tempdir().unwrap();
        // Create a symlink inside skill_dir that points to the outside directory
        std::os::unix::fs::symlink(outside_dir.path(), skill_dir.path().join("escape-dir"))
            .unwrap();
        // Try to access a non-existent file through the symlinked parent
        let result = validate_tool_paths(&["escape-dir/new.txt"], skill_dir.path());
        assert!(
            matches!(result, Err(SandboxViolation::SymlinkEscape { .. })),
            "expected SymlinkEscape for symlinked parent dir, got: {result:?}"
        );
    }
}
