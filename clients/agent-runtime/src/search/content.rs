use crate::search::discovery::{
    discover_searchable_files_with_stats, is_binary_file, normalize_relative_path,
    DiscoveryRules as SearchDiscoveryRules,
};
use crate::search::{CandidateCoverage, CandidateRequest, WorkspaceTrigramIndex};
use crate::security::SecurityPolicy;
use anyhow::Context;
use regex::Regex;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_FILES_SCANNED: usize = 10_000;
const MAX_OUTPUT_BYTES: usize = 100 * 1024;
const MAX_MATCHES_PER_FILE: usize = 50;
const MAX_LINE_CONTENT_CHARS: usize = 500;
const EXECUTION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub(crate) struct SharedSearchRequest {
    pub pattern: String,
    pub path: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub is_regex: bool,
    pub case_sensitive: bool,
    pub max_results: usize,
    pub context_lines: usize,
    pub whole_word: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SearchLimits {
    pub max_file_size_bytes: u64,
    pub max_files_scanned: usize,
    pub max_output_bytes: usize,
    pub max_matches_per_file: usize,
    pub timeout: Duration,
    pub max_line_content_chars: usize,
}

pub(crate) const DEFAULT_LIMITS: SearchLimits = SearchLimits {
    max_file_size_bytes: MAX_FILE_SIZE_BYTES,
    max_files_scanned: MAX_FILES_SCANNED,
    max_output_bytes: MAX_OUTPUT_BYTES,
    max_matches_per_file: MAX_MATCHES_PER_FILE,
    timeout: EXECUTION_TIMEOUT,
    max_line_content_chars: MAX_LINE_CONTENT_CHARS,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SearchMatch {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub content: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    pub line_end: usize,
    pub column_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SearchStats {
    pub files_searched: usize,
    pub files_matched: usize,
    pub total_matches: usize,
    pub truncated: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchOutcome {
    pub matches: Vec<SearchMatch>,
    pub stats: SearchStats,
    pub warnings: Vec<String>,
    pub fatal_error: Option<String>,
}

#[derive(Debug, Clone)]
struct VerificationInput {
    relative_path: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct LineRange {
    start: usize,
    end: usize,
}

pub(crate) fn search_workspace(
    security: Arc<SecurityPolicy>,
    request: SharedSearchRequest,
    regex: Regex,
    limits: SearchLimits,
) -> SearchOutcome {
    let start = Instant::now();
    let mut warnings = Vec::new();
    let plan = WorkspaceTrigramIndex::for_workspace(&security.workspace_dir)
        .plan_candidates(
            &security,
            &CandidateRequest {
                relative_root: request.path.clone(),
                include: request.include.clone(),
                exclude: request.exclude.clone(),
                raw_pattern: request.pattern.clone(),
                is_regex: request.is_regex,
                case_sensitive: request.case_sensitive,
                whole_word: request.whole_word,
            },
            limits.max_file_size_bytes,
        )
        .unwrap_or_else(|_| crate::search::CandidatePlan {
            coverage: CandidateCoverage::Unavailable,
            ordered_paths: Vec::new(),
            reason: "index_planning_failed".to_string(),
        });

    let inputs = match build_verification_inputs(&security, &request, &plan, limits, &mut warnings)
    {
        Ok(inputs) => inputs,
        Err(error) => {
            return SearchOutcome {
                matches: Vec::new(),
                stats: SearchStats {
                    files_searched: 0,
                    files_matched: 0,
                    total_matches: 0,
                    truncated: false,
                    duration_ms: 0,
                },
                warnings: vec![error.to_string()],
                fatal_error: Some(error.to_string()),
            };
        }
    };

    verify_inputs(inputs, &request, &regex, limits, start, warnings)
}

fn build_verification_inputs(
    security: &SecurityPolicy,
    request: &SharedSearchRequest,
    plan: &crate::search::CandidatePlan,
    limits: SearchLimits,
    warnings: &mut Vec<String>,
) -> anyhow::Result<Vec<VerificationInput>> {
    match plan.coverage {
        CandidateCoverage::Complete => {
            match read_index_candidate_inputs(security, &plan.ordered_paths, limits) {
                Ok(inputs) => {
                    if inputs.len() >= limits.max_files_scanned
                        && plan.ordered_paths.len() > inputs.len()
                    {
                        push_warning(
                        warnings,
                        format!(
                            "Search stopped after {} files. Narrow your search with 'path' or 'include' filters.",
                            limits.max_files_scanned
                        ),
                    );
                    }
                    Ok(inputs)
                }
                Err(_) => discover_fallback_inputs(security, request, limits, warnings),
            }
        }
        CandidateCoverage::Partial | CandidateCoverage::Unavailable => {
            discover_fallback_inputs(security, request, limits, warnings)
        }
    }
}

fn discover_fallback_inputs(
    security: &SecurityPolicy,
    request: &SharedSearchRequest,
    limits: SearchLimits,
    warnings: &mut Vec<String>,
) -> anyhow::Result<Vec<VerificationInput>> {
    let discovered = discover_searchable_files_with_stats(
        security,
        &request.path,
        &request.include,
        &request.exclude,
        SearchDiscoveryRules {
            max_file_size_bytes: limits.max_file_size_bytes,
            max_files: Some(limits.max_files_scanned),
            ..SearchDiscoveryRules::default()
        },
    )?;

    if discovered.hit_max_files {
        push_warning(
            warnings,
            format!(
                "Search stopped after {} files. Narrow your search with 'path' or 'include' filters.",
                limits.max_files_scanned
            ),
        );
    }

    Ok(discovered
        .files
        .into_iter()
        .map(|entry| VerificationInput {
            relative_path: entry.file.relative_path,
            bytes: entry.bytes,
        })
        .collect())
}

fn read_index_candidate_inputs(
    security: &SecurityPolicy,
    ordered_paths: &[String],
    limits: SearchLimits,
) -> anyhow::Result<Vec<VerificationInput>> {
    let workspace_root = security
        .workspace_dir
        .canonicalize()
        .unwrap_or_else(|_| security.workspace_dir.clone());
    let mut inputs = Vec::new();

    for relative_path in ordered_paths.iter().take(limits.max_files_scanned) {
        let resolved_path = resolve_relative_file(security, &workspace_root, relative_path)?;
        let mut file = OpenOptions::new()
            .read(true)
            .open(&resolved_path)
            .with_context(|| format!("failed to open candidate file '{}'", relative_path))?;
        let metadata = file
            .metadata()
            .with_context(|| format!("failed to read candidate metadata '{}'", relative_path))?;
        if !metadata.is_file() || metadata.len() > limits.max_file_size_bytes {
            continue;
        }

        let mut bytes = Vec::with_capacity(metadata.len().try_into().unwrap_or(0));
        file.read_to_end(&mut bytes)
            .with_context(|| format!("failed to read candidate file '{}'", relative_path))?;
        if is_binary_file(&bytes) {
            continue;
        }

        inputs.push(VerificationInput {
            relative_path: relative_path.clone(),
            bytes,
        });
    }

    Ok(inputs)
}

fn resolve_relative_file(
    security: &SecurityPolicy,
    workspace_root: &Path,
    relative_path: &str,
) -> anyhow::Result<PathBuf> {
    if !security.is_path_allowed(relative_path) {
        anyhow::bail!("candidate path not allowed by security policy: {relative_path}");
    }

    let joined = security.workspace_dir.join(relative_path);
    let resolved = joined
        .canonicalize()
        .with_context(|| format!("failed to resolve candidate path '{}'", relative_path))?;
    anyhow::ensure!(
        security.is_resolved_path_allowed(&resolved),
        "candidate path escapes workspace: {}",
        resolved.display()
    );
    let normalized = normalize_relative_path(workspace_root, &resolved)?;
    anyhow::ensure!(
        normalized == relative_path,
        "candidate path changed during resolution: expected '{}' but got '{}'",
        relative_path,
        normalized
    );
    Ok(resolved)
}

fn verify_inputs(
    inputs: Vec<VerificationInput>,
    request: &SharedSearchRequest,
    regex: &Regex,
    limits: SearchLimits,
    start: Instant,
    mut warnings: Vec<String>,
) -> SearchOutcome {
    let mut files_searched = 0usize;
    let mut files_matched = 0usize;
    let mut total_matches = 0usize;
    let mut matches = Vec::new();
    let mut truncated = warnings
        .iter()
        .any(|warning| warning.starts_with("Search stopped after"));

    'files: for input in inputs {
        if start.elapsed() >= limits.timeout {
            truncated = true;
            push_warning(
                &mut warnings,
                format!(
                    "Search timed out after {}ms. Partial results returned.",
                    limits.timeout.as_millis()
                ),
            );
            break;
        }

        files_searched += 1;
        let file_matches = verify_file_matches(
            &input.relative_path,
            &input.bytes,
            regex,
            request.context_lines,
            limits.max_line_content_chars,
            limits.max_matches_per_file,
        );

        if !file_matches.is_empty() {
            files_matched += 1;
        }

        for matched in file_matches {
            if matches.len() >= request.max_results {
                truncated = true;
                push_warning(
                    &mut warnings,
                    format!(
                        "Results truncated at {} matches. Narrow your search with 'path' or 'include' filters.",
                        request.max_results
                    ),
                );
                break 'files;
            }

            total_matches += 1;
            matches.push(matched);
        }
    }

    SearchOutcome {
        matches,
        stats: SearchStats {
            files_searched,
            files_matched,
            total_matches,
            truncated,
            duration_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        },
        warnings,
        fatal_error: None,
    }
}

fn verify_file_matches(
    relative_path: &str,
    bytes: &[u8],
    regex: &Regex,
    context_lines: usize,
    max_line_content_chars: usize,
    max_matches_per_file: usize,
) -> Vec<SearchMatch> {
    let contents = String::from_utf8_lossy(bytes);
    let lines = line_ranges(&contents);
    let mut matches = Vec::new();

    for found in regex.find_iter(&contents) {
        if matches.len() >= max_matches_per_file {
            break;
        }

        let start_line_index = locate_line(&lines, found.start());
        let end_anchor = found
            .end()
            .saturating_sub(1)
            .min(contents.len().saturating_sub(1));
        let end_line_index = locate_line(&lines, end_anchor);
        let line = slice_line(&contents, lines[start_line_index]);
        let (context_before, context_after) =
            collect_context(&contents, &lines, start_line_index, context_lines);

        matches.push(SearchMatch {
            file: relative_path.to_string(),
            line: start_line_index + 1,
            column: found.start().saturating_sub(lines[start_line_index].start) + 1,
            content: truncate_chars(line, max_line_content_chars),
            context_before,
            context_after,
            line_end: end_line_index + 1,
            column_end: found.end().saturating_sub(lines[end_line_index].start) + 1,
            byte_start: found.start(),
            byte_end: found.end(),
            preview: truncate_chars(
                &contents[found.start()..found.end()],
                max_line_content_chars,
            ),
        });
    }

    matches.sort_by(|left, right| {
        left.byte_start
            .cmp(&right.byte_start)
            .then(left.byte_end.cmp(&right.byte_end))
    });
    matches
}

fn collect_context(
    contents: &str,
    lines: &[LineRange],
    line_index: usize,
    context_lines: usize,
) -> (Vec<String>, Vec<String>) {
    if context_lines == 0 {
        return (Vec::new(), Vec::new());
    }

    let before_start = line_index.saturating_sub(context_lines);
    let before = lines[before_start..line_index]
        .iter()
        .map(|line| slice_line(contents, *line).to_string())
        .collect();
    let after_end = (line_index + 1 + context_lines).min(lines.len());
    let after = lines[line_index + 1..after_end]
        .iter()
        .map(|line| slice_line(contents, *line).to_string())
        .collect();

    (before, after)
}

fn line_ranges(contents: &str) -> Vec<LineRange> {
    if contents.is_empty() {
        return vec![LineRange { start: 0, end: 0 }];
    }

    let bytes = contents.as_bytes();
    let mut ranges = Vec::new();
    let mut start = 0usize;

    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            let mut end = index;
            if end > start && bytes[end - 1] == b'\r' {
                end -= 1;
            }
            ranges.push(LineRange { start, end });
            start = index + 1;
        }
    }

    if start < bytes.len() {
        ranges.push(LineRange {
            start,
            end: bytes.len(),
        });
    }

    if ranges.is_empty() {
        ranges.push(LineRange { start: 0, end: 0 });
    }

    ranges
}

fn locate_line(lines: &[LineRange], byte_offset: usize) -> usize {
    lines
        .partition_point(|line| line.start <= byte_offset)
        .saturating_sub(1)
}

fn slice_line(contents: &str, line: LineRange) -> &str {
    &contents[line.start..line.end]
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn push_warning(warnings: &mut Vec<String>, warning: String) {
    if !warnings.iter().any(|existing| existing == &warning) {
        warnings.push(warning);
    }
}
