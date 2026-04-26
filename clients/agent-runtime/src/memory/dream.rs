use anyhow::Result;
use chrono::{Duration, Local, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const DREAM_STATE_FILE: &str = "dream_state.json";
const DREAM_LOCK_FILE: &str = "dream.lock";
const DREAM_MEMORY_LINE_BUDGET: usize = 200;
const DREAM_MEMORY_BYTE_BUDGET: usize = 25 * 1024;
const DREAM_SESSION_TRIGGER_COUNT: usize = 5;
const DREAM_TIME_TRIGGER_HOURS: i64 = 24;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DreamTriggerReason {
    TimeElapsed,
    SessionCount,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DreamRunStatus {
    Skipped,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DreamLockState {
    Acquired,
    Busy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DreamPhase {
    Orientation,
    RecentSignalCollection,
    Consolidation,
    PruneIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DreamPhaseResult {
    pub phase: DreamPhase,
    pub summary: String,
    pub touched_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DreamLaunchContract {
    pub read_only_project_access: bool,
    pub writable_roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DreamSessionStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DreamSessionStateRecord {
    pub session_id: String,
    pub status: DreamSessionStatus,
    pub trigger_reason: DreamTriggerReason,
    pub completion_recorded_at: String,
    pub last_attempt_at: Option<String>,
    pub completed_at: Option<String>,
    pub artifact_refs: Vec<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DreamSessionReport {
    pub session_id: String,
    pub status: DreamSessionStatus,
    pub artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryConsolidationReport {
    pub trigger_reason: DreamTriggerReason,
    pub status: DreamRunStatus,
    pub lock_state: DreamLockState,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub phases: Vec<DreamPhaseResult>,
    pub normalized_dates: u64,
    pub duplicates_removed: u64,
    pub retained_lines: usize,
    pub launch_contract: DreamLaunchContract,
    pub sessions_considered: usize,
    pub sessions_processed: usize,
    pub session_reports: Vec<DreamSessionReport>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DreamState {
    last_successful_run_at: Option<String>,
    completed_sessions: Vec<DreamSessionStateRecord>,
    last_report: Option<MemoryConsolidationReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DreamEligibility {
    Eligible,
    NotCompleted,
    AlreadyConsolidated,
}

pub fn run_if_triggered(workspace_dir: &Path) -> Result<Option<MemoryConsolidationReport>> {
    let state = load_state(workspace_dir)?;
    let Some(trigger_reason) = due_trigger_reason(&state)? else {
        return Ok(None);
    };
    run_now(workspace_dir, trigger_reason)
}

pub fn run_now(
    workspace_dir: &Path,
    trigger_reason: DreamTriggerReason,
) -> Result<Option<MemoryConsolidationReport>> {
    let Some(lock_guard) = DreamLockGuard::acquire(workspace_dir)? else {
        return Ok(Some(MemoryConsolidationReport {
            trigger_reason,
            status: DreamRunStatus::Skipped,
            lock_state: DreamLockState::Busy,
            started_at: Utc::now().to_rfc3339(),
            finished_at: Some(Utc::now().to_rfc3339()),
            phases: vec![],
            normalized_dates: 0,
            duplicates_removed: 0,
            retained_lines: 0,
            launch_contract: launch_contract(workspace_dir),
            sessions_considered: 0,
            sessions_processed: 0,
            session_reports: vec![],
        }));
    };

    let mut state = load_state(workspace_dir)?;
    let session_ids = pending_session_ids(&state);
    let started_at = Utc::now().to_rfc3339();
    let memory_dir = workspace_dir.join("memory");
    fs::create_dir_all(&memory_dir)?;
    let core_path = workspace_dir.join("MEMORY.md");

    let mut phases = Vec::new();
    let touched = vec![display_path(&core_path), display_path(&memory_dir)];
    phases.push(DreamPhaseResult {
        phase: DreamPhase::Orientation,
        summary: format!(
            "Scanned memory root, core memory file, and {} pending Dream session(s).",
            session_ids.len()
        ),
        touched_files: touched.clone(),
    });

    let recent_signal_files = recent_memory_files(&memory_dir)?;
    phases.push(DreamPhaseResult {
        phase: DreamPhase::RecentSignalCollection,
        summary: format!(
            "Collected {} recent memory log files.",
            recent_signal_files.len()
        ),
        touched_files: recent_signal_files
            .iter()
            .map(|path| display_path(path))
            .collect(),
    });

    let existing = if core_path.exists() {
        fs::read_to_string(&core_path)?
    } else {
        "# Long-Term Memory\n\n".to_string()
    };
    let mut normalized_dates = 0_u64;
    let mut merged_lines = extract_memory_lines(&existing);
    for file in &recent_signal_files {
        let content = fs::read_to_string(file)?;
        for line in extract_memory_lines(&content) {
            let normalized = normalize_relative_dates(&line, &mut normalized_dates);
            merged_lines.push(normalized);
        }
    }

    let mut session_reports = Vec::new();
    let mut artifact_refs = Vec::new();
    for session_id in &session_ids {
        let artifact_ref = artifact_ref_for_session(session_id);
        artifact_refs.push(artifact_ref.clone());
        merged_lines.push(format!("Dream summary for completed session {session_id}"));
        session_reports.push(DreamSessionReport {
            session_id: session_id.clone(),
            status: DreamSessionStatus::Completed,
            artifact_refs: vec![artifact_ref],
        });
    }

    phases.push(DreamPhaseResult {
        phase: DreamPhase::Consolidation,
        summary: format!(
            "Merged {} candidate memory lines across {} completed session(s).",
            merged_lines.len(),
            session_ids.len()
        ),
        touched_files: vec![display_path(&core_path)],
    });

    let deduped = dedupe_preserving_order(merged_lines);
    let duplicates_removed = deduped.removed;
    let pruned = prune_to_budget(&deduped.lines);
    let retained_lines = pruned.len();

    let mut rendered = String::from("# Long-Term Memory\n\n");
    for line in &pruned {
        rendered.push_str("- ");
        rendered.push_str(line);
        rendered.push('\n');
    }
    fs::write(&core_path, rendered)?;

    phases.push(DreamPhaseResult {
        phase: DreamPhase::PruneIndex,
        summary: format!(
            "Pruned duplicates and enforced {DREAM_MEMORY_LINE_BUDGET} line / {DREAM_MEMORY_BYTE_BUDGET} byte budget."
        ),
        touched_files: vec![display_path(&core_path)],
    });

    mark_sessions_completed(&mut state, &session_ids, &artifact_refs, &trigger_reason);

    let report = MemoryConsolidationReport {
        trigger_reason,
        status: DreamRunStatus::Completed,
        lock_state: DreamLockState::Acquired,
        started_at,
        finished_at: Some(Utc::now().to_rfc3339()),
        phases,
        normalized_dates,
        duplicates_removed,
        retained_lines,
        launch_contract: launch_contract(workspace_dir),
        sessions_considered: session_ids.len(),
        sessions_processed: session_ids.len(),
        session_reports,
    };

    state.last_successful_run_at = report.finished_at.clone();
    state.last_report = Some(report.clone());
    store_state(workspace_dir, &state)?;
    drop(lock_guard);

    Ok(Some(report))
}

pub fn record_session_completion(
    workspace_dir: &Path,
    session_id: &str,
) -> Result<DreamSessionStateRecord> {
    let mut state = load_state(workspace_dir)?;
    let now = Utc::now().to_rfc3339();
    let artifact_ref = artifact_ref_for_session(session_id);

    let core_path = workspace_dir.join("MEMORY.md");
    let artifact_already_written = core_path.exists()
        && fs::read_to_string(&core_path)
            .map(|content| {
                content.contains(&format!(
                    "- Dream summary for completed session {session_id}"
                ))
            })
            .unwrap_or(false);

    if let Some(existing) = state
        .completed_sessions
        .iter_mut()
        .find(|record| record.session_id == session_id)
    {
        if existing.status != DreamSessionStatus::Completed && artifact_already_written {
            existing.status = DreamSessionStatus::Completed;
            existing.completed_at = Some(now.clone());
            existing.last_attempt_at = Some(now.clone());
            existing.artifact_refs = vec![artifact_ref];
            existing.failure_reason = None;
        }
        let existing = existing.clone();
        store_state(workspace_dir, &state)?;
        return Ok(existing);
    }

    let record = DreamSessionStateRecord {
        session_id: session_id.to_string(),
        status: if artifact_already_written {
            DreamSessionStatus::Completed
        } else {
            DreamSessionStatus::Pending
        },
        trigger_reason: DreamTriggerReason::SessionCount,
        completion_recorded_at: now.clone(),
        last_attempt_at: artifact_already_written.then(|| now.clone()),
        completed_at: artifact_already_written.then(|| now.clone()),
        artifact_refs: if artifact_already_written {
            vec![artifact_ref]
        } else {
            vec![]
        },
        failure_reason: None,
    };
    state.completed_sessions.push(record.clone());
    state
        .completed_sessions
        .sort_by(|a, b| a.session_id.cmp(&b.session_id));
    store_state(workspace_dir, &state)?;
    Ok(record)
}

pub fn dream_eligibility(workspace_dir: &Path, session_id: &str) -> Result<DreamEligibility> {
    let state = load_state(workspace_dir)?;
    Ok(
        match state
            .completed_sessions
            .iter()
            .find(|record| record.session_id == session_id)
        {
            None => DreamEligibility::NotCompleted,
            Some(record) if record.status == DreamSessionStatus::Completed => {
                DreamEligibility::AlreadyConsolidated
            }
            Some(_) => DreamEligibility::Eligible,
        },
    )
}

fn due_trigger_reason(state: &DreamState) -> Result<Option<DreamTriggerReason>> {
    let pending_count = state
        .completed_sessions
        .iter()
        .filter(|record| {
            matches!(
                record.status,
                DreamSessionStatus::Pending | DreamSessionStatus::Failed
            )
        })
        .count();
    if pending_count >= DREAM_SESSION_TRIGGER_COUNT {
        return Ok(Some(DreamTriggerReason::SessionCount));
    }

    let Some(last_run_at) = state.last_successful_run_at.as_deref() else {
        return if pending_count > 0 {
            Ok(Some(DreamTriggerReason::TimeElapsed))
        } else {
            Ok(None)
        };
    };

    let parsed = chrono::DateTime::parse_from_rfc3339(last_run_at)?.with_timezone(&Utc);
    if pending_count > 0
        && Utc::now().signed_duration_since(parsed) >= Duration::hours(DREAM_TIME_TRIGGER_HOURS)
    {
        return Ok(Some(DreamTriggerReason::TimeElapsed));
    }

    Ok(None)
}

fn pending_session_ids(state: &DreamState) -> Vec<String> {
    state
        .completed_sessions
        .iter()
        .filter(|record| {
            matches!(
                record.status,
                DreamSessionStatus::Pending | DreamSessionStatus::Failed
            )
        })
        .map(|record| record.session_id.clone())
        .collect()
}

fn mark_sessions_completed(
    state: &mut DreamState,
    session_ids: &[String],
    artifact_refs: &[String],
    trigger_reason: &DreamTriggerReason,
) {
    let completed_at = Utc::now().to_rfc3339();
    for session_id in session_ids {
        if let Some(record) = state
            .completed_sessions
            .iter_mut()
            .find(|record| record.session_id == *session_id)
        {
            record.status = DreamSessionStatus::Completed;
            record.trigger_reason = trigger_reason.clone();
            record.last_attempt_at = Some(completed_at.clone());
            record.completed_at = Some(completed_at.clone());
            record.artifact_refs = vec![artifact_ref_for_session(session_id)];
            record.failure_reason = None;
        }
    }
    let _ = artifact_refs;
}

fn artifact_ref_for_session(session_id: &str) -> String {
    format!("dream/session/{session_id}")
}

fn extract_memory_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.trim_start_matches("- ").trim().to_string())
        .collect()
}

fn normalize_relative_dates(line: &str, counter: &mut u64) -> String {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let yesterday = (Local::now() - Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let tomorrow = (Local::now() + Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let mut normalized = line.to_string();
    for (needle, replacement) in [
        (" today", format!(" on {today}")),
        (" yesterday", format!(" on {yesterday}")),
        (" tomorrow", format!(" on {tomorrow}")),
    ] {
        if normalized.contains(needle) {
            normalized = normalized.replace(needle, &replacement);
            *counter = counter.saturating_add(1);
        }
    }
    normalized
}

struct DedupedLines {
    lines: Vec<String>,
    removed: u64,
}

fn dedupe_preserving_order(lines: Vec<String>) -> DedupedLines {
    let mut seen = BTreeSet::new();
    let mut kept = Vec::new();
    let mut removed = 0_u64;
    for line in lines {
        let key = line.to_ascii_lowercase();
        if seen.insert(key) {
            kept.push(line);
        } else {
            removed = removed.saturating_add(1);
        }
    }
    DedupedLines {
        lines: kept,
        removed,
    }
}

fn prune_to_budget(lines: &[String]) -> Vec<String> {
    let mut kept = Vec::new();
    let mut bytes = "# Long-Term Memory\n\n".len();
    for line in lines.iter().rev() {
        let projected = bytes + line.len() + 3;
        if kept.len() >= DREAM_MEMORY_LINE_BUDGET || projected > DREAM_MEMORY_BYTE_BUDGET {
            continue;
        }
        kept.push(line.clone());
        bytes = projected;
    }
    kept.reverse();
    kept
}

fn recent_memory_files(memory_dir: &Path) -> Result<Vec<PathBuf>> {
    if !memory_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files: Vec<PathBuf> = fs::read_dir(memory_dir)?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
        .collect();
    files.sort();
    if files.len() > 5 {
        files = files.split_off(files.len() - 5);
    }
    Ok(files)
}

fn launch_contract(workspace_dir: &Path) -> DreamLaunchContract {
    DreamLaunchContract {
        read_only_project_access: true,
        writable_roots: vec![
            display_path(&workspace_dir.join("MEMORY.md")),
            display_path(&workspace_dir.join("memory")),
            display_path(&workspace_dir.join("state")),
        ],
    }
}

fn dream_state_path(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("state").join(DREAM_STATE_FILE)
}

fn dream_lock_path(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("state").join(DREAM_LOCK_FILE)
}

fn load_state(workspace_dir: &Path) -> Result<DreamState> {
    let path = dream_state_path(workspace_dir);
    if !path.exists() {
        return Ok(DreamState::default());
    }
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

fn store_state(workspace_dir: &Path, state: &DreamState) -> Result<()> {
    let path = dream_state_path(workspace_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

struct DreamLockGuard {
    path: PathBuf,
}

impl DreamLockGuard {
    fn acquire(workspace_dir: &Path) -> Result<Option<Self>> {
        let path = dream_lock_path(workspace_dir);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => Ok(Some(Self { path })),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for DreamLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn rejects_active_session_without_recorded_completion() {
        let tmp = TempDir::new().unwrap();

        let eligibility = dream_eligibility(tmp.path(), "sess-active").unwrap();

        assert_eq!(eligibility, DreamEligibility::NotCompleted);
        assert!(run_if_triggered(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn accepts_recorded_completed_session_and_creates_deterministic_artifact_ref() {
        let tmp = TempDir::new().unwrap();
        record_session_completion(tmp.path(), "sess-123").unwrap();

        let eligibility = dream_eligibility(tmp.path(), "sess-123").unwrap();
        assert_eq!(eligibility, DreamEligibility::Eligible);

        let report = run_if_triggered(tmp.path()).unwrap().unwrap();
        assert_eq!(report.trigger_reason, DreamTriggerReason::TimeElapsed);
        assert_eq!(report.sessions_processed, 1);
        assert_eq!(
            report.session_reports[0].artifact_refs,
            vec!["dream/session/sess-123".to_string()]
        );
    }

    #[test]
    fn suppresses_duplicate_triggers_by_session_id() {
        let tmp = TempDir::new().unwrap();
        let first = record_session_completion(tmp.path(), "sess-123").unwrap();
        let second = record_session_completion(tmp.path(), "sess-123").unwrap();

        assert_eq!(first.session_id, second.session_id);

        let first_report = run_if_triggered(tmp.path()).unwrap().unwrap();
        assert_eq!(first_report.sessions_processed, 1);

        let eligibility = dream_eligibility(tmp.path(), "sess-123").unwrap();
        assert_eq!(eligibility, DreamEligibility::AlreadyConsolidated);
        assert!(run_if_triggered(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn session_trigger_fires_after_five_sessions() {
        let tmp = TempDir::new().unwrap();
        for index in 0..5 {
            record_session_completion(tmp.path(), &format!("sess-{index}")).unwrap();
        }

        let report = run_if_triggered(tmp.path()).unwrap().unwrap();
        assert_eq!(report.trigger_reason, DreamTriggerReason::SessionCount);
        assert_eq!(report.status, DreamRunStatus::Completed);
        assert_eq!(report.sessions_processed, 5);
    }

    #[test]
    fn lock_prevents_concurrent_runs() {
        let tmp = TempDir::new().unwrap();
        let _guard = DreamLockGuard::acquire(tmp.path()).unwrap().unwrap();
        let report = run_now(tmp.path(), DreamTriggerReason::Manual)
            .unwrap()
            .unwrap();
        assert_eq!(report.lock_state, DreamLockState::Busy);
        assert_eq!(report.status, DreamRunStatus::Skipped);
    }

    #[test]
    fn dream_normalizes_relative_dates_and_prunes_duplicates() {
        let tmp = TempDir::new().unwrap();
        record_session_completion(tmp.path(), "sess-123").unwrap();
        fs::create_dir_all(tmp.path().join("memory")).unwrap();
        fs::write(
            tmp.path().join("memory").join("2026-04-22.md"),
            "# Daily Log\n\n- Met user today\n- Met user today\n- Reviewed roadmap yesterday\n",
        )
        .unwrap();

        let report = run_now(tmp.path(), DreamTriggerReason::Manual)
            .unwrap()
            .unwrap();
        let memory = fs::read_to_string(tmp.path().join("MEMORY.md")).unwrap();

        assert!(report.normalized_dates >= 2);
        assert!(report.duplicates_removed >= 1);
        assert!(memory.contains("on "));
        assert!(memory.contains("Dream summary for completed session sess-123"));
    }

    #[test]
    fn retries_failed_session_after_manual_recovery_and_keeps_single_logical_result() {
        let tmp = TempDir::new().unwrap();
        let record = record_session_completion(tmp.path(), "sess-retry").unwrap();
        let state_path = tmp.path().join("state").join(DREAM_STATE_FILE);
        let raw = fs::read_to_string(&state_path).unwrap();
        let mut state: DreamState = serde_json::from_str(&raw).unwrap();
        let failed = state
            .completed_sessions
            .iter_mut()
            .find(|entry| entry.session_id == "sess-retry")
            .unwrap();
        failed.status = DreamSessionStatus::Failed;
        failed.last_attempt_at = Some(record.completion_recorded_at.clone());
        failed.failure_reason = Some("transient backend error".to_string());
        fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();

        let report = run_now(tmp.path(), DreamTriggerReason::Manual)
            .unwrap()
            .unwrap();
        let recovered = record_session_completion(tmp.path(), "sess-retry").unwrap();
        let memory = fs::read_to_string(tmp.path().join("MEMORY.md")).unwrap();
        let dream_summary_count = memory
            .lines()
            .filter(|line| line.contains("Dream summary for completed session sess-retry"))
            .count();

        assert_eq!(report.sessions_processed, 1);
        assert_eq!(report.session_reports.len(), 1);
        assert_eq!(
            report.session_reports[0].artifact_refs,
            vec!["dream/session/sess-retry".to_string()]
        );
        assert_eq!(recovered.status, DreamSessionStatus::Completed);
        assert!(recovered.failure_reason.is_none());
        assert_eq!(dream_summary_count, 1);
    }

    #[test]
    fn busy_run_does_not_consume_pending_session_and_succeeds_after_lock_release() {
        let tmp = TempDir::new().unwrap();
        record_session_completion(tmp.path(), "sess-busy").unwrap();

        let busy_guard = DreamLockGuard::acquire(tmp.path()).unwrap().unwrap();
        let busy = run_now(tmp.path(), DreamTriggerReason::Manual)
            .unwrap()
            .unwrap();
        assert_eq!(busy.lock_state, DreamLockState::Busy);
        assert_eq!(busy.sessions_processed, 0);
        assert_eq!(
            dream_eligibility(tmp.path(), "sess-busy").unwrap(),
            DreamEligibility::Eligible
        );

        drop(busy_guard);

        let completed = run_now(tmp.path(), DreamTriggerReason::Manual)
            .unwrap()
            .unwrap();
        assert_eq!(completed.status, DreamRunStatus::Completed);
        assert_eq!(completed.sessions_processed, 1);
        assert_eq!(
            dream_eligibility(tmp.path(), "sess-busy").unwrap(),
            DreamEligibility::AlreadyConsolidated
        );
    }
}
