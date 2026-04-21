use crate::domain::{ProviderAccount, ProviderVendor};

pub fn default_base_url(vendor: &ProviderVendor) -> Option<&'static str> {
    match vendor {
        ProviderVendor::OpenAi => Some("https://api.openai.com"),
        ProviderVendor::Anthropic => None,
        ProviderVendor::Google => None,
        ProviderVendor::OpenRouter => Some("https://openrouter.ai/api"),
        ProviderVendor::DeepSeek => Some("https://api.deepseek.com"),
        ProviderVendor::Other(_) => None,
    }
}

pub fn effective_base_url(account: &ProviderAccount) -> Option<String> {
    account
        .api_base_override
        .as_deref()
        .map(|base| base.trim_end_matches('/').to_string())
        .or_else(|| default_base_url(&account.vendor).map(str::to_string))
}

pub fn auth_header(vendor: &ProviderVendor, api_key: &str) -> Option<(&'static str, String)> {
    match vendor {
        ProviderVendor::Anthropic => Some(("x-api-key", api_key.to_string())),
        _ => Some(("authorization", format!("Bearer {api_key}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AccountId, ProviderAccount, ProviderVendor};

    fn make_account(vendor: ProviderVendor) -> ProviderAccount {
        ProviderAccount {
            id: AccountId::generate(),
            vendor,
            display_name: "test".to_string(),
            api_base_override: None,
            api_key: Some("sk-test".to_string()),
            enabled: true,
            weight: 1,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        }
    }

    #[test]
    fn default_base_url_maps_known_vendors() {
        assert_eq!(default_base_url(&ProviderVendor::OpenAi), Some("https://api.openai.com"));
        assert_eq!(default_base_url(&ProviderVendor::Anthropic), None);
        assert_eq!(default_base_url(&ProviderVendor::Google), None);
        assert_eq!(
            default_base_url(&ProviderVendor::OpenRouter),
            Some("https://openrouter.ai/api")
        );
        assert_eq!(default_base_url(&ProviderVendor::DeepSeek), Some("https://api.deepseek.com"));
    }

    #[test]
    fn default_base_url_returns_none_for_other_vendor() {
        assert_eq!(default_base_url(&ProviderVendor::Other("mistral".to_string())), None);
    }

    #[test]
    fn effective_base_url_prefers_override_and_strips_trailing_slash() {
        let mut account = make_account(ProviderVendor::OpenAi);
        account.api_base_override = Some("https://my-proxy.example.com/".to_string());

        assert_eq!(
            effective_base_url(&account),
            Some("https://my-proxy.example.com".to_string())
        );
    }

    #[test]
    fn effective_base_url_uses_default_for_known_vendor_without_override() {
        let account = make_account(ProviderVendor::OpenAi);
        assert_eq!(
            effective_base_url(&account),
            Some("https://api.openai.com".to_string())
        );
    }

    #[test]
    fn effective_base_url_returns_none_for_other_vendor_without_override() {
        let account = make_account(ProviderVendor::Other("mistral".to_string()));
        assert_eq!(effective_base_url(&account), None);
    }

    #[test]
    fn auth_header_uses_bearer_for_openai_compatible_vendors() {
        assert_eq!(
            auth_header(&ProviderVendor::OpenAi, "sk-abc"),
            Some(("authorization", "Bearer sk-abc".to_string()))
        );
        assert_eq!(
            auth_header(&ProviderVendor::DeepSeek, "sk-abc"),
            Some(("authorization", "Bearer sk-abc".to_string()))
        );
        assert_eq!(
            auth_header(&ProviderVendor::Other("custom".to_string()), "sk-abc"),
            Some(("authorization", "Bearer sk-abc".to_string()))
        );
    }

    #[test]
    fn auth_header_uses_x_api_key_for_anthropic() {
        assert_eq!(
            auth_header(&ProviderVendor::Anthropic, "sk-ant-123"),
            Some(("x-api-key", "sk-ant-123".to_string()))
        );
    }
}
