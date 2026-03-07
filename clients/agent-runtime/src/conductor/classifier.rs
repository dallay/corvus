use crate::conductor::TaskDomain;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassificationResult {
    pub domain: TaskDomain,
    pub confidence: Confidence,
}

#[async_trait]
pub trait TaskClassifier: Send + Sync {
    async fn classify(&self, description: &str) -> Result<ClassificationResult>;
}

#[derive(Default)]
pub struct RuleBasedClassifier;

#[async_trait]
impl TaskClassifier for RuleBasedClassifier {
    async fn classify(&self, description: &str) -> Result<ClassificationResult> {
        let normalized = description.to_ascii_lowercase();

        let coding_hits = count_hits(&normalized, &["fix", "refactor", "test", "compile"]);
        let research_hits = count_hits(&normalized, &["research", "analyze", "summarize"]);
        let browser_hits = count_hits(&normalized, &["browser", "screenshot", "scrape"]);
        let system_hits = count_hits(&normalized, &["deploy", "restart", "install"]);

        let mut active_domains = Vec::new();
        if coding_hits > 0 {
            active_domains.push(TaskDomain::Coding);
        }
        if research_hits > 0 {
            active_domains.push(TaskDomain::Research);
        }
        if browser_hits > 0 {
            active_domains.push(TaskDomain::Browser);
        }
        if system_hits > 0 {
            active_domains.push(TaskDomain::System);
        }

        if active_domains.len() > 1 {
            return Ok(ClassificationResult {
                domain: TaskDomain::Composite,
                confidence: Confidence::High,
            });
        }

        if let Some(domain) = active_domains.first().copied() {
            return Ok(ClassificationResult {
                domain,
                confidence: Confidence::High,
            });
        }

        Ok(ClassificationResult {
            domain: TaskDomain::Composite,
            confidence: Confidence::Low,
        })
    }
}

fn count_hits(input: &str, keywords: &[&str]) -> usize {
    keywords
        .iter()
        .filter(|keyword| input.contains(**keyword))
        .count()
}

pub struct StaticLlmClassifier {
    domain: TaskDomain,
    confidence: Confidence,
}

impl StaticLlmClassifier {
    pub fn new(domain: TaskDomain, confidence: Confidence) -> Self {
        Self { domain, confidence }
    }
}

#[async_trait]
impl TaskClassifier for StaticLlmClassifier {
    async fn classify(&self, _description: &str) -> Result<ClassificationResult> {
        Ok(ClassificationResult {
            domain: self.domain,
            confidence: self.confidence,
        })
    }
}

pub struct LlmClassifier {
    inner: Box<dyn TaskClassifier>,
}

impl LlmClassifier {
    pub fn new(inner: impl TaskClassifier + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}

#[async_trait]
impl TaskClassifier for LlmClassifier {
    async fn classify(&self, description: &str) -> Result<ClassificationResult> {
        self.inner.classify(description).await
    }
}

pub struct ChainedClassifier {
    rule_based: RuleBasedClassifier,
    llm: Box<dyn TaskClassifier>,
}

impl ChainedClassifier {
    pub fn new(rule_based: RuleBasedClassifier, llm: impl TaskClassifier + 'static) -> Self {
        Self {
            rule_based,
            llm: Box::new(llm),
        }
    }
}

#[async_trait]
impl TaskClassifier for ChainedClassifier {
    async fn classify(&self, description: &str) -> Result<ClassificationResult> {
        let fast_path = self.rule_based.classify(description).await?;
        if fast_path.confidence == Confidence::High {
            return Ok(fast_path);
        }
        self.llm.classify(description).await
    }
}
