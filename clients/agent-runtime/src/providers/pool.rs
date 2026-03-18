use super::traits::{
    build_tool_instructions_text, ChatMessage, ChatResponse, StreamChunk, StreamError,
    StreamOptions, StreamResult, ToolsPayload,
};
use super::{Provider, ProviderRuntimeOptions};
use crate::config::{AccountPoolStrategy, ProviderAccountConfig, ProviderAccountPoolConfig};
use anyhow::{Context, Result};
use futures_util::{stream, StreamExt};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_COOLDOWN_MS: u64 = 2_000;
const MAX_COOLDOWN_MS: u64 = 30_000;

struct WeightedState {
    current: Vec<i64>,
}

impl WeightedState {
    fn new(len: usize) -> Self {
        Self {
            current: vec![0; len],
        }
    }
}

pub struct AccountPoolProvider {
    provider_name: String,
    strategy: AccountPoolStrategy,
    accounts: Vec<ProviderAccountConfig>,
    index: AtomicUsize,
    cooldown_until: Arc<Mutex<HashMap<String, Instant>>>,
    cache: Mutex<HashMap<String, Arc<dyn Provider>>>,
    weighted_state: Mutex<WeightedState>,
    runtime: ProviderRuntimeOptions,
    default_api_url: Option<String>,
}

impl AccountPoolProvider {
    pub fn new(
        provider_name: String,
        pool: ProviderAccountPoolConfig,
        runtime: ProviderRuntimeOptions,
        default_api_url: Option<String>,
    ) -> Self {
        let len = pool.accounts.len();
        Self {
            provider_name,
            strategy: pool.strategy,
            accounts: pool.accounts,
            index: AtomicUsize::new(0),
            cooldown_until: Arc::new(Mutex::new(HashMap::new())),
            cache: Mutex::new(HashMap::new()),
            weighted_state: Mutex::new(WeightedState::new(len)),
            runtime,
            default_api_url,
        }
    }

    fn select_account_index(&self) -> Result<usize> {
        if self.accounts.is_empty() {
            anyhow::bail!("provider account pool is empty for {}", self.provider_name);
        }

        let eligible = self.eligible_indices();
        if eligible.is_empty() {
            anyhow::bail!(
                "no enabled provider accounts available for {}",
                self.provider_name
            );
        }

        match self.strategy {
            AccountPoolStrategy::RoundRobin => self.select_round_robin(&eligible),
            AccountPoolStrategy::WeightedRoundRobin => self.select_weighted(&eligible),
        }
    }

    fn eligible_indices(&self) -> Vec<usize> {
        let now = Instant::now();
        let mut cooldowns = self.cooldown_until.lock();
        cooldowns.retain(|_, until| *until > now);

        self.accounts
            .iter()
            .enumerate()
            .filter(|(_, account)| {
                account.enabled && !cooldowns.contains_key(&account.id)
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    fn select_round_robin(&self, eligible: &[usize]) -> Result<usize> {
        let len = self.accounts.len();
        let start = self.index.fetch_add(1, Ordering::Relaxed);

        (0..len)
            .map(|offset| (start + offset) % len)
            .find(|idx| eligible.contains(idx))
            .ok_or_else(|| {
                anyhow::anyhow!("no eligible provider accounts for {}", self.provider_name)
            })
    }

    fn select_weighted(&self, eligible: &[usize]) -> Result<usize> {
        let mut state = self.weighted_state.lock();
        if state.current.len() != self.accounts.len() {
            *state = WeightedState::new(self.accounts.len());
        }

        let mut total_weight: i64 = 0;
        let mut selected: Option<usize> = None;
        let mut selected_weight: i64 = i64::MIN;

        for &idx in eligible {
            let weight = i64::from(self.accounts[idx].weight);
            total_weight += weight;
            state.current[idx] += weight;
            if state.current[idx] > selected_weight {
                selected_weight = state.current[idx];
                selected = Some(idx);
            }
        }

        let selected = selected.ok_or_else(|| {
            anyhow::anyhow!("no eligible provider accounts for {}", self.provider_name)
        })?;

        state.current[selected] -= total_weight;
        Ok(selected)
    }

    fn provider_for_account(&self, account: &ProviderAccountConfig) -> Result<Arc<dyn Provider>> {
        if let Some(provider) = self.cache.lock().get(&account.id) {
            return Ok(Arc::clone(provider));
        }

        let api_url = account
            .api_url
            .as_deref()
            .or(self.default_api_url.as_deref());
        let provider = super::create_provider_for_pool(
            &self.provider_name,
            Some(account.api_key.as_str()),
            api_url,
            &self.runtime,
        )?;
        let provider = Arc::from(provider);
        self.cache
            .lock()
            .insert(account.id.clone(), Arc::clone(&provider));
        Ok(provider)
    }

    fn mark_cooldown(&self, account_id: &str, err: &anyhow::Error) {
        if !is_rate_limited(err) {
            return;
        }

        mark_cooldown_inner(&self.cooldown_until, account_id, err);
    }

    async fn with_account<T, F, Fut>(&self, f: F) -> Result<T>
    where
        F: FnOnce(Arc<dyn Provider>, &ProviderAccountConfig) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let idx = self.select_account_index()?;
        let account = self.accounts[idx].clone();
        let provider = self.provider_for_account(&account)?;

        let result = f(provider, &account).await;
        if let Err(err) = &result {
            self.mark_cooldown(&account.id, err);
        }
        result.with_context(|| {
            format!(
                "provider {} account {} request failed",
                self.provider_name, account.id
            )
        })
    }

    fn provider_capabilities(&self) -> Option<super::traits::ProviderCapabilities> {
        let account = self
            .accounts
            .iter()
            .find(|account| account.enabled)
            .or_else(|| self.accounts.first())?;
        let provider = self.provider_for_account(account).ok()?;
        Some(provider.capabilities())
    }
}

#[async_trait::async_trait]
impl Provider for AccountPoolProvider {
    fn capabilities(&self) -> super::traits::ProviderCapabilities {
        self.provider_capabilities().unwrap_or_default()
    }

    fn supports_native_tools(&self) -> bool {
        self.provider_capabilities()
            .is_some_and(|caps| caps.native_tool_calling)
    }

    fn convert_tools(&self, tools: &[crate::tools::ToolSpec]) -> ToolsPayload {
        let account = self
            .accounts
            .iter()
            .find(|account| account.enabled)
            .or_else(|| self.accounts.first());

        if let Some(account) = account {
            if let Ok(provider) = self.provider_for_account(account) {
                return provider.convert_tools(tools);
            }
        }

        ToolsPayload::PromptGuided {
            instructions: build_tool_instructions_text(tools),
        }
    }

    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> Result<String> {
        self.with_account(|provider, _account| async move {
            provider
                .chat_with_system(system_prompt, message, model, temperature)
                .await
        })
        .await
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> Result<String> {
        self.with_account(|provider, _account| async move {
            provider.chat_with_history(messages, model, temperature).await
        })
        .await
    }

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[serde_json::Value],
        model: &str,
        temperature: f64,
    ) -> Result<ChatResponse> {
        self.with_account(|provider, _account| async move {
            provider
                .chat_with_tools(messages, tools, model, temperature)
                .await
        })
        .await
    }

    fn supports_streaming(&self) -> bool {
        self.accounts
            .iter()
            .find(|account| account.enabled)
            .and_then(|account| self.provider_for_account(account).ok())
            .is_some_and(|provider| provider.supports_streaming())
    }

    fn stream_chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
        options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        let account = match self.select_account_index() {
            Ok(idx) => self.accounts[idx].clone(),
            Err(err) => {
                let message = err.to_string();
                return stream::once(async move { Err(StreamError::Provider(message)) }).boxed();
            }
        };

        let provider = match self.provider_for_account(&account) {
            Ok(provider) => provider,
            Err(err) => {
                let message = err.to_string();
                return stream::once(async move { Err(StreamError::Provider(message)) }).boxed();
            }
        };

        let cooldowns = Arc::clone(&self.cooldown_until);
        let account_id = account.id.clone();

        provider.stream_chat_with_system(
            system_prompt,
            message,
            model,
            temperature,
            options,
        )
        .map(move |item| {
            if let Err(err) = &item {
                mark_stream_cooldown(&cooldowns, &account_id, err);
            }
            item
        })
        .boxed()
    }

    fn stream_chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
        options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        let account = match self.select_account_index() {
            Ok(idx) => self.accounts[idx].clone(),
            Err(err) => {
                let message = err.to_string();
                return stream::once(async move { Err(StreamError::Provider(message)) }).boxed();
            }
        };

        let provider = match self.provider_for_account(&account) {
            Ok(provider) => provider,
            Err(err) => {
                let message = err.to_string();
                return stream::once(async move { Err(StreamError::Provider(message)) }).boxed();
            }
        };

        let cooldowns = Arc::clone(&self.cooldown_until);
        let account_id = account.id.clone();

        provider
            .stream_chat_with_history(messages, model, temperature, options)
            .map(move |item| {
                if let Err(err) = &item {
                    mark_stream_cooldown(&cooldowns, &account_id, err);
                }
                item
            })
            .boxed()
    }
}

fn is_rate_limited(err: &anyhow::Error) -> bool {
    if err
        .downcast_ref::<reqwest::Error>()
        .and_then(|reqwest_err| reqwest_err.status())
        .is_some_and(|status| status.as_u16() == 429)
    {
        return true;
    }
    message_indicates_rate_limit(&err.to_string())
}

fn parse_retry_after_ms(err: &anyhow::Error) -> Option<u64> {
    parse_retry_after_ms_from_message(&err.to_string())
}

fn parse_retry_after_ms_from_message(message: &str) -> Option<u64> {
    const RETRY_AFTER_PREFIXES: [&str; 4] = [
        "retry-after:",
        "retry_after:",
        "retry-after ",
        "retry_after ",
    ];

    let lower = message.to_lowercase();

    RETRY_AFTER_PREFIXES
        .iter()
        .find_map(|prefix| parse_retry_after_with_prefix(message, &lower, prefix))
}

fn parse_retry_after_with_prefix(msg: &str, lower: &str, prefix: &str) -> Option<u64> {
    let pos = lower.find(prefix)?;
    let after = &msg[pos + prefix.len()..];
    let secs = parse_retry_after_seconds(after)?;
    secs_to_millis(secs)
}

fn message_indicates_rate_limit(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("429")
        && (lower.contains("too many") || lower.contains("rate") || lower.contains("limit"))
}

fn mark_cooldown_inner(
    cooldowns: &Mutex<HashMap<String, Instant>>,
    account_id: &str,
    err: &anyhow::Error,
) {
    let cooldown_ms = parse_retry_after_ms(err)
        .unwrap_or(DEFAULT_COOLDOWN_MS)
        .min(MAX_COOLDOWN_MS);
    let until = Instant::now() + Duration::from_millis(cooldown_ms);
    cooldowns.lock().insert(account_id.to_string(), until);
}

fn mark_stream_cooldown(
    cooldowns: &Mutex<HashMap<String, Instant>>,
    account_id: &str,
    err: &StreamError,
) {
    let is_rate_limited = match err {
        StreamError::Http(http_err) => http_err
            .status()
            .is_some_and(|status| status.as_u16() == 429),
        StreamError::Provider(message) => message_indicates_rate_limit(message),
        _ => message_indicates_rate_limit(&err.to_string()),
    };

    if !is_rate_limited {
        return;
    }

    let cooldown_ms = parse_retry_after_ms_from_message(&err.to_string())
        .unwrap_or(DEFAULT_COOLDOWN_MS)
        .min(MAX_COOLDOWN_MS);
    let until = Instant::now() + Duration::from_millis(cooldown_ms);
    cooldowns.lock().insert(account_id.to_string(), until);
}

fn parse_retry_after_seconds(input: &str) -> Option<f64> {
    let num_str: String = input
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let secs = num_str.parse::<f64>().ok()?;
    if secs.is_finite() && secs >= 0.0 {
        Some(secs)
    } else {
        None
    }
}

fn secs_to_millis(secs: f64) -> Option<u64> {
    let millis = Duration::from_secs_f64(secs).as_millis();
    u64::try_from(millis).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_account(id: &str, weight: u32) -> ProviderAccountConfig {
        ProviderAccountConfig {
            id: id.to_string(),
            api_key: format!("key-{id}"),
            api_url: None,
            weight,
            enabled: true,
        }
    }

    fn round_robin_provider() -> AccountPoolProvider {
        AccountPoolProvider::new(
            "test-provider".into(),
            ProviderAccountPoolConfig {
                strategy: AccountPoolStrategy::RoundRobin,
                accounts: vec![sample_account("a", 1), sample_account("b", 1)],
            },
            ProviderRuntimeOptions::default(),
            None,
        )
    }

    #[test]
    fn round_robin_selects_alternating_accounts() {
        let provider = round_robin_provider();
        let first = provider.select_account_index().unwrap();
        let second = provider.select_account_index().unwrap();

        assert_ne!(first, second);
        assert_eq!(provider.accounts[first].id, "a");
        assert_eq!(provider.accounts[second].id, "b");
    }

    #[test]
    fn weighted_round_robin_respects_weights() {
        let provider = AccountPoolProvider::new(
            "test-provider".into(),
            ProviderAccountPoolConfig {
                strategy: AccountPoolStrategy::WeightedRoundRobin,
                accounts: vec![sample_account("a", 2), sample_account("b", 1)],
            },
            ProviderRuntimeOptions::default(),
            None,
        );

        let sequence: Vec<String> = (0..5)
            .map(|_| provider.accounts[provider.select_account_index().unwrap()].id.clone())
            .collect();

        assert_eq!(sequence, vec!["a", "b", "a", "a", "b"]);
    }

    #[test]
    fn cooldown_skips_rate_limited_account() {
        let provider = round_robin_provider();
        provider.cooldown_until.lock().insert(
            "a".to_string(),
            Instant::now() + Duration::from_secs(60),
        );

        let selected = provider.select_account_index().unwrap();
        assert_eq!(provider.accounts[selected].id, "b");
    }

    #[test]
    fn single_account_pool_selects_deterministically() {
        let provider = AccountPoolProvider::new(
            "test-provider".into(),
            ProviderAccountPoolConfig {
                strategy: AccountPoolStrategy::RoundRobin,
                accounts: vec![sample_account("solo", 1)],
            },
            ProviderRuntimeOptions::default(),
            None,
        );

        let selections: Vec<usize> = (0..5)
            .map(|_| provider.select_account_index().unwrap())
            .collect();

        assert!(selections.iter().all(|idx| *idx == 0));
        assert_eq!(provider.accounts[0].id, "solo");
    }

    #[test]
    fn provider_cache_is_account_bound() {
        let provider = round_robin_provider();
        let account_a = provider.accounts[0].clone();
        let account_b = provider.accounts[1].clone();

        let first_a = provider.provider_for_account(&account_a).unwrap();
        let second_a = provider.provider_for_account(&account_a).unwrap();
        let first_b = provider.provider_for_account(&account_b).unwrap();

        assert!(Arc::ptr_eq(&first_a, &second_a));
        assert!(!Arc::ptr_eq(&first_a, &first_b));
    }
}
