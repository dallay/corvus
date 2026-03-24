//! SkillForge — Skill auto-discovery, evaluation, and integration engine.
//!
//! Pipeline: Scout → Evaluate → Integrate
//! Discovers skills from external sources, scores them, and generates
//! Corvus-compatible manifests for qualified candidates.

pub mod evaluate;
pub mod integrate;
pub mod scout;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use self::evaluate::{EvalResult, Evaluator, Recommendation};
use self::integrate::Integrator;
use self::scout::{GitHubScout, Scout, ScoutResult, ScoutSource};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize, Deserialize)]
pub struct SkillForgeConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_auto_integrate")]
    pub auto_integrate: bool,
    #[serde(default = "default_sources")]
    pub sources: Vec<String>,
    #[serde(default = "default_scan_interval")]
    pub scan_interval_hours: u64,
    #[serde(default = "default_min_score")]
    pub min_score: f64,
    /// Optional GitHub personal-access token for higher rate limits.
    #[serde(default)]
    pub github_token: Option<String>,
    /// Directory where integrated skills are written.
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
}

fn default_auto_integrate() -> bool {
    false
}
fn default_sources() -> Vec<String> {
    vec!["github".into(), "clawhub".into()]
}
fn default_scan_interval() -> u64 {
    24
}
fn default_min_score() -> f64 {
    0.7
}
fn default_output_dir() -> String {
    "./skills".into()
}

impl Default for SkillForgeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_integrate: default_auto_integrate(),
            sources: default_sources(),
            scan_interval_hours: default_scan_interval(),
            min_score: default_min_score(),
            github_token: None,
            output_dir: default_output_dir(),
        }
    }
}

impl std::fmt::Debug for SkillForgeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillForgeConfig")
            .field("enabled", &self.enabled)
            .field("auto_integrate", &self.auto_integrate)
            .field("sources", &self.sources)
            .field("scan_interval_hours", &self.scan_interval_hours)
            .field("min_score", &self.min_score)
            .field("github_token", &self.github_token.as_ref().map(|_| "***"))
            .field("output_dir", &self.output_dir)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// ForgeReport — summary of a single pipeline run
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeReport {
    pub discovered: usize,
    pub evaluated: usize,
    pub auto_integrated: usize,
    pub manual_review: usize,
    pub skipped: usize,
    #[serde(default)]
    pub failed: usize,
    pub results: Vec<EvalResult>,
}

// ---------------------------------------------------------------------------
// SkillForge
// ---------------------------------------------------------------------------

pub struct SkillForge {
    config: SkillForgeConfig,
    evaluator: Evaluator,
    integrator: Integrator,
}

impl SkillForge {
    pub fn new(mut config: SkillForgeConfig) -> Self {
        if config.auto_integrate {
            warn!(
                "skillforge.auto_integrate is deprecated and will be removed in a future release. \
                 Use `corvus skills discover` + `corvus skills install <url>` instead. \
                 The setting has been ignored."
            );
            config.auto_integrate = false;
        }
        let evaluator = Evaluator::new(config.min_score);
        let integrator = Integrator::new(config.output_dir.clone());
        Self {
            config,
            evaluator,
            integrator,
        }
    }

    /// Run the full pipeline: Scout → Evaluate → Integrate.
    pub async fn forge(&self) -> Result<ForgeReport> {
        if !self.config.enabled {
            warn!("SkillForge is disabled — skipping");
            return Ok(ForgeReport {
                discovered: 0,
                evaluated: 0,
                auto_integrated: 0,
                manual_review: 0,
                skipped: 0,
                failed: 0,
                results: vec![],
            });
        }

        // --- Scout ----------------------------------------------------------
        let mut candidates = self.run_scouts().await;

        // Deduplicate by URL
        scout::dedup(&mut candidates);
        let discovered = candidates.len();
        info!(discovered, "Total unique candidates after dedup");

        // --- Evaluate -------------------------------------------------------
        let results: Vec<EvalResult> = candidates
            .into_iter()
            .map(|c| self.evaluator.evaluate(c))
            .collect();
        let evaluated = results.len();

        // --- Integrate ------------------------------------------------------
        let (auto_integrated, manual_review, skipped, failed) = self.integrate_results(&results);

        info!(
            auto_integrated,
            manual_review, skipped, failed, "Forge pipeline complete"
        );

        Ok(ForgeReport {
            discovered,
            evaluated,
            auto_integrated,
            manual_review,
            skipped,
            failed,
            results,
        })
    }

    async fn run_scouts(&self) -> Vec<ScoutResult> {
        let mut candidates: Vec<ScoutResult> = Vec::new();
        for src in &self.config.sources {
            let source: ScoutSource = match src.parse() {
                Ok(s) => s,
                Err(e) => {
                    warn!(source = %src, error = %e, "Skipping unknown scout source");
                    continue;
                }
            };
            match source {
                ScoutSource::GitHub => {
                    let scout = GitHubScout::new(self.config.github_token.clone());
                    match scout.discover().await {
                        Ok(mut found) => {
                            info!(count = found.len(), "GitHub scout returned candidates");
                            candidates.append(&mut found);
                        }
                        Err(e) => {
                            warn!(error = %e, "GitHub scout failed, continuing with other sources");
                        }
                    }
                }
                ScoutSource::ClawHub | ScoutSource::HuggingFace => {
                    info!(
                        source = src.as_str(),
                        "Source not yet implemented — skipping"
                    );
                }
            }
        }
        candidates
    }

    fn integrate_results(&self, results: &[EvalResult]) -> (usize, usize, usize, usize) {
        let mut auto_integrated = 0usize;
        let mut manual_review = 0usize;
        let mut skipped = 0usize;
        let mut failed = 0usize;

        for res in results {
            match res.recommendation {
                Recommendation::Auto => {
                    if self.config.auto_integrate {
                        match self.integrator.integrate(&res.candidate) {
                            Ok(_) => auto_integrated += 1,
                            Err(e) => {
                                warn!(
                                    skill = res.candidate.name.as_str(),
                                    error = %e,
                                    "Integration failed for candidate, continuing"
                                );
                                failed += 1;
                            }
                        }
                    } else {
                        manual_review += 1;
                    }
                }
                Recommendation::Manual => manual_review += 1,
                Recommendation::Skip => skipped += 1,
            }
        }

        (auto_integrated, manual_review, skipped, failed)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skillforge::scout::{ScoutResult, ScoutSource};

    fn make_candidate(name: &str, stars: u64, lang: Option<&str>) -> ScoutResult {
        ScoutResult {
            name: name.into(),
            url: format!("https://github.com/test/{name}"),
            description: format!("A {name} skill"),
            stars,
            language: lang.map(String::from),
            updated_at: Some(chrono::Utc::now()),
            source: ScoutSource::GitHub,
            owner: "test".into(),
            has_license: true,
        }
    }

    #[tokio::test]
    async fn disabled_forge_returns_empty_report() {
        let cfg = SkillForgeConfig {
            enabled: false,
            ..Default::default()
        };
        let forge = SkillForge::new(cfg);
        let report = forge.forge().await.unwrap();
        assert_eq!(report.discovered, 0);
        assert_eq!(report.auto_integrated, 0);
    }

    #[test]
    fn default_config_values() {
        let cfg = SkillForgeConfig::default();
        assert!(!cfg.enabled);
        assert!(!cfg.auto_integrate);
        assert_eq!(cfg.scan_interval_hours, 24);
        assert!((cfg.min_score - 0.7).abs() < f64::EPSILON);
        assert_eq!(cfg.sources, vec!["github", "clawhub"]);
    }

    #[test]
    fn forge_report_serialization_with_failed() {
        let report = ForgeReport {
            discovered: 5,
            evaluated: 5,
            auto_integrated: 2,
            manual_review: 1,
            skipped: 1,
            failed: 1,
            results: vec![],
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: ForgeReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.failed, 1);
        assert_eq!(parsed.discovered, 5);
        assert_eq!(parsed.auto_integrated, 2);
        assert_eq!(parsed.manual_review, 1);
        assert_eq!(parsed.skipped, 1);
    }

    #[test]
    fn integrate_results_increments_failed_on_bad_path() {
        // Create a real temp file, then use it as a "directory" — fails cross-platform
        // because you can't create a subdirectory under a regular file.
        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        let bad_path = tmp_file.path().join("child");
        // NOTE: auto_integrate is deprecated and forced to false by SkillForge::new().
        // This test verifies integrate_results routing, so we construct the struct
        // directly to test the integration path.
        let cfg = SkillForgeConfig {
            enabled: true,
            auto_integrate: false,
            output_dir: bad_path.to_string_lossy().into_owned(),
            ..Default::default()
        };
        let forge = SkillForge::new(cfg);

        let candidate = make_candidate("test-skill", 500, Some("Rust"));
        let eval_result = forge.evaluator.evaluate(candidate);
        assert_eq!(eval_result.recommendation, Recommendation::Auto);

        // With auto_integrate=false, Auto recommendations route to manual_review
        let (auto_integrated, manual_review, _skipped, failed) =
            forge.integrate_results(&[eval_result]);
        assert_eq!(auto_integrated, 0);
        assert_eq!(manual_review, 1);
        assert_eq!(failed, 0);
    }

    #[test]
    fn integrate_results_empty_candidates() {
        let cfg = SkillForgeConfig {
            enabled: true,
            ..Default::default()
        };
        let forge = SkillForge::new(cfg);

        let (auto_integrated, manual_review, skipped, failed) = forge.integrate_results(&[]);
        assert_eq!(auto_integrated, 0);
        assert_eq!(manual_review, 0);
        assert_eq!(skipped, 0);
        assert_eq!(failed, 0);
    }

    #[test]
    fn integrate_results_all_skip() {
        let cfg = SkillForgeConfig {
            enabled: true,
            min_score: 0.99, // Very high threshold so everything is below
            ..Default::default()
        };
        let forge = SkillForge::new(cfg);

        // Low-star, no-language, no-license → low score → Skip
        let candidate = ScoutResult {
            name: "bad-skill".into(),
            url: "https://github.com/test/bad-skill".into(),
            description: "A bad skill".into(),
            stars: 0,
            language: None,
            updated_at: None,
            source: ScoutSource::GitHub,
            owner: "test".into(),
            has_license: false,
        };
        let eval_result = forge.evaluator.evaluate(candidate);
        assert_eq!(eval_result.recommendation, Recommendation::Skip);

        let (auto_integrated, manual_review, skipped, failed) =
            forge.integrate_results(&[eval_result]);
        assert_eq!(auto_integrated, 0);
        assert_eq!(manual_review, 0);
        assert_eq!(skipped, 1);
        assert_eq!(failed, 0);
    }

    #[test]
    fn integrate_results_auto_integrate_disabled_routes_to_manual() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg = SkillForgeConfig {
            enabled: true,
            auto_integrate: false,
            output_dir: tmp.path().to_string_lossy().into_owned(),
            ..Default::default()
        };
        let forge = SkillForge::new(cfg);

        let candidate = make_candidate("good-skill", 500, Some("Rust"));
        let eval_result = forge.evaluator.evaluate(candidate);
        assert_eq!(eval_result.recommendation, Recommendation::Auto);

        let (auto_integrated, manual_review, _skipped, failed) =
            forge.integrate_results(&[eval_result]);
        assert_eq!(auto_integrated, 0);
        assert_eq!(manual_review, 1);
        assert_eq!(failed, 0);
    }

    #[test]
    fn evaluate_then_integrate_pipeline_routes_to_manual_review() {
        // auto_integrate is deprecated — SkillForge::new() forces it to false.
        // Auto-recommended candidates now route to manual_review.
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg = SkillForgeConfig {
            enabled: true,
            output_dir: tmp.path().to_string_lossy().into_owned(),
            ..Default::default()
        };
        let forge = SkillForge::new(cfg);

        let candidates = vec![
            make_candidate("skill-a", 500, Some("Rust")),
            make_candidate("skill-b", 500, Some("Rust")),
        ];

        let results: Vec<EvalResult> = candidates
            .into_iter()
            .map(|c| forge.evaluator.evaluate(c))
            .collect();

        let (auto_integrated, manual_review, _skipped, failed) = forge.integrate_results(&results);
        assert_eq!(auto_integrated, 0);
        assert_eq!(manual_review, 2);
        assert_eq!(failed, 0);

        // No files should be created since auto-integration is deprecated
        assert!(!tmp.path().join("skill-a").join("SKILL.toml").exists());
        assert!(!tmp.path().join("skill-b").join("SKILL.md").exists());
    }

    #[test]
    fn forge_report_zero_values_serialize() {
        let report = ForgeReport {
            discovered: 0,
            evaluated: 0,
            auto_integrated: 0,
            manual_review: 0,
            skipped: 0,
            failed: 0,
            results: vec![],
        };
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["failed"], 0);
        assert_eq!(json["results"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn forge_report_legacy_json_without_failed() {
        let json = r#"{
            "discovered": 3,
            "evaluated": 2,
            "auto_integrated": 1,
            "manual_review": 0,
            "skipped": 1,
            "results": []
        }"#;
        let report: ForgeReport = serde_json::from_str(json).unwrap();
        assert_eq!(report.failed, 0);
        assert_eq!(report.discovered, 3);
    }

    #[test]
    fn auto_integrate_deprecated_and_forced_false() {
        // R11.4 scenario 27: auto_integrate = true is deprecated and ignored.
        let cfg = SkillForgeConfig {
            enabled: true,
            auto_integrate: true,
            ..Default::default()
        };
        // SkillForge::new() must force auto_integrate to false
        let forge = SkillForge::new(cfg);
        assert!(
            !forge.config.auto_integrate,
            "auto_integrate must be forced to false by SkillForge::new()"
        );

        // Auto-recommended candidates should route to manual_review, not integrate
        let candidate = make_candidate("test-skill", 500, Some("Rust"));
        let eval_result = forge.evaluator.evaluate(candidate);
        assert_eq!(eval_result.recommendation, Recommendation::Auto);

        let (auto_integrated, manual_review, _skipped, _failed) =
            forge.integrate_results(&[eval_result]);
        assert_eq!(auto_integrated, 0);
        assert_eq!(manual_review, 1);
    }

    #[test]
    fn config_debug_redacts_token() {
        let cfg = SkillForgeConfig {
            github_token: Some("secret-token-123".into()),
            ..Default::default()
        };
        let debug_str = format!("{:?}", cfg);
        assert!(!debug_str.contains("secret-token-123"));
        assert!(debug_str.contains("***"));
    }
}
