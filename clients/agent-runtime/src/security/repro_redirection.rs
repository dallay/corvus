#[cfg(test)]
mod tests {
    use crate::security::{SecurityPolicy, AutonomyLevel};

    #[test]
    fn test_input_redirection_bypass() {
        let policy = SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            allowed_commands: vec!["cat".into()],
            ..SecurityPolicy::default()
        };

        // This SHOULD be blocked if we want to prevent reading files via redirection
        // currently is_command_allowed only blocks '>'
        let cmd = "cat < /etc/passwd";
        assert!(!policy.is_command_allowed(cmd), "Input redirection '<' should be blocked");
    }
}
