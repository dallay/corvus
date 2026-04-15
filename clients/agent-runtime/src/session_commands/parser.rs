use super::types::SessionSlashCommand;

pub struct SessionCommandParser;

impl SessionCommandParser {
    pub fn parse(prompt: &str) -> Option<SessionSlashCommand> {
        let input = prompt.trim_end();
        Self::parse_exact(input, "/tldr", |_| Some(SessionSlashCommand::Tldr))
            .or_else(|| {
                Self::parse_exact(input, "/suspend", |_| Some(SessionSlashCommand::Suspend))
            })
            .or_else(|| {
                Self::parse_with_prefix(input, "/compact", |rest| {
                    Some(SessionSlashCommand::Compact {
                        args: rest.trim_start().to_string(),
                    })
                })
            })
            .or_else(|| {
                Self::parse_with_prefix(input, "/resume", |rest| {
                    let trimmed = rest.trim_start();
                    let target_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
                    let target = if target_end == 0 {
                        None
                    } else {
                        Some(trimmed[..target_end].to_string())
                    };
                    let args = trimmed.to_string();
                    Some(SessionSlashCommand::Resume { target, args })
                })
            })
    }

    /// Parse a command that must match the input exactly (no trailing args).
    fn parse_exact<F, T>(input: &str, command: &str, build: F) -> Option<T>
    where
        F: FnOnce(&str) -> Option<T>,
    {
        let rest = input.strip_prefix(command)?;
        if rest.trim().is_empty() {
            build(rest)
        } else {
            None
        }
    }

    fn parse_with_prefix<F, T>(input: &str, command: &str, build: F) -> Option<T>
    where
        F: FnOnce(&str) -> Option<T>,
    {
        let rest = input.strip_prefix(command)?;
        if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
            return None;
        }
        build(rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exact_commands() {
        assert_eq!(
            SessionCommandParser::parse("/tldr"),
            Some(SessionSlashCommand::Tldr)
        );
        assert_eq!(
            SessionCommandParser::parse("/suspend"),
            Some(SessionSlashCommand::Suspend)
        );
    }

    #[test]
    fn parses_resume_target_and_compact_trailing_args() {
        assert_eq!(
            SessionCommandParser::parse("/resume abc-123"),
            Some(SessionSlashCommand::Resume {
                target: Some("abc-123".to_string()),
                args: "abc-123".to_string(),
            })
        );
        assert_eq!(
            SessionCommandParser::parse("/compact keep only the latest goals"),
            Some(SessionSlashCommand::Compact {
                args: "keep only the latest goals".to_string(),
            })
        );
    }

    #[test]
    fn resume_without_target_lists_sessions() {
        assert_eq!(
            SessionCommandParser::parse("/resume   \n"),
            Some(SessionSlashCommand::Resume {
                target: None,
                args: String::new(),
            })
        );
    }

    #[test]
    fn slash_like_unknown_inputs_fall_through() {
        assert_eq!(SessionCommandParser::parse("/resume-later"), None);
        assert_eq!(SessionCommandParser::parse("hello /tldr"), None);
    }

    #[test]
    fn tldr_and_suspend_reject_trailing_args() {
        assert_eq!(SessionCommandParser::parse("/tldr please"), None);
        assert_eq!(SessionCommandParser::parse("/suspend now"), None);
        assert_eq!(SessionCommandParser::parse("/tldr extra args"), None);
        assert_eq!(SessionCommandParser::parse("/suspend extra"), None);
    }

    #[test]
    fn tldr_and_suspend_accept_trailing_whitespace() {
        assert_eq!(
            SessionCommandParser::parse("/tldr   "),
            Some(SessionSlashCommand::Tldr)
        );
        assert_eq!(
            SessionCommandParser::parse("/suspend  \n"),
            Some(SessionSlashCommand::Suspend)
        );
    }
}
