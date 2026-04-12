#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityAvailability {
    Constructible,
    Uncompiled,
    PlatformUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub key: &'static str,
    pub display_name: &'static str,
    pub aliases: &'static [&'static str],
    pub compiled: bool,
    pub platform_supported: bool,
    pub supports_native_tools: bool,
}

const PROVIDERS: &[ProviderDescriptor] = &[
    ProviderDescriptor {
        key: "openrouter",
        display_name: "OpenRouter",
        aliases: &[],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "anthropic",
        display_name: "Anthropic",
        aliases: &[],
        compiled: true,
        platform_supported: true,
        supports_native_tools: true,
    },
    ProviderDescriptor {
        key: "openai",
        display_name: "OpenAI",
        aliases: &[],
        compiled: true,
        platform_supported: true,
        supports_native_tools: true,
    },
    ProviderDescriptor {
        key: "openai-codex",
        display_name: "OpenAI Codex",
        aliases: &["openai_codex", "codex"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: true,
    },
    ProviderDescriptor {
        key: "ollama",
        display_name: "Ollama",
        aliases: &[],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "gemini",
        display_name: "Google Gemini",
        aliases: &["google", "google-gemini"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: true,
    },
    ProviderDescriptor {
        key: "venice",
        display_name: "Venice",
        aliases: &[],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "vercel",
        display_name: "Vercel AI Gateway",
        aliases: &["vercel-ai"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "cloudflare",
        display_name: "Cloudflare AI",
        aliases: &["cloudflare-ai"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "moonshot",
        display_name: "Moonshot",
        aliases: &[
            "moonshot-intl",
            "moonshot-global",
            "kimi",
            "kimi-intl",
            "kimi-global",
            "moonshot-cn",
            "kimi-cn",
        ],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "synthetic",
        display_name: "Synthetic",
        aliases: &[],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "opencode",
        display_name: "OpenCode Zen",
        aliases: &["opencode-zen"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "zai",
        display_name: "Z.AI",
        aliases: &["z.ai", "zai-global", "z.ai-global", "zai-cn", "z.ai-cn"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "glm",
        display_name: "GLM",
        aliases: &[
            "zhipu",
            "glm-global",
            "zhipu-global",
            "glm-cn",
            "zhipu-cn",
            "bigmodel",
        ],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "minimax",
        display_name: "MiniMax",
        aliases: &[
            "minimax-intl",
            "minimax-io",
            "minimax-global",
            "minimax-cn",
            "minimaxi",
        ],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "bedrock",
        display_name: "Amazon Bedrock",
        aliases: &["aws-bedrock"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "qianfan",
        display_name: "Qianfan",
        aliases: &["baidu"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "qwen",
        display_name: "Qwen",
        aliases: &[
            "dashscope",
            "qwen-cn",
            "dashscope-cn",
            "qwen-intl",
            "dashscope-intl",
            "qwen-international",
            "dashscope-international",
            "qwen-us",
            "dashscope-us",
        ],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "groq",
        display_name: "Groq",
        aliases: &[],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "mistral",
        display_name: "Mistral",
        aliases: &[],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "xai",
        display_name: "xAI",
        aliases: &["grok"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "deepseek",
        display_name: "DeepSeek",
        aliases: &[],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "together",
        display_name: "Together AI",
        aliases: &["together-ai"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "fireworks",
        display_name: "Fireworks AI",
        aliases: &["fireworks-ai"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "perplexity",
        display_name: "Perplexity",
        aliases: &[],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "cohere",
        display_name: "Cohere",
        aliases: &[],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "copilot",
        display_name: "GitHub Copilot",
        aliases: &["github-copilot"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "lmstudio",
        display_name: "LM Studio",
        aliases: &["lm-studio"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
    ProviderDescriptor {
        key: "nvidia",
        display_name: "NVIDIA NIM",
        aliases: &["nvidia-nim", "build.nvidia.com"],
        compiled: true,
        platform_supported: true,
        supports_native_tools: false,
    },
];

pub fn list_providers() -> &'static [ProviderDescriptor] {
    PROVIDERS
}

pub fn resolve_provider_key(name: &str) -> Option<&'static str> {
    let candidate = name.trim();
    PROVIDERS
        .iter()
        .find(|descriptor| {
            descriptor.key.eq_ignore_ascii_case(candidate)
                || descriptor
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(candidate))
        })
        .map(|descriptor| descriptor.key)
}

pub fn provider_availability(name: &str) -> Option<CapabilityAvailability> {
    let key = resolve_provider_key(name)?;
    PROVIDERS
        .iter()
        .find(|descriptor| descriptor.key == key)
        .map(|descriptor| {
            if !descriptor.platform_supported {
                CapabilityAvailability::PlatformUnavailable
            } else if !descriptor.compiled {
                CapabilityAvailability::Uncompiled
            } else {
                CapabilityAvailability::Constructible
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_aliases_to_canonical_provider_keys() {
        assert_eq!(resolve_provider_key("google"), Some("gemini"));
        assert_eq!(resolve_provider_key("github-copilot"), Some("copilot"));
    }
}
