use crate::config::schema::QueryClassificationConfig;

fn within_length_constraints(
    len: usize,
    min_length: Option<usize>,
    max_length: Option<usize>,
) -> bool {
    if let Some(min) = min_length {
        if len < min {
            return false;
        }
    }

    if let Some(max) = max_length {
        if len > max {
            return false;
        }
    }

    true
}

fn has_keyword_or_pattern_match(
    lower_message: &str,
    message: &str,
    keywords: &[String],
    patterns: &[String],
) -> bool {
    let keyword_hit = keywords
        .iter()
        .any(|keyword| keyword_matches(lower_message, keyword));
    let pattern_hit = patterns
        .iter()
        .any(|pattern| message.contains(pattern.as_str()));
    keyword_hit || pattern_hit
}

fn keyword_matches(lower_message: &str, keyword: &str) -> bool {
    if keyword.bytes().all(|byte| !byte.is_ascii_uppercase()) {
        return lower_message.contains(keyword);
    }

    lower_message.contains(&keyword.to_ascii_lowercase())
}

/// Classify a user message against the configured rules and return the
/// matching hint string, if any.
///
/// Returns `None` when classification is disabled, no rules are configured,
/// or no rule matches the message.
pub fn classify(config: &QueryClassificationConfig, message: &str) -> Option<String> {
    if !config.enabled || config.rules.is_empty() {
        return None;
    }

    let lower = message.to_lowercase();
    let len = message.len();

    let mut rules: Vec<_> = config.rules.iter().collect();
    rules.sort_by_key(|rule| std::cmp::Reverse(rule.priority));

    for rule in rules {
        if !within_length_constraints(len, rule.min_length, rule.max_length) {
            continue;
        }

        if has_keyword_or_pattern_match(&lower, message, &rule.keywords, &rule.patterns) {
            return Some(rule.hint.clone());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{ClassificationRule, QueryClassificationConfig};

    fn make_config(enabled: bool, rules: Vec<ClassificationRule>) -> QueryClassificationConfig {
        QueryClassificationConfig { enabled, rules }
    }

    #[test]
    fn disabled_returns_none() {
        let config = make_config(
            false,
            vec![ClassificationRule {
                hint: "fast".into(),
                keywords: vec!["hello".into()],
                ..Default::default()
            }],
        );
        assert_eq!(classify(&config, "hello"), None);
    }

    #[test]
    fn empty_rules_returns_none() {
        let config = make_config(true, vec![]);
        assert_eq!(classify(&config, "hello"), None);
    }

    #[test]
    fn keyword_match_case_insensitive() {
        let config = make_config(
            true,
            vec![ClassificationRule {
                hint: "fast".into(),
                keywords: vec!["hello".into()],
                ..Default::default()
            }],
        );
        assert_eq!(classify(&config, "HELLO world"), Some("fast".into()));
    }

    #[test]
    fn pattern_match_case_sensitive() {
        let config = make_config(
            true,
            vec![ClassificationRule {
                hint: "code".into(),
                patterns: vec!["fn ".into()],
                ..Default::default()
            }],
        );
        assert_eq!(classify(&config, "fn main()"), Some("code".into()));
        assert_eq!(classify(&config, "FN MAIN()"), None);
    }

    #[test]
    fn length_constraints() {
        let config = make_config(
            true,
            vec![ClassificationRule {
                hint: "fast".into(),
                keywords: vec!["hi".into()],
                max_length: Some(10),
                ..Default::default()
            }],
        );
        assert_eq!(classify(&config, "hi"), Some("fast".into()));
        assert_eq!(
            classify(&config, "hi there, how are you doing today?"),
            None
        );

        let config2 = make_config(
            true,
            vec![ClassificationRule {
                hint: "reasoning".into(),
                keywords: vec!["explain".into()],
                min_length: Some(20),
                ..Default::default()
            }],
        );
        assert_eq!(classify(&config2, "explain"), None);
        assert_eq!(
            classify(&config2, "explain how this works in detail"),
            Some("reasoning".into())
        );
    }

    #[test]
    fn priority_ordering() {
        let config = make_config(
            true,
            vec![
                ClassificationRule {
                    hint: "fast".into(),
                    keywords: vec!["code".into()],
                    priority: 1,
                    ..Default::default()
                },
                ClassificationRule {
                    hint: "code".into(),
                    keywords: vec!["code".into()],
                    priority: 10,
                    ..Default::default()
                },
            ],
        );
        assert_eq!(classify(&config, "write some code"), Some("code".into()));
    }

    #[test]
    fn no_match_returns_none() {
        let config = make_config(
            true,
            vec![ClassificationRule {
                hint: "fast".into(),
                keywords: vec!["hello".into()],
                ..Default::default()
            }],
        );
        assert_eq!(classify(&config, "something completely different"), None);
    }
}
