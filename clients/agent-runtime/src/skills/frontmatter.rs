//! Minimal YAML frontmatter parser for SKILL.md files.
//! Extracts fields relevant to trust model: name, description, allowed-tools, version, author, tags.
//! Does NOT add serde_yaml as a dependency — parses the simple flat structure directly.

/// Parsed frontmatter from a SKILL.md file.
#[derive(Debug, Clone, Default)]
pub struct SkillFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub allowed_tools: Vec<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
}

/// Parse YAML frontmatter from SKILL.md content.
/// Expects `---` delimiters. Returns default on parse failure (safe default).
pub fn parse_frontmatter(content: &str) -> SkillFrontmatter {
    let Some(fm_block) = extract_frontmatter_block(content) else {
        return SkillFrontmatter::default();
    };
    parse_frontmatter_block(fm_block)
}

/// Extract the raw frontmatter text between `---` delimiters.
fn extract_frontmatter_block(content: &str) -> Option<&str> {
    let trimmed = content.trim_start();
    let rest = trimmed.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

/// Parse key-value pairs and list items from the frontmatter block.
fn parse_frontmatter_block(block: &str) -> SkillFrontmatter {
    let mut fm = SkillFrontmatter::default();
    let mut current_list: Option<&'static str> = None;

    for line in block.lines() {
        let trimmed = line.trim();

        if try_push_list_item(trimmed, current_list, &mut fm) {
            continue;
        }

        // Line is not a list continuation — reset list context
        current_list = parse_scalar_or_list_key(trimmed, &mut fm);
    }

    fm
}

/// If we are inside a YAML list block and the line starts with `- `,
/// push the value and return `true`. Returns `false` otherwise.
fn try_push_list_item(trimmed: &str, list_key: Option<&str>, fm: &mut SkillFrontmatter) -> bool {
    let Some(list_key) = list_key else {
        return false;
    };
    let Some(item) = trimmed.strip_prefix("- ") else {
        return false;
    };
    let value = item.trim().trim_matches('"').trim_matches('\'');
    match list_key {
        "allowed-tools" => fm.allowed_tools.push(value.to_string()),
        "tags" => fm.tags.push(value.to_string()),
        _ => {}
    }
    true
}

/// Parse a `key: value` line. Scalar keys set fields directly;
/// list keys (`allowed-tools:`, `tags:`) with an empty value return
/// the key name so the caller enters list-collection mode.
fn parse_scalar_or_list_key(trimmed: &str, fm: &mut SkillFrontmatter) -> Option<&'static str> {
    let (key, value) = trimmed.split_once(':')?;
    let key = key.trim();
    let value = value.trim().trim_matches('"').trim_matches('\'');
    match key {
        "name" => fm.name = Some(value.to_string()),
        "description" => fm.description = Some(value.to_string()),
        "version" => fm.version = Some(value.to_string()),
        "author" => fm.author = Some(value.to_string()),
        "allowed-tools" if value.is_empty() => return Some("allowed-tools"),
        "tags" if value.is_empty() => return Some("tags"),
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_frontmatter_with_all_fields() {
        let content = "\
---
name: example-skill
description: An example skill
allowed-tools:
  - Read
  - Grep
  - Glob
---

# Example Skill
Some content here.
";
        let fm = parse_frontmatter(content);
        assert_eq!(fm.name.as_deref(), Some("example-skill"));
        assert_eq!(fm.description.as_deref(), Some("An example skill"));
        assert_eq!(fm.allowed_tools, vec!["Read", "Grep", "Glob"]);
    }

    #[test]
    fn valid_frontmatter_without_allowed_tools() {
        let content = "\
---
name: simple-skill
description: A simple skill
---

# Simple Skill
";
        let fm = parse_frontmatter(content);
        assert_eq!(fm.name.as_deref(), Some("simple-skill"));
        assert_eq!(fm.description.as_deref(), Some("A simple skill"));
        assert!(fm.allowed_tools.is_empty());
    }

    #[test]
    fn no_frontmatter_delimiters_returns_default() {
        let content = "# Just a Heading\nSome content.\n";
        let fm = parse_frontmatter(content);
        assert!(fm.name.is_none());
        assert!(fm.description.is_none());
        assert!(fm.allowed_tools.is_empty());
    }

    #[test]
    fn malformed_content_returns_default_no_panic() {
        let content = "\
---
this is not: valid: yaml: at: all
   :::broken:::
---
";
        let fm = parse_frontmatter(content);
        // Should not panic — returns default or partial parse
        assert!(fm.allowed_tools.is_empty());
    }

    #[test]
    fn empty_allowed_tools_returns_empty_vec() {
        let content = "\
---
name: no-tools
description: Skill without tools
allowed-tools:
---

# No Tools
";
        let fm = parse_frontmatter(content);
        assert_eq!(fm.name.as_deref(), Some("no-tools"));
        assert!(fm.allowed_tools.is_empty());
    }

    #[test]
    fn extra_unknown_fields_still_parses_known() {
        let content = "\
---
name: extra-fields
description: Has extra stuff
version: 2.0.0
author: someone
allowed-tools:
  - Read
custom-field: should-be-ignored
---

# Extra Fields
";
        let fm = parse_frontmatter(content);
        assert_eq!(fm.name.as_deref(), Some("extra-fields"));
        assert_eq!(fm.description.as_deref(), Some("Has extra stuff"));
        assert_eq!(fm.version.as_deref(), Some("2.0.0"));
        assert_eq!(fm.author.as_deref(), Some("someone"));
        assert_eq!(fm.allowed_tools, vec!["Read"]);
    }

    #[test]
    fn frontmatter_with_version_author_tags() {
        let content = "\
---
name: test-skill
description: A test
version: 1.2.0
author: Jane Doe
tags:
  - git
  - vcs
allowed-tools:
  - Read
---

# Test Skill
";
        let fm = parse_frontmatter(content);
        assert_eq!(fm.name.as_deref(), Some("test-skill"));
        assert_eq!(fm.description.as_deref(), Some("A test"));
        assert_eq!(fm.version.as_deref(), Some("1.2.0"));
        assert_eq!(fm.author.as_deref(), Some("Jane Doe"));
        assert_eq!(fm.tags, vec!["git", "vcs"]);
        assert_eq!(fm.allowed_tools, vec!["Read"]);
    }

    #[test]
    fn missing_new_fields_default_to_none_and_empty() {
        let content = "\
---
name: minimal
description: No new fields
---

# Minimal
";
        let fm = parse_frontmatter(content);
        assert_eq!(fm.name.as_deref(), Some("minimal"));
        assert!(fm.version.is_none());
        assert!(fm.author.is_none());
        assert!(fm.tags.is_empty());
    }

    #[test]
    fn only_opening_delimiter_returns_default() {
        let content = "\
---
name: incomplete
description: No closing delimiter
";
        let fm = parse_frontmatter(content);
        assert!(fm.name.is_none());
        assert!(fm.description.is_none());
        assert!(fm.allowed_tools.is_empty());
    }
}
