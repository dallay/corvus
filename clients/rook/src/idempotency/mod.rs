pub mod canonical;
pub mod middleware;
pub mod types;

pub const MAX_IDEMPOTENCY_KEY_LENGTH: usize = 255;

pub fn is_valid_idempotency_key(value: &str) -> bool {
    let len = value.len();
    if len == 0 || len > MAX_IDEMPOTENCY_KEY_LENGTH {
        return false;
    }

    value
        .bytes()
        .all(|byte| (0x21..=0x7e).contains(&byte) && byte != b' ')
}

#[cfg(test)]
mod tests {
    use super::is_valid_idempotency_key;

    #[test]
    fn idempotency_key_validation_accepts_visible_ascii_without_spaces() {
        assert!(is_valid_idempotency_key("chat-123_ABC~z"));
    }

    #[test]
    fn idempotency_key_validation_rejects_spaces_and_control_characters() {
        assert!(!is_valid_idempotency_key("bad key"));
        assert!(!is_valid_idempotency_key("bad\nkey"));
    }
}
