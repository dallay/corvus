use crate::errors::CerebroError;

pub fn require_non_empty(field: &str, value: &str) -> Result<(), CerebroError> {
    if value.trim().is_empty() {
        return Err(CerebroError::Validation(format!(
            "{field} must be non-empty",
        )));
    }
    Ok(())
}
