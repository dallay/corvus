use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const VERSION_CHECK_FILE: &str = "version_check.json";
const VERSION_CHECK_TTL_SECS: u64 = 24 * 60 * 60;
const VERSION_CHECK_TIMEOUT_SECS: u64 = 2;
const UPDATE_CHECK_DISABLE_ENV: &str = "CORVUS_DISABLE_UPDATE_CHECK";
const INSTALL_SCRIPT_URL: &str = "https://profiletailors.com/install";
const PACKAGE_NAME: &str = "@dallay/corvus";
const RELEASE_ENDPOINTS: [&str; 2] = [
    "https://api.github.com/repos/profiletailors/corvus/releases/latest",
    "https://api.github.com/repos/dallay/corvus/releases/latest",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionCheckState {
    latest_version: String,
    checked_at_unix: u64,
    update_available: bool,
}

#[derive(Debug, Deserialize)]
struct LatestReleaseResponse {
    tag_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpdateNotice {
    current_version: String,
    latest_version: String,
}

pub async fn maybe_print_update_notice(config: &Config) {
    if is_update_check_disabled() {
        return;
    }

    if let Some(notice) = check_for_update(config, env!("CARGO_PKG_VERSION")).await {
        println!();
        println!(
            "⬆️  Update available: v{} (current v{})",
            notice.latest_version, notice.current_version
        );
        println!(
            "   If installed via script/binary: curl -fsSL {} | bash",
            INSTALL_SCRIPT_URL
        );
        println!(
            "   If installed via package manager: npm i -g {}@latest (or pnpm/yarn/bun)",
            PACKAGE_NAME
        );
    }
}

async fn check_for_update(config: &Config, current_version: &str) -> Option<UpdateNotice> {
    let current = normalize_version(current_version)?;
    let state_path = version_check_path(&config.workspace_dir);
    let cached_state = load_state(&state_path).await.ok().flatten();

    if let Some(cached) = cached_state.as_ref().filter(|state| !is_stale(state)) {
        return notice_from_state(current.clone(), cached);
    }

    let fetched = fetch_latest_release_version().await.ok();

    if let Some(latest_version) = fetched {
        let update_available =
            compare_semverish(&latest_version, &current).is_some_and(|ordering| ordering.is_gt());
        let state = VersionCheckState {
            latest_version,
            checked_at_unix: now_unix_secs(),
            update_available,
        };

        let _ = save_state(&state_path, &state).await;
        return notice_from_state(current, &state);
    }

    cached_state
        .as_ref()
        .and_then(|state| notice_from_state(current, state))
}

fn notice_from_state(current_version: String, state: &VersionCheckState) -> Option<UpdateNotice> {
    if !state.update_available {
        return None;
    }

    if compare_semverish(&state.latest_version, &current_version)
        .is_some_and(|ordering| ordering.is_gt())
    {
        Some(UpdateNotice {
            current_version,
            latest_version: state.latest_version.clone(),
        })
    } else {
        None
    }
}

fn is_update_check_disabled() -> bool {
    std::env::var(UPDATE_CHECK_DISABLE_ENV)
        .ok()
        .is_some_and(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
}

fn version_check_path(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("state").join(VERSION_CHECK_FILE)
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn is_stale(state: &VersionCheckState) -> bool {
    now_unix_secs().saturating_sub(state.checked_at_unix) > VERSION_CHECK_TTL_SECS
}

async fn load_state(path: &Path) -> Result<Option<VersionCheckState>> {
    if !tokio::fs::try_exists(path)
        .await
        .with_context(|| format!("failed to check version check state at {}", path.display()))?
    {
        return Ok(None);
    }

    let raw = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("failed to read version check state at {}", path.display()))?;
    let state = serde_json::from_str::<VersionCheckState>(&raw)
        .context("failed to parse version check state")?;
    Ok(Some(state))
}

async fn save_state(path: &Path, state: &VersionCheckState) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.with_context(|| {
            format!(
                "failed to create version check state directory {}",
                parent.display()
            )
        })?;
    }

    let body = serde_json::to_vec_pretty(state).context("failed to serialize version state")?;
    tokio::fs::write(path, body)
        .await
        .with_context(|| format!("failed to write version check state at {}", path.display()))
}

async fn fetch_latest_release_version() -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(VERSION_CHECK_TIMEOUT_SECS))
        .build()
        .context("failed to build update-check client")?;

    for endpoint in RELEASE_ENDPOINTS {
        let response = client
            .get(endpoint)
            .header(reqwest::header::USER_AGENT, "corvus-update-check")
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .await;

        let Ok(response) = response else {
            continue;
        };

        let Ok(response) = response.error_for_status() else {
            continue;
        };

        let payload: LatestReleaseResponse = response
            .json()
            .await
            .context("failed to parse release metadata")?;

        if let Some(normalized) = normalize_version(&payload.tag_name) {
            return Ok(normalized);
        }
    }

    anyhow::bail!("failed to resolve latest release version from release endpoints")
}

fn normalize_version(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed)
        .to_string();

    parse_semverish(&normalized).map(|_| normalized)
}

fn compare_semverish(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let left_parsed = parse_semverish(left)?;
    let right_parsed = parse_semverish(right)?;

    let core_ordering = left_parsed
        .0
        .cmp(&right_parsed.0)
        .then(left_parsed.1.cmp(&right_parsed.1))
        .then(left_parsed.2.cmp(&right_parsed.2));
    if !core_ordering.is_eq() {
        return Some(core_ordering);
    }

    Some(compare_prerelease(
        left_parsed.3.as_deref(),
        right_parsed.3.as_deref(),
    ))
}

fn compare_prerelease(left: Option<&str>, right: Option<&str>) -> std::cmp::Ordering {
    match (left, right) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(left), Some(right)) => {
            let left_parts: Vec<&str> = left.split('.').collect();
            let right_parts: Vec<&str> = right.split('.').collect();

            for (l, r) in left_parts.iter().zip(right_parts.iter()) {
                let left_numeric = l.parse::<u64>();
                let right_numeric = r.parse::<u64>();

                let ordering = match (left_numeric, right_numeric) {
                    (Ok(a), Ok(b)) => a.cmp(&b),
                    (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                    (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                    (Err(_), Err(_)) => l.cmp(r),
                };

                if !ordering.is_eq() {
                    return ordering;
                }
            }

            left_parts.len().cmp(&right_parts.len())
        }
    }
}

fn parse_semverish(version: &str) -> Option<(u64, u64, u64, Option<String>)> {
    let version = version.trim();
    if version.is_empty() {
        return None;
    }

    let without_build = version.split_once('+').map_or(version, |(core, _)| core);
    let (core, prerelease_raw) = without_build
        .split_once('-')
        .map_or((without_build, None), |(core, prerelease)| {
            (core, Some(prerelease.trim()))
        });

    let core = core.trim();
    if core.is_empty() {
        return None;
    }

    let mut parts = core.split('.');

    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }

    let prerelease = prerelease_raw
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    Some((major, minor, patch, prerelease))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_version_accepts_with_or_without_v_prefix() {
        assert_eq!(normalize_version("v0.1.7"), Some("0.1.7".to_string()));
        assert_eq!(normalize_version("0.1.7"), Some("0.1.7".to_string()));
        assert_eq!(normalize_version("V1.2.3"), Some("1.2.3".to_string()));
    }

    #[test]
    fn normalize_version_rejects_invalid_values() {
        assert_eq!(normalize_version("latest"), None);
        assert_eq!(normalize_version("v1"), None);
        assert_eq!(normalize_version(""), None);
    }

    #[test]
    fn compare_semverish_orders_versions() {
        assert_eq!(
            compare_semverish("0.1.8", "0.1.7"),
            Some(std::cmp::Ordering::Greater)
        );
        assert_eq!(
            compare_semverish("0.1.7", "0.1.7"),
            Some(std::cmp::Ordering::Equal)
        );
        assert_eq!(
            compare_semverish("0.1.6", "0.1.7"),
            Some(std::cmp::Ordering::Less)
        );
    }

    #[test]
    fn parse_semverish_accepts_pre_release_patch_suffix() {
        assert_eq!(
            parse_semverish("1.2.3-beta.1"),
            Some((1, 2, 3, Some("beta.1".to_string())))
        );
    }

    #[test]
    fn compare_semverish_treats_prerelease_as_lower_precedence() {
        assert_eq!(
            compare_semverish("1.0.0", "1.0.0-beta.1"),
            Some(std::cmp::Ordering::Greater)
        );
        assert_eq!(
            compare_semverish("1.0.0-beta.1", "1.0.0"),
            Some(std::cmp::Ordering::Less)
        );
    }

    #[test]
    fn stale_cache_detection_works() {
        let now = now_unix_secs();
        let fresh = VersionCheckState {
            latest_version: "0.1.8".into(),
            checked_at_unix: now.saturating_sub(30),
            update_available: true,
        };
        let stale = VersionCheckState {
            latest_version: "0.1.8".into(),
            checked_at_unix: now.saturating_sub(VERSION_CHECK_TTL_SECS + 10),
            update_available: true,
        };

        assert!(!is_stale(&fresh));
        assert!(is_stale(&stale));
    }

    #[test]
    fn notice_requires_newer_latest_version() {
        let update = VersionCheckState {
            latest_version: "0.1.8".into(),
            checked_at_unix: now_unix_secs(),
            update_available: true,
        };
        let no_update_same = VersionCheckState {
            latest_version: "0.1.7".into(),
            checked_at_unix: now_unix_secs(),
            update_available: true,
        };
        let no_update_flag = VersionCheckState {
            latest_version: "0.1.8".into(),
            checked_at_unix: now_unix_secs(),
            update_available: false,
        };

        assert!(notice_from_state("0.1.7".into(), &update).is_some());
        assert!(notice_from_state("0.1.7".into(), &no_update_same).is_none());
        assert!(notice_from_state("0.1.7".into(), &no_update_flag).is_none());
    }

    #[tokio::test]
    async fn save_and_load_state_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state").join("version_check.json");
        let state = VersionCheckState {
            latest_version: "0.1.9".into(),
            checked_at_unix: 123,
            update_available: true,
        };

        save_state(&path, &state).await.unwrap();
        let loaded = load_state(&path).await.unwrap().unwrap();

        assert_eq!(loaded.latest_version, "0.1.9");
        assert_eq!(loaded.checked_at_unix, 123);
        assert!(loaded.update_available);
    }
}
