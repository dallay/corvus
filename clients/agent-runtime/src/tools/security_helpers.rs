use crate::security::SecurityPolicy;
use std::path::PathBuf;
use std::sync::Arc;

use super::traits::ToolResult;

/// Outcome of the shared path security check.
///
/// A successful check returns the canonicalized, workspace-confined path
/// ready for use. A rejected check returns an early `ToolResult` that the
/// caller must immediately return to the agent.
pub(crate) enum PathCheckOutcome {
    Resolved(PathBuf),
    Rejected(ToolResult),
}

/// Run the five mandatory security guards that every file-accessing tool must
/// perform before touching the filesystem:
///
/// 1. Pre-check rate limit (fast path, no budget consumed).
/// 2. Validate the raw path against the security policy allowlist.
/// 3. Record the action (consumes one budget token, preventing path-probing).
/// 4. Canonicalize the joined path to resolve symlinks.
/// 5. Verify the canonical path is still within the workspace.
///
/// The `max_size` check and any tool-specific validation (e.g. `is_file()`)
/// are intentionally left to the caller because the limits differ per tool.
///
/// # Ordering rationale
/// `record_action` is called BEFORE `canonicalize` so that every
/// non-trivially-rejected request consumes rate-limit budget. This prevents
/// an attacker from probing path existence (via canonicalize I/O errors)
/// without paying the rate-limit cost.
pub(crate) async fn check_and_resolve_path(
    security: &Arc<SecurityPolicy>,
    path: &str,
) -> PathCheckOutcome {
    if security.is_rate_limited() {
        return PathCheckOutcome::Rejected(ToolResult {
            success: false,
            output: String::new(),
            error: Some("Rate limit exceeded: too many actions in the last hour".into()),
            structured: None,
        });
    }

    if !security.is_path_allowed(path) {
        return PathCheckOutcome::Rejected(ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!("Path not allowed by security policy: {path}")),
            structured: None,
        });
    }

    // Record action BEFORE canonicalization so that every non-trivially-rejected
    // request consumes rate limit budget. This prevents attackers from probing
    // path existence (via canonicalize errors) without rate limit cost.
    if !security.record_action() {
        return PathCheckOutcome::Rejected(ToolResult {
            success: false,
            output: String::new(),
            error: Some("Rate limit exceeded: action budget exhausted".into()),
            structured: None,
        });
    }

    let full_path = security.workspace_dir.join(path);

    let resolved_path = match tokio::fs::canonicalize(&full_path).await {
        Ok(p) => p,
        Err(e) => {
            return PathCheckOutcome::Rejected(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to resolve file path: {e}")),
                structured: None,
            });
        }
    };

    if !security.is_resolved_path_allowed(&resolved_path) {
        return PathCheckOutcome::Rejected(ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!(
                "Resolved path escapes workspace: {}",
                resolved_path.display()
            )),
            structured: None,
        });
    }

    PathCheckOutcome::Resolved(resolved_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AutonomyLevel, SecurityPolicy};
    use std::sync::Arc;
    use tempfile::TempDir;

    fn make_security(dir: &std::path::Path) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            workspace_dir: dir.to_path_buf(),
            ..SecurityPolicy::default()
        })
    }

    #[tokio::test]
    async fn resolves_valid_path() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("hello.txt");
        std::fs::write(&file, "hi").unwrap();

        let security = make_security(dir.path());
        let outcome = check_and_resolve_path(&security, "hello.txt").await;

        assert!(matches!(outcome, PathCheckOutcome::Resolved(_)));
    }

    #[tokio::test]
    async fn rejects_path_outside_workspace() {
        let dir = TempDir::new().unwrap();
        let security = make_security(dir.path());

        let outcome = check_and_resolve_path(&security, "../etc/passwd").await;

        match outcome {
            PathCheckOutcome::Rejected(result) => {
                assert!(!result.success);
                let err = result.error.unwrap();
                assert!(
                    err.contains("Path not allowed") || err.contains("escapes workspace"),
                    "unexpected error: {err}"
                );
            }
            PathCheckOutcome::Resolved(_) => panic!("expected rejection"),
        }
    }

    #[tokio::test]
    async fn rejects_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let security = make_security(dir.path());

        let outcome = check_and_resolve_path(&security, "no_such_file.txt").await;

        match outcome {
            PathCheckOutcome::Rejected(result) => {
                assert!(!result.success);
                assert!(result
                    .error
                    .unwrap()
                    .contains("Failed to resolve file path"));
            }
            PathCheckOutcome::Resolved(_) => panic!("expected rejection"),
        }
    }
}
