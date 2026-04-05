use anyhow::{Context, Result};
use corvus::runtime::NativeRuntime;
use corvus::search::{CandidateCoverage, CandidateRequest, WorkspaceTrigramIndex};
use corvus::security::{AutonomyLevel, NoopSandbox, SecurityPolicy};
use corvus::tools::traits::{Tool, ToolResult};
use corvus::tools::{CodeSearchTool, ShellTool};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const DEFAULT_SAMPLES: usize = 5;
const DEFAULT_COLD_BUILD_SAMPLES: usize = 2;
const DEFAULT_PATH: &str = ".";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryKind {
    Literal,
    Regex,
}

impl QueryKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Literal => "literal",
            Self::Regex => "regex",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultShape {
    Small,
    Large,
    Miss,
}

impl ResultShape {
    fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small-hit",
            Self::Large => "large-hit",
            Self::Miss => "no-hit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutionMode {
    ShellBaseline,
    NativeNoIndex,
    NativeColdBuild,
    NativeWarmIndex,
}

impl ExecutionMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::ShellBaseline => "shell_baseline",
            Self::NativeNoIndex => "native_no_index",
            Self::NativeColdBuild => "native_cold_build",
            Self::NativeWarmIndex => "native_warm_index",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlanMode {
    IndexedNarrowing,
    FallbackDiscoveryLiveVerification,
    IndexUnavailable,
}

impl PlanMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::IndexedNarrowing => "indexed_narrowing",
            Self::FallbackDiscoveryLiveVerification => "fallback_discovery_live_verification",
            Self::IndexUnavailable => "index_unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchmarkCase {
    id: &'static str,
    query_kind: QueryKind,
    result_shape: ResultShape,
    pattern: &'static str,
    path: &'static str,
    case_sensitive: bool,
    whole_word: bool,
}

impl BenchmarkCase {
    fn is_regex(&self) -> bool {
        self.query_kind == QueryKind::Regex
    }

    fn tool_args(&self) -> Value {
        let mut args = json!({
            "pattern": self.pattern,
            "is_regex": self.is_regex(),
            "case_sensitive": self.case_sensitive,
            "whole_word": self.whole_word,
            "path": self.path,
            "max_results": 500,
        });
        if self.path == DEFAULT_PATH {
            args.as_object_mut().unwrap().remove("path");
        }
        args
    }

    fn candidate_request(&self) -> CandidateRequest {
        CandidateRequest {
            relative_root: self.path.to_string(),
            include: Vec::new(),
            exclude: Vec::new(),
            raw_pattern: self.pattern.to_string(),
            is_regex: self.is_regex(),
            case_sensitive: self.case_sensitive,
            whole_word: self.whole_word,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CanonicalLineMatch {
    file: String,
    line: usize,
    content: String,
}

#[derive(Debug, Clone)]
struct BenchmarkMeasurement {
    case_id: String,
    execution_mode: ExecutionMode,
    plan_mode: Option<PlanMode>,
    plan_reason: String,
    samples: usize,
    median_ms: u64,
    p95_ms: u64,
    build_median_ms: Option<u64>,
    search_median_ms: Option<u64>,
    total_median_ms: Option<u64>,
    parity_passed: Option<bool>,
}

#[derive(Debug, Clone)]
struct WorkspaceReport {
    metadata: EnvironmentMetadata,
    matrix: Vec<BenchmarkCase>,
    measurements: Vec<BenchmarkMeasurement>,
}

#[derive(Debug, Clone)]
struct EnvironmentMetadata {
    workspace_label: String,
    workspace_kind: &'static str,
    file_count: usize,
    os: String,
    arch: String,
    cpu: String,
    rust_profile: &'static str,
    benchmarked_at: String,
    commit_sha: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceSelection {
    Fixture,
    Repo,
    Both,
}

#[derive(Debug, Clone)]
struct CliArgs {
    workspace: WorkspaceSelection,
    repo_path: PathBuf,
    samples: usize,
    cold_build_samples: usize,
}

#[derive(Debug)]
struct WorkspaceContext {
    label: String,
    kind: &'static str,
    root: PathBuf,
    fixture_guard: Option<TempDir>,
    cases: Vec<BenchmarkCase>,
}

#[derive(Debug, Clone)]
struct ShellExecutionSummary {
    canonical: Vec<CanonicalLineMatch>,
    durations: Vec<Duration>,
}

#[derive(Debug, Clone)]
struct NativeExecutionSummary {
    canonical: Vec<CanonicalLineMatch>,
    search_durations: Vec<Duration>,
    build_durations: Vec<Duration>,
    total_durations: Vec<Duration>,
    plan_mode: PlanMode,
    plan_reason: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = parse_args(env::args().skip(1))?;
    let mut reports = Vec::new();

    if matches!(
        args.workspace,
        WorkspaceSelection::Fixture | WorkspaceSelection::Both
    ) {
        let workspace = create_fixture_workspace()?;
        reports.push(run_workspace_suite(&workspace, args.samples, args.cold_build_samples).await?);
    }

    if matches!(
        args.workspace,
        WorkspaceSelection::Repo | WorkspaceSelection::Both
    ) {
        let workspace = repo_workspace_context(&args.repo_path)?;
        reports.push(run_workspace_suite(&workspace, args.samples, args.cold_build_samples).await?);
    }

    for report in reports {
        print_workspace_report(&report);
    }

    Ok(())
}

fn parse_args<I>(mut args: I) -> Result<CliArgs>
where
    I: Iterator<Item = String>,
{
    let mut workspace = WorkspaceSelection::Both;
    let mut repo_path = repo_root_from_manifest()?;
    let mut samples = DEFAULT_SAMPLES;
    let mut cold_build_samples = DEFAULT_COLD_BUILD_SAMPLES;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--workspace" => {
                let value = args.next().context("missing value for --workspace")?;
                workspace = match value.as_str() {
                    "fixture" => WorkspaceSelection::Fixture,
                    "repo" => WorkspaceSelection::Repo,
                    "both" => WorkspaceSelection::Both,
                    other => anyhow::bail!("unsupported --workspace value '{other}'"),
                };
            }
            "--repo-path" => {
                repo_path = PathBuf::from(args.next().context("missing value for --repo-path")?);
            }
            "--samples" => {
                samples = parse_positive_usize(
                    &args.next().context("missing value for --samples")?,
                    "--samples",
                )?;
            }
            "--cold-build-samples" => {
                cold_build_samples = parse_positive_usize(
                    &args
                        .next()
                        .context("missing value for --cold-build-samples")?,
                    "--cold-build-samples",
                )?;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown argument '{other}'"),
        }
    }

    Ok(CliArgs {
        workspace,
        repo_path,
        samples,
        cold_build_samples,
    })
}

fn print_help() {
    println!(
        "code_search rollout benchmark\n\n\
Usage:\n  cargo run --example code_search_rollout_benchmark -- [options]\n\n\
Options:\n  --workspace <fixture|repo|both>\n  --repo-path <path>\n  --samples <n>\n  --cold-build-samples <n>\n"
    );
}

fn parse_positive_usize(raw: &str, flag: &str) -> Result<usize> {
    let value = raw
        .parse::<usize>()
        .with_context(|| format!("invalid {flag} value '{raw}'"))?;
    anyhow::ensure!(value > 0, "{flag} must be > 0");
    Ok(value)
}

fn repo_root_from_manifest() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("failed to derive repo root from CARGO_MANIFEST_DIR")
}

fn fixture_cases() -> Vec<BenchmarkCase> {
    vec![
        BenchmarkCase {
            id: "literal_small_hit",
            query_kind: QueryKind::Literal,
            result_shape: ResultShape::Small,
            pattern: "fixture_small_literal_unique",
            path: "src",
            case_sensitive: true,
            whole_word: false,
        },
        BenchmarkCase {
            id: "literal_large_hit",
            query_kind: QueryKind::Literal,
            result_shape: ResultShape::Large,
            pattern: "fixture_large_literal_shared",
            path: "src",
            case_sensitive: true,
            whole_word: false,
        },
        BenchmarkCase {
            id: "literal_no_hit",
            query_kind: QueryKind::Literal,
            result_shape: ResultShape::Miss,
            pattern: "fixture_literal_rollout_no_hit",
            path: "src",
            case_sensitive: true,
            whole_word: false,
        },
        BenchmarkCase {
            id: "regex_small_hit",
            query_kind: QueryKind::Regex,
            result_shape: ResultShape::Small,
            pattern: "fixture_regex_unique_.+",
            path: "src",
            case_sensitive: true,
            whole_word: false,
        },
        BenchmarkCase {
            id: "regex_large_hit",
            query_kind: QueryKind::Regex,
            result_shape: ResultShape::Large,
            pattern: "fixture_regex_bulk_case_.+",
            path: "src",
            case_sensitive: true,
            whole_word: false,
        },
        BenchmarkCase {
            id: "regex_no_hit",
            query_kind: QueryKind::Regex,
            result_shape: ResultShape::Miss,
            pattern: "fixture_regex_rollout_no_match_.+",
            path: "src",
            case_sensitive: true,
            whole_word: false,
        },
    ]
}

fn repo_cases() -> Vec<BenchmarkCase> {
    vec![
        BenchmarkCase {
            id: "literal_small_hit",
            query_kind: QueryKind::Literal,
            result_shape: ResultShape::Small,
            pattern: "pub struct ToolResult",
            path: "clients/agent-runtime/src/tools",
            case_sensitive: true,
            whole_word: false,
        },
        BenchmarkCase {
            id: "literal_large_hit",
            query_kind: QueryKind::Literal,
            result_shape: ResultShape::Large,
            pattern: "success:",
            path: "clients/agent-runtime/src/tools",
            case_sensitive: true,
            whole_word: false,
        },
        BenchmarkCase {
            id: "literal_no_hit",
            query_kind: QueryKind::Literal,
            result_shape: ResultShape::Miss,
            pattern: "code_search_rollout_literal_no_match_20260405",
            path: "clients/agent-runtime/src/tools",
            case_sensitive: true,
            whole_word: false,
        },
        BenchmarkCase {
            id: "regex_small_hit",
            query_kind: QueryKind::Regex,
            result_shape: ResultShape::Small,
            pattern: "pub +struct +ToolResult",
            path: "clients/agent-runtime/src/tools",
            case_sensitive: true,
            whole_word: false,
        },
        BenchmarkCase {
            id: "regex_large_hit",
            query_kind: QueryKind::Regex,
            result_shape: ResultShape::Large,
            pattern: "output( .+)? *:",
            path: "clients/agent-runtime/src/tools",
            case_sensitive: true,
            whole_word: false,
        },
        BenchmarkCase {
            id: "regex_no_hit",
            query_kind: QueryKind::Regex,
            result_shape: ResultShape::Miss,
            pattern: "code_search_rollout_regex_no_match_.+",
            path: "clients/agent-runtime/src/tools",
            case_sensitive: true,
            whole_word: false,
        },
    ]
}

fn create_fixture_workspace() -> Result<WorkspaceContext> {
    let fixture_guard = TempDir::new().context("failed to create fixture tempdir")?;
    let root = fixture_guard.path();
    fs::create_dir_all(root.join("src")).context("failed to create fixture src")?;
    fs::create_dir_all(root.join("docs")).context("failed to create fixture docs")?;

    fs::write(
        root.join("src/small.rs"),
        [
            "fn fixture_regex_unique_target() {",
            "  let token = \"fixture_small_literal_unique\";",
            "}",
        ]
        .join("\n")
            + "\n",
    )
    .context("failed to write fixture small.rs")?;

    let mut bulk = String::new();
    for index in 0..24 {
        bulk.push_str(&format!(
            "fn fixture_regex_bulk_case_{index}() {{ let token = \"fixture_large_literal_shared\"; }}\n"
        ));
    }
    fs::write(root.join("src/bulk.rs"), bulk).context("failed to write fixture bulk.rs")?;
    fs::write(
        root.join("src/noise.txt"),
        "this file is searched too\nfixture_large_literal_shared appears once here\n",
    )
    .context("failed to write fixture noise.txt")?;
    fs::write(root.join("docs/notes.md"), "documentation only\n")
        .context("failed to write fixture docs")?;

    Ok(WorkspaceContext {
        label: "deterministic fixture".to_string(),
        kind: "fixture",
        root: root.to_path_buf(),
        fixture_guard: Some(fixture_guard),
        cases: fixture_cases(),
    })
}

fn repo_workspace_context(repo_path: &Path) -> Result<WorkspaceContext> {
    let root = repo_path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize repo path '{}'", repo_path.display()))?;
    anyhow::ensure!(
        root.exists(),
        "repo path '{}' does not exist",
        root.display()
    );
    Ok(WorkspaceContext {
        label: "current repo snapshot".to_string(),
        kind: "repo_snapshot",
        root,
        fixture_guard: None,
        cases: repo_cases(),
    })
}

async fn run_workspace_suite(
    workspace: &WorkspaceContext,
    samples: usize,
    cold_build_samples: usize,
) -> Result<WorkspaceReport> {
    let _fixture_guard = workspace.fixture_guard.as_ref();
    let metadata = capture_environment_metadata(workspace)?;
    let security = benchmark_security(&workspace.root);
    let shell = benchmark_shell_tool(security.clone());
    let code_search = CodeSearchTool::new(security.clone());
    let index = WorkspaceTrigramIndex::for_workspace(&workspace.root);
    let mut measurements = Vec::new();

    for case in &workspace.cases {
        let shell_summary = run_shell_baseline(&shell, case, samples).await?;
        measurements.push(BenchmarkMeasurement {
            case_id: case.id.to_string(),
            execution_mode: ExecutionMode::ShellBaseline,
            plan_mode: None,
            plan_reason: "shell_grep_baseline".to_string(),
            samples,
            median_ms: percentile_ms(&shell_summary.durations, 50),
            p95_ms: percentile_ms(&shell_summary.durations, 95),
            build_median_ms: None,
            search_median_ms: None,
            total_median_ms: None,
            parity_passed: None,
        });

        let no_index =
            run_native_no_index(&code_search, &index, security.as_ref(), case, samples).await?;
        measurements.push(as_measurement(
            case.id,
            ExecutionMode::NativeNoIndex,
            &no_index,
            &shell_summary.canonical,
        ));

        let cold = run_native_cold_build(
            &code_search,
            &index,
            security.as_ref(),
            case,
            cold_build_samples,
        )
        .await?;
        measurements.push(as_measurement(
            case.id,
            ExecutionMode::NativeColdBuild,
            &cold,
            &shell_summary.canonical,
        ));

        let warm =
            run_native_warm_index(&code_search, &index, security.as_ref(), case, samples).await?;
        measurements.push(as_measurement(
            case.id,
            ExecutionMode::NativeWarmIndex,
            &warm,
            &shell_summary.canonical,
        ));
    }

    Ok(WorkspaceReport {
        metadata,
        matrix: workspace.cases.clone(),
        measurements,
    })
}

fn as_measurement(
    case_id: &str,
    execution_mode: ExecutionMode,
    native: &NativeExecutionSummary,
    shell_canonical: &[CanonicalLineMatch],
) -> BenchmarkMeasurement {
    BenchmarkMeasurement {
        case_id: case_id.to_string(),
        execution_mode,
        plan_mode: Some(native.plan_mode),
        plan_reason: native.plan_reason.clone(),
        samples: native.search_durations.len(),
        median_ms: percentile_ms(&native.total_durations, 50),
        p95_ms: percentile_ms(&native.total_durations, 95),
        build_median_ms: (!native.build_durations.is_empty())
            .then(|| percentile_ms(&native.build_durations, 50)),
        search_median_ms: Some(percentile_ms(&native.search_durations, 50)),
        total_median_ms: Some(percentile_ms(&native.total_durations, 50)),
        parity_passed: Some(native.canonical == shell_canonical),
    }
}

fn benchmark_security(workspace: &Path) -> Arc<SecurityPolicy> {
    let mut policy = SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: workspace.to_path_buf(),
        max_actions_per_hour: 1_000_000,
        ..SecurityPolicy::default()
    };
    if !policy
        .allowed_commands
        .iter()
        .any(|command| command == "true")
    {
        policy.allowed_commands.push("true".to_string());
    }
    Arc::new(policy)
}

fn benchmark_shell_tool(security: Arc<SecurityPolicy>) -> ShellTool {
    ShellTool::new(
        security,
        Arc::new(NativeRuntime::new()),
        Arc::new(NoopSandbox),
    )
}

async fn run_shell_baseline(
    shell: &ShellTool,
    case: &BenchmarkCase,
    samples: usize,
) -> Result<ShellExecutionSummary> {
    let mut durations = Vec::with_capacity(samples);
    let warmup = run_shell_once(shell, case).await?;
    let expected = warmup.canonical;

    for _ in 0..samples {
        let run = run_shell_once(shell, case).await?;
        anyhow::ensure!(
            run.canonical == expected,
            "shell results drifted for case {}",
            case.id
        );
        durations.push(run.duration);
    }

    Ok(ShellExecutionSummary {
        canonical: expected,
        durations,
    })
}

struct ShellRun {
    canonical: Vec<CanonicalLineMatch>,
    duration: Duration,
}

async fn run_shell_once(shell: &ShellTool, case: &BenchmarkCase) -> Result<ShellRun> {
    let command = build_grep_command(case);
    let started = Instant::now();
    let result = shell
        .execute(json!({ "command": command, "approved": true }))
        .await
        .context("shell tool execution failed")?;
    let duration = started.elapsed();
    anyhow::ensure!(
        result.success,
        "shell baseline failed for case {}: {:?}",
        case.id,
        result.error
    );

    Ok(ShellRun {
        canonical: canonicalize_shell_output(&result.output)?,
        duration,
    })
}

async fn run_native_no_index(
    code_search: &CodeSearchTool,
    index: &WorkspaceTrigramIndex,
    security: &SecurityPolicy,
    case: &BenchmarkCase,
    samples: usize,
) -> Result<NativeExecutionSummary> {
    let mut search_durations = Vec::with_capacity(samples);
    let mut total_durations = Vec::with_capacity(samples);

    clear_index_artifacts(index)?;
    let warmup = run_native_search_once(code_search, index, security, case, false).await?;
    let canonical = warmup.canonical;
    let plan_mode = warmup.plan_mode;
    let plan_reason = warmup.plan_reason;

    for _ in 0..samples {
        clear_index_artifacts(index)?;
        let run = run_native_search_once(code_search, index, security, case, false).await?;
        anyhow::ensure!(
            run.canonical == canonical.as_slice(),
            "native no-index parity drift for case {}",
            case.id
        );
        anyhow::ensure!(
            run.plan_mode == plan_mode,
            "native no-index plan mode drift for case {}",
            case.id
        );
        anyhow::ensure!(
            run.plan_reason == plan_reason,
            "native no-index plan reason drift for case {}",
            case.id
        );
        search_durations.push(run.search_duration);
        total_durations.push(run.total_duration);
    }

    Ok(NativeExecutionSummary {
        canonical,
        search_durations,
        build_durations: Vec::new(),
        total_durations,
        plan_mode,
        plan_reason,
    })
}

async fn run_native_cold_build(
    code_search: &CodeSearchTool,
    index: &WorkspaceTrigramIndex,
    security: &SecurityPolicy,
    case: &BenchmarkCase,
    samples: usize,
) -> Result<NativeExecutionSummary> {
    let mut build_durations = Vec::with_capacity(samples);
    let mut search_durations = Vec::with_capacity(samples);
    let mut total_durations = Vec::with_capacity(samples);

    clear_index_artifacts(index)?;
    let warmup = run_native_search_once(code_search, index, security, case, true).await?;
    let canonical = warmup.canonical;
    let plan_mode = warmup.plan_mode;
    let plan_reason = warmup.plan_reason;

    for _ in 0..samples {
        clear_index_artifacts(index)?;
        let run = run_native_search_once(code_search, index, security, case, true).await?;
        anyhow::ensure!(
            run.canonical == canonical.as_slice(),
            "native cold-build parity drift for case {}",
            case.id
        );
        anyhow::ensure!(
            run.plan_mode == plan_mode,
            "native cold-build plan mode drift for case {}",
            case.id
        );
        anyhow::ensure!(
            run.plan_reason == plan_reason,
            "native cold-build plan reason drift for case {}",
            case.id
        );
        build_durations.push(run.build_duration.unwrap_or_default());
        search_durations.push(run.search_duration);
        total_durations.push(run.total_duration);
    }

    Ok(NativeExecutionSummary {
        canonical,
        search_durations,
        build_durations,
        total_durations,
        plan_mode,
        plan_reason,
    })
}

async fn run_native_warm_index(
    code_search: &CodeSearchTool,
    index: &WorkspaceTrigramIndex,
    security: &SecurityPolicy,
    case: &BenchmarkCase,
    samples: usize,
) -> Result<NativeExecutionSummary> {
    let security_arc = Arc::new(security.clone());
    index
        .refresh_or_rebuild(security_arc)
        .context("failed to prepare warm index")?;

    let mut search_durations = Vec::with_capacity(samples);
    let mut total_durations = Vec::with_capacity(samples);
    let warmup = run_native_search_once(code_search, index, security, case, false).await?;
    let expected = warmup.canonical;
    let expected_plan_mode = warmup.plan_mode;
    let expected_plan_reason = warmup.plan_reason;

    for _ in 0..samples {
        let run = run_native_search_once(code_search, index, security, case, false).await?;
        anyhow::ensure!(
            run.canonical == expected,
            "native warm-index parity drift for case {}",
            case.id
        );
        anyhow::ensure!(
            run.plan_mode == expected_plan_mode,
            "native warm-index plan mode drift for case {}",
            case.id
        );
        anyhow::ensure!(
            run.plan_reason == expected_plan_reason,
            "native warm-index plan reason drift for case {}",
            case.id
        );
        search_durations.push(run.search_duration);
        total_durations.push(run.total_duration);
    }

    Ok(NativeExecutionSummary {
        canonical: expected,
        search_durations,
        build_durations: Vec::new(),
        total_durations,
        plan_mode: expected_plan_mode,
        plan_reason: expected_plan_reason,
    })
}

struct NativeRun {
    canonical: Vec<CanonicalLineMatch>,
    build_duration: Option<Duration>,
    search_duration: Duration,
    total_duration: Duration,
    plan_mode: PlanMode,
    plan_reason: String,
}

async fn run_native_search_once(
    code_search: &CodeSearchTool,
    index: &WorkspaceTrigramIndex,
    security: &SecurityPolicy,
    case: &BenchmarkCase,
    build_before_search: bool,
) -> Result<NativeRun> {
    let total_started = Instant::now();
    let build_duration = if build_before_search {
        let build_started = Instant::now();
        index
            .refresh_or_rebuild(Arc::new(security.clone()))
            .context("failed to build index for cold benchmark")?;
        Some(build_started.elapsed())
    } else {
        None
    };

    let plan = index
        .plan_candidates(security, &case.candidate_request(), 10 * 1024 * 1024)
        .context("candidate planning failed")?;
    let plan_mode = label_plan_mode(plan.coverage.clone(), &plan.reason);
    let plan_reason = plan.reason;

    let search_started = Instant::now();
    let result = code_search
        .execute(case.tool_args())
        .await
        .context("code_search execution failed")?;
    let search_duration = search_started.elapsed();
    let total_duration = total_started.elapsed();

    anyhow::ensure!(
        result.success,
        "code_search failed for case {}: {:?}",
        case.id,
        result.error
    );

    Ok(NativeRun {
        canonical: canonicalize_native_result(&result)?,
        build_duration,
        search_duration,
        total_duration,
        plan_mode,
        plan_reason,
    })
}

fn build_grep_command(case: &BenchmarkCase) -> String {
    let mut parts = vec![
        "grep".to_string(),
        "-R".to_string(),
        "-n".to_string(),
        "-H".to_string(),
    ];
    parts.push(match case.query_kind {
        QueryKind::Literal => "-F".to_string(),
        QueryKind::Regex => "-E".to_string(),
    });
    if !case.case_sensitive {
        parts.push("-i".to_string());
    }
    if case.whole_word {
        parts.push("-w".to_string());
    }
    parts.push("-e".to_string());
    parts.push(shell_quote(case.pattern));
    parts.push("--".to_string());
    parts.push(shell_quote(case.path));

    let grep = parts.join(" ");
    format!("{grep} || true")
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn label_plan_mode(coverage: CandidateCoverage, reason: &str) -> PlanMode {
    match coverage {
        CandidateCoverage::Complete => PlanMode::IndexedNarrowing,
        CandidateCoverage::Partial => PlanMode::FallbackDiscoveryLiveVerification,
        CandidateCoverage::Unavailable if reason == "index_unavailable" => {
            PlanMode::IndexUnavailable
        }
        CandidateCoverage::Unavailable => PlanMode::FallbackDiscoveryLiveVerification,
    }
}

fn canonicalize_shell_output(output: &str) -> Result<Vec<CanonicalLineMatch>> {
    let mut seen = BTreeMap::<(String, usize), String>::new();

    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let mut parts = line.splitn(3, ':');
        let file = parts
            .next()
            .context("shell output missing file")?
            .trim_start_matches("./")
            .to_string();
        let line_number = parts
            .next()
            .context("shell output missing line number")?
            .parse::<usize>()
            .with_context(|| format!("invalid shell line number in '{line}'"))?;
        let content = parts
            .next()
            .context("shell output missing content")?
            .to_string();
        insert_canonical_line(&mut seen, file, line_number, content)?;
    }

    Ok(seen
        .into_iter()
        .map(|((file, line), content)| CanonicalLineMatch {
            file,
            line,
            content,
        })
        .collect())
}

fn canonicalize_native_result(result: &ToolResult) -> Result<Vec<CanonicalLineMatch>> {
    let structured = result
        .structured
        .as_ref()
        .context("code_search result missing structured payload")?;
    let matches = structured["matches"]
        .as_array()
        .context("code_search structured payload missing matches array")?;
    let mut seen = BTreeMap::<(String, usize), String>::new();

    for entry in matches {
        let file = entry["file"]
            .as_str()
            .context("native match missing file")?
            .trim_start_matches("./")
            .to_string();
        let line = entry["line"]
            .as_u64()
            .context("native match missing line")? as usize;
        let content = entry["content"]
            .as_str()
            .context("native match missing content")?
            .to_string();
        insert_canonical_line(&mut seen, file, line, content)?;
    }

    Ok(seen
        .into_iter()
        .map(|((file, line), content)| CanonicalLineMatch {
            file,
            line,
            content,
        })
        .collect())
}

fn insert_canonical_line(
    seen: &mut BTreeMap<(String, usize), String>,
    file: String,
    line: usize,
    content: String,
) -> Result<()> {
    match seen.get(&(file.clone(), line)) {
        Some(existing) if existing == &content => Ok(()),
        Some(existing) => anyhow::bail!(
            "conflicting canonical content for {}:{}: {:?} != {:?}",
            file,
            line,
            existing,
            content
        ),
        None => {
            seen.insert((file, line), content);
            Ok(())
        }
    }
}

fn percentile_ms(durations: &[Duration], percentile: usize) -> u64 {
    if durations.is_empty() {
        return 0;
    }
    let mut values: Vec<f64> = durations
        .iter()
        .map(|duration| duration.as_millis() as f64)
        .collect();
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

    let rank = (percentile as f64 / 100.0) * (values.len().saturating_sub(1) as f64);
    let lower_index = rank.floor() as usize;
    let upper_index = rank.ceil() as usize;
    let weight = rank - lower_index as f64;
    let interpolated = if lower_index == upper_index {
        values[lower_index]
    } else {
        values[lower_index] + (values[upper_index] - values[lower_index]) * weight
    };

    interpolated.round().clamp(0.0, u64::MAX as f64) as u64
}

fn clear_index_artifacts(index: &WorkspaceTrigramIndex) -> Result<()> {
    let state_dir = index
        .db_path()
        .parent()
        .context("index path missing state directory")?;
    if !state_dir.exists() {
        return Ok(());
    }

    for suffix in ["", "-shm", "-wal"] {
        let candidate = index.db_path().with_file_name(format!("index.db{suffix}"));
        if candidate.exists() {
            fs::remove_file(&candidate)
                .with_context(|| format!("failed to remove '{}'", candidate.display()))?;
        }
    }
    Ok(())
}

fn capture_environment_metadata(workspace: &WorkspaceContext) -> Result<EnvironmentMetadata> {
    Ok(EnvironmentMetadata {
        workspace_label: workspace.label.clone(),
        workspace_kind: workspace.kind,
        file_count: count_files(&workspace.root)?,
        os: env::consts::OS.to_string(),
        arch: env::consts::ARCH.to_string(),
        cpu: detect_cpu_descriptor(),
        rust_profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        benchmarked_at: chrono::Utc::now().to_rfc3339(),
        commit_sha: if workspace.kind == "repo_snapshot" {
            git_commit_sha(&workspace.root)
        } else {
            None
        },
    })
}

fn count_files(root: &Path) -> Result<usize> {
    let mut stack = vec![root.to_path_buf()];
    let mut count = 0_usize;
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path)
            .with_context(|| format!("failed to read directory '{}'", path.display()))?
        {
            let entry = entry?;
            let entry_path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(entry_path);
            } else {
                count += 1;
            }
        }
    }
    Ok(count)
}

fn detect_cpu_descriptor() -> String {
    if cfg!(target_os = "macos") {
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
        {
            if output.status.success() {
                let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !value.is_empty() {
                    return value;
                }
            }
        }
    }

    match std::thread::available_parallelism() {
        Ok(parallelism) => format!("{} logical CPUs", parallelism.get()),
        Err(_) => "unknown".to_string(),
    }
}

fn git_commit_sha(root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn print_workspace_report(report: &WorkspaceReport) {
    println!(
        "# code_search rollout benchmark — {}",
        report.metadata.workspace_label
    );
    println!();
    println!("- workspace_kind: {}", report.metadata.workspace_kind);
    println!("- workspace_root: <redacted>");
    println!("- file_count: {}", report.metadata.file_count);
    println!("- os: {}", report.metadata.os);
    println!("- arch: {}", report.metadata.arch);
    println!("- cpu: {}", report.metadata.cpu);
    println!("- rust_profile: {}", report.metadata.rust_profile);
    println!("- benchmarked_at: {}", report.metadata.benchmarked_at);
    println!(
        "- commit_sha: {}",
        report
            .metadata
            .commit_sha
            .clone()
            .unwrap_or_else(|| "n/a".to_string())
    );
    println!();
    println!("## Benchmark matrix");
    println!();
    println!("| Case | Query kind | Result shape | Path | Pattern |");
    println!("| --- | --- | --- | --- | --- |");
    for case in &report.matrix {
        println!(
            "| {} | {} | {} | `{}` | `{}` |",
            case.id,
            case.query_kind.as_str(),
            case.result_shape.as_str(),
            case.path,
            case.pattern.replace('`', "\\`")
        );
    }
    println!();
    println!("## Measurements");
    println!();
    println!("| Case | Mode | Plan mode | Plan reason | Samples | Median ms | P95 ms | Build median ms | Search median ms | Total median ms | Parity |");
    println!("| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |");
    for measurement in &report.measurements {
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            measurement.case_id,
            measurement.execution_mode.as_str(),
            measurement.plan_mode.map(PlanMode::as_str).unwrap_or("—"),
            if measurement.plan_reason.is_empty() {
                "—"
            } else {
                &measurement.plan_reason
            },
            measurement.samples,
            measurement.median_ms,
            measurement.p95_ms,
            measurement
                .build_median_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "—".to_string()),
            measurement
                .search_median_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "—".to_string()),
            measurement
                .total_median_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "—".to_string()),
            match measurement.parity_passed {
                Some(true) => "pass",
                Some(false) => "FAIL",
                None => "baseline",
            }
        );
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_grep_command_is_deterministic_for_literal_cases() {
        let case = BenchmarkCase {
            id: "literal_small_hit",
            query_kind: QueryKind::Literal,
            result_shape: ResultShape::Small,
            pattern: "needle",
            path: "src",
            case_sensitive: true,
            whole_word: false,
        };

        assert_eq!(
            build_grep_command(&case),
            "grep -R -n -H -F -e 'needle' -- 'src' || true"
        );
    }

    #[test]
    fn build_grep_command_is_deterministic_for_regex_cases() {
        let case = BenchmarkCase {
            id: "regex_small_hit",
            query_kind: QueryKind::Regex,
            result_shape: ResultShape::Small,
            pattern: "output:",
            path: "src/lib",
            case_sensitive: false,
            whole_word: true,
        };

        assert_eq!(
            build_grep_command(&case),
            "grep -R -n -H -E -i -w -e 'output:' -- 'src/lib' || true"
        );
    }

    #[test]
    fn percentile_ms_interpolates_between_neighboring_samples() {
        let durations = [
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
        ];

        assert_eq!(percentile_ms(&durations, 95), 39);
        assert_eq!(percentile_ms(&durations, 50), 25);
    }

    #[test]
    fn label_plan_mode_marks_regex_fallback_after_index_build() {
        assert_eq!(
            label_plan_mode(CandidateCoverage::Unavailable, "query_regex_not_supported"),
            PlanMode::FallbackDiscoveryLiveVerification
        );
        assert_eq!(
            label_plan_mode(CandidateCoverage::Unavailable, "index_unavailable"),
            PlanMode::IndexUnavailable
        );
        assert_eq!(
            label_plan_mode(CandidateCoverage::Complete, "indexed_candidates_complete"),
            PlanMode::IndexedNarrowing
        );
    }

    #[test]
    fn canonicalize_native_result_deduplicates_same_line_entries() {
        let result = ToolResult {
            success: true,
            output: String::new(),
            error: None,
            structured: Some(json!({
                "matches": [
                    { "file": "src/main.rs", "line": 7, "content": "let token = \"needle\";" },
                    { "file": "src/main.rs", "line": 7, "content": "let token = \"needle\";" },
                    { "file": "src/main.rs", "line": 9, "content": "needle again" }
                ]
            })),
        };

        let canonical = canonicalize_native_result(&result).unwrap();
        assert_eq!(canonical.len(), 2);
        assert_eq!(canonical[0].line, 7);
        assert_eq!(canonical[1].line, 9);
    }

    #[test]
    fn canonicalize_native_result_rejects_conflicting_duplicate_lines() {
        let result = ToolResult {
            success: true,
            output: String::new(),
            error: None,
            structured: Some(json!({
                "matches": [
                    { "file": "src/main.rs", "line": 7, "content": "first" },
                    { "file": "src/main.rs", "line": 7, "content": "second" }
                ]
            })),
        };

        let error = canonicalize_native_result(&result).unwrap_err();
        assert!(format!("{error:#}").contains("conflicting canonical content"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fixture_smoke_case_has_shell_native_parity_and_measurements() {
        let workspace = create_fixture_workspace().unwrap();
        let security = benchmark_security(&workspace.root);
        let shell = benchmark_shell_tool(security.clone());
        let code_search = CodeSearchTool::new(security.clone());
        let index = WorkspaceTrigramIndex::for_workspace(&workspace.root);
        let case = fixture_cases()
            .into_iter()
            .find(|candidate| candidate.id == "literal_small_hit")
            .unwrap();

        let shell_summary = run_shell_baseline(&shell, &case, 1).await.unwrap();
        assert!(!shell_summary.canonical.is_empty());
        assert_eq!(shell_summary.durations.len(), 1);

        let no_index = run_native_no_index(&code_search, &index, security.as_ref(), &case, 1)
            .await
            .unwrap();
        assert_eq!(no_index.canonical, shell_summary.canonical);
        assert_eq!(no_index.search_durations.len(), 1);

        let cold = run_native_cold_build(&code_search, &index, security.as_ref(), &case, 1)
            .await
            .unwrap();
        assert_eq!(cold.canonical, shell_summary.canonical);
        assert_eq!(cold.build_durations.len(), 1);
        assert_eq!(cold.search_durations.len(), 1);

        let warm = run_native_warm_index(&code_search, &index, security.as_ref(), &case, 1)
            .await
            .unwrap();
        assert_eq!(warm.canonical, shell_summary.canonical);
        assert_eq!(warm.search_durations.len(), 1);
    }
}
