use super::types::RawSlashInvocation;

pub struct SessionCommandParser;

impl SessionCommandParser {
    pub fn parse(prompt: &str) -> Option<RawSlashInvocation> {
        let input = prompt.trim_end();
        if !input.starts_with('/') {
            return None;
        }

        let token_end = input.find(char::is_whitespace).unwrap_or(input.len());
        let invoked_name = &input[..token_end];
        if invoked_name.len() <= 1 {
            return None;
        }

        Some(RawSlashInvocation {
            invoked_name: invoked_name.to_string(),
            raw_args: input[token_end..].trim_start().to_string(),
        })
    }

    pub fn split_primary_target(raw_args: &str) -> (Option<String>, String) {
        let trimmed = raw_args.trim_start();
        let target_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
        let primary_target = if target_end == 0 {
            None
        } else {
            Some(trimmed[..target_end].to_string())
        };

        let remaining_args = if target_end == 0 {
            trimmed.to_string()
        } else {
            trimmed[target_end..].trim_start().to_string()
        };

        (primary_target, remaining_args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_slash_input_into_raw_invocation() {
        assert_eq!(
            SessionCommandParser::parse("/tldr"),
            Some(RawSlashInvocation {
                invoked_name: "/tldr".to_string(),
                raw_args: String::new(),
            })
        );
        assert_eq!(
            SessionCommandParser::parse("/compact keep only the latest goals"),
            Some(RawSlashInvocation {
                invoked_name: "/compact".to_string(),
                raw_args: "keep only the latest goals".to_string(),
            })
        );
    }

    #[test]
    fn split_primary_target_preserves_remaining_args() {
        assert_eq!(
            SessionCommandParser::split_primary_target("abc-123 keep latest goals"),
            (Some("abc-123".to_string()), "keep latest goals".to_string(),)
        );
        assert_eq!(
            SessionCommandParser::split_primary_target(""),
            (None, String::new())
        );
        // Two-word input: first word is target, rest is remaining args
        assert_eq!(
            SessionCommandParser::split_primary_target("just args"),
            (Some("just".to_string()), "args".to_string())
        );
    }

    #[test]
    fn slash_like_unknown_inputs_fall_through() {
        assert_eq!(SessionCommandParser::parse("hello /tldr"), None);
        assert_eq!(SessionCommandParser::parse("/"), None);
    }
}
