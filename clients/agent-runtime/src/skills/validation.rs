//! Skill name validation per Agent Skills standard.

/// Validation error for skill names.
#[derive(Debug, thiserror::Error)]
pub enum SkillValidationError {
    #[error("skill name is empty")]
    Empty,
    #[error("skill name '{0}' exceeds 64 characters")]
    TooLong(String),
    #[error("skill name '{0}' contains invalid characters (must be [a-z0-9-])")]
    InvalidChars(String),
    #[error("skill name '{0}' starts or ends with a hyphen")]
    HyphenBoundary(String),
    #[error("skill name '{0}' contains consecutive hyphens")]
    ConsecutiveHyphens(String),
}

/// Validate a skill name per Agent Skills standard rules.
/// Valid: `^[a-z0-9]([a-z0-9-]*[a-z0-9])?$`, 1-64 chars, no `--`.
pub fn validate_skill_name(name: &str) -> Result<(), SkillValidationError> {
    if name.is_empty() {
        return Err(SkillValidationError::Empty);
    }
    if name.len() > 64 {
        return Err(SkillValidationError::TooLong(name.to_string()));
    }
    if !name
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(SkillValidationError::InvalidChars(name.to_string()));
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Err(SkillValidationError::HyphenBoundary(name.to_string()));
    }
    if name.contains("--") {
        return Err(SkillValidationError::ConsecutiveHyphens(name.to_string()));
    }
    Ok(())
}

/// Convenience check — returns true if name is valid.
pub fn is_valid_skill_name(name: &str) -> bool {
    validate_skill_name(name).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_names() {
        assert!(validate_skill_name("my-skill").is_ok());
        assert!(validate_skill_name("x").is_ok());
        assert!(validate_skill_name("a1b2").is_ok());
        assert!(validate_skill_name("git-expert").is_ok());
        assert!(validate_skill_name("a").is_ok());
        // 64 chars
        let name_64 = "a".repeat(64);
        assert!(validate_skill_name(&name_64).is_ok());
    }

    #[test]
    fn invalid_empty() {
        assert!(matches!(
            validate_skill_name(""),
            Err(SkillValidationError::Empty)
        ));
    }

    #[test]
    fn invalid_too_long() {
        let name_65 = "a".repeat(65);
        assert!(matches!(
            validate_skill_name(&name_65),
            Err(SkillValidationError::TooLong(_))
        ));
    }

    #[test]
    fn invalid_uppercase() {
        assert!(matches!(
            validate_skill_name("My-Skill"),
            Err(SkillValidationError::InvalidChars(_))
        ));
    }

    #[test]
    fn invalid_underscore() {
        assert!(matches!(
            validate_skill_name("my_skill"),
            Err(SkillValidationError::InvalidChars(_))
        ));
    }

    #[test]
    fn invalid_leading_hyphen() {
        assert!(matches!(
            validate_skill_name("-bad"),
            Err(SkillValidationError::HyphenBoundary(_))
        ));
    }

    #[test]
    fn invalid_trailing_hyphen() {
        assert!(matches!(
            validate_skill_name("bad-"),
            Err(SkillValidationError::HyphenBoundary(_))
        ));
    }

    #[test]
    fn invalid_consecutive_hyphens() {
        assert!(matches!(
            validate_skill_name("bad--name"),
            Err(SkillValidationError::ConsecutiveHyphens(_))
        ));
    }

    #[test]
    fn is_valid_convenience() {
        assert!(is_valid_skill_name("good-name"));
        assert!(!is_valid_skill_name("BAD"));
    }
}
