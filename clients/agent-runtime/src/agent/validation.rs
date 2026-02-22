use crate::memory::Memory;
use crate::providers::Provider;

const MAX_PROMPT_DRAFT_RESPONSE_CHARS: usize = 8_000;
const MAX_PROMPT_VIOLATIONS_CHARS: usize = 2_000;

fn truncate_for_prompt(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let preview: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_none() {
        return preview;
    }

    format!("{preview}\n...[truncated for safety]...")
}

pub(crate) async fn enforce_strict_validation(
    mem: &dyn Memory,
    provider: &dyn Provider,
    model: &str,
    temperature: f64,
    user_query: &str,
    candidate: String,
) -> String {
    let validation = match mem.validate_response(user_query, &candidate, None).await {
        Ok(value) => value,
        Err(_) => {
            tracing::warn!("Memory validation failed");
            return "I cannot provide a validated answer right now because ontology validation failed."
                .to_string();
        }
    };

    if validation.valid {
        return candidate;
    }

    let violations_text = if validation.violations.is_empty() {
        "- unknown ontology violation".to_string()
    } else {
        validation
            .violations
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let candidate_for_prompt = truncate_for_prompt(&candidate, MAX_PROMPT_DRAFT_RESPONSE_CHARS);
    let violations_for_prompt = truncate_for_prompt(&violations_text, MAX_PROMPT_VIOLATIONS_CHARS);

    let correction_prompt = format!(
        "User query:\n{}\n\nDraft response:\n{}\n\nOntology violations:\n{}\n\nRewrite the draft response so all violations are fixed. Keep it concise and factual. Do not call tools.",
        user_query,
        candidate_for_prompt,
        violations_for_prompt,
    );

    let corrected = match provider
        .chat_with_system(
            Some(
                "You repair responses to satisfy strict domain ontology rules. Return only the corrected response text.",
            ),
            &correction_prompt,
            model,
            temperature,
        )
        .await
    {
        Ok(value) => value,
        Err(_) => {
            tracing::warn!("Ontology correction pass failed");
            return format!(
                "I cannot provide a validated answer because strict ontology checks failed:\n{}",
                violations_text
            );
        }
    };

    match mem.validate_response(user_query, &corrected, None).await {
        Ok(checked) if checked.valid => corrected,
        Ok(checked) => {
            let checked_violations = if checked.violations.is_empty() {
                violations_text
            } else {
                checked
                    .violations
                    .iter()
                    .map(|item| format!("- {item}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            format!(
                "I cannot provide a validated answer because strict ontology checks still fail:\n{}",
                checked_violations
            )
        }
        Err(_) => {
            tracing::warn!("Post-correction ontology validation failed");
            "I cannot provide a validated answer because ontology checks are unavailable."
                .to_string()
        }
    }
}
