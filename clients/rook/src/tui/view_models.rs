use crate::admin::types::{AccountView, HealthAccountView, HealthSummaryView, PoolView, RouteView};
use crate::domain::ProviderVendor;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusViewModel {
    pub total_accounts: usize,
    pub enabled_accounts: usize,
    pub disabled_accounts: usize,
    pub provider_count: usize,
    pub provider_groups: Vec<ProviderGroupSummary>,
    pub health_summary: HealthSummaryView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderGroupSummary {
    pub vendor: String,
    pub total_accounts: usize,
    pub enabled_accounts: usize,
    pub disabled_accounts: usize,
    pub healthy_accounts: usize,
    pub degraded_accounts: usize,
    pub unhealthy_accounts: usize,
    pub unknown_accounts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvidersViewModel {
    pub groups: Vec<ProviderAccountGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAccountGroup {
    pub vendor: String,
    pub accounts: Vec<ProviderAccountRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAccountRow {
    pub account: AccountView,
    pub health: Option<HealthAccountView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolsViewModel {
    pub pools: Vec<PoolRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolRow {
    pub pool: PoolView,
    pub member_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthViewModel {
    pub summary: HealthSummaryView,
    pub rows: Vec<HealthAccountView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutesViewModel {
    pub rows: Vec<RouteRow>,
    pub selected_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRow {
    pub route: RouteView,
    pub target_pool_label: String,
    pub fallback_route_label: Option<String>,
}

impl RoutesViewModel {
    pub fn select_next(&mut self) {
        if !self.rows.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.rows.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.rows.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.rows.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }
}

fn vendor_label(vendor: &ProviderVendor) -> String {
    match vendor {
        ProviderVendor::OpenAi => "open_ai".to_string(),
        ProviderVendor::Anthropic => "anthropic".to_string(),
        ProviderVendor::Google => "google".to_string(),
        ProviderVendor::OpenRouter => "open_router".to_string(),
        ProviderVendor::DeepSeek => "deep_seek".to_string(),
        ProviderVendor::Other(value) => value.clone(),
    }
}

fn health_bucket(status: &str, summary: &mut ProviderGroupSummary) {
    match status {
        "healthy" => summary.healthy_accounts += 1,
        "degraded" => summary.degraded_accounts += 1,
        "unhealthy" => summary.unhealthy_accounts += 1,
        _ => summary.unknown_accounts += 1,
    }
}

pub fn build_status_view(
    accounts: &[AccountView],
    health_rows: &[HealthAccountView],
    health_summary: HealthSummaryView,
) -> StatusViewModel {
    let mut summaries: BTreeMap<String, ProviderGroupSummary> = BTreeMap::new();
    let health_by_account = health_rows
        .iter()
        .map(|row| (row.account_id, row))
        .collect::<std::collections::HashMap<_, _>>();

    for account in accounts {
        let vendor = vendor_label(&account.vendor);
        let group = summaries
            .entry(vendor.clone())
            .or_insert_with(|| ProviderGroupSummary {
                vendor,
                total_accounts: 0,
                enabled_accounts: 0,
                disabled_accounts: 0,
                healthy_accounts: 0,
                degraded_accounts: 0,
                unhealthy_accounts: 0,
                unknown_accounts: 0,
            });

        group.total_accounts += 1;
        if account.enabled {
            group.enabled_accounts += 1;
        } else {
            group.disabled_accounts += 1;
        }

        if let Some(health) = health_by_account.get(&account.id) {
            health_bucket(&health.status, group);
        } else {
            group.unknown_accounts += 1;
        }
    }

    StatusViewModel {
        total_accounts: accounts.len(),
        enabled_accounts: accounts.iter().filter(|account| account.enabled).count(),
        disabled_accounts: accounts.iter().filter(|account| !account.enabled).count(),
        provider_count: summaries.len(),
        provider_groups: summaries.into_values().collect(),
        health_summary,
    }
}

pub fn build_providers_view(
    accounts: &[AccountView],
    health_rows: &[HealthAccountView],
) -> ProvidersViewModel {
    let mut grouped: BTreeMap<String, Vec<ProviderAccountRow>> = BTreeMap::new();
    let health_by_account = health_rows
        .iter()
        .cloned()
        .map(|row| (row.account_id, row))
        .collect::<std::collections::HashMap<_, _>>();

    for account in accounts {
        grouped
            .entry(vendor_label(&account.vendor))
            .or_default()
            .push(ProviderAccountRow {
                account: account.clone(),
                health: health_by_account.get(&account.id).cloned(),
            });
    }

    ProvidersViewModel {
        groups: grouped
            .into_iter()
            .map(|(vendor, mut rows)| {
                rows.sort_by(|left, right| {
                    left.account.display_name.cmp(&right.account.display_name)
                });
                ProviderAccountGroup {
                    vendor,
                    accounts: rows,
                }
            })
            .collect(),
    }
}

pub fn build_pools_view(accounts: &[AccountView], pools: &[PoolView]) -> PoolsViewModel {
    let account_labels = accounts
        .iter()
        .map(|account| (account.id, account.display_name.clone()))
        .collect::<std::collections::HashMap<_, _>>();

    PoolsViewModel {
        pools: pools
            .iter()
            .cloned()
            .map(|pool| PoolRow {
                member_labels: pool
                    .members
                    .iter()
                    .map(|member| {
                        account_labels
                            .get(member)
                            .cloned()
                            .unwrap_or_else(|| member.to_string())
                    })
                    .collect(),
                pool,
            })
            .collect(),
    }
}

pub fn build_health_view(
    summary: HealthSummaryView,
    mut rows: Vec<HealthAccountView>,
) -> HealthViewModel {
    rows.sort_by(|left, right| left.display_name.cmp(&right.display_name));
    HealthViewModel { summary, rows }
}

pub fn build_routes_view(mut rows: Vec<RouteRow>) -> RoutesViewModel {
    rows.sort_by(|left, right| left.route.logical_model.cmp(&right.route.logical_model));
    RoutesViewModel {
        rows,
        selected_index: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::types::{
        AccountView, HealthAccountView, HealthSummaryView, PoolView, RouteView,
    };
    use crate::domain::{AccountId, PoolId, ProviderVendor, RouteId, SelectionStrategy};

    fn account(name: &str, vendor: ProviderVendor, enabled: bool) -> AccountView {
        AccountView {
            id: AccountId::generate(),
            vendor,
            display_name: name.to_string(),
            api_base_override: None,
            has_api_key: true,
            enabled,
            weight: 1,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        }
    }

    fn health(account: &AccountView, status: &str) -> HealthAccountView {
        HealthAccountView {
            account_id: account.id,
            display_name: account.display_name.clone(),
            vendor: account.vendor.clone(),
            enabled: account.enabled,
            status: status.to_string(),
            last_checked: None,
            consecutive_failures: 0,
            cooldown_until: None,
            is_available: status != "unhealthy",
        }
    }

    #[test]
    fn groups_providers_from_vendor_values_and_counts_status_totals() {
        let openai_primary = account("OpenAI Primary", ProviderVendor::OpenAi, true);
        let openai_backup = account("OpenAI Backup", ProviderVendor::OpenAi, false);
        let anthropic = account("Anthropic", ProviderVendor::Anthropic, true);

        let status = build_status_view(
            &[
                openai_primary.clone(),
                openai_backup.clone(),
                anthropic.clone(),
            ],
            &[
                health(&openai_primary, "healthy"),
                health(&openai_backup, "unknown"),
                health(&anthropic, "degraded"),
            ],
            HealthSummaryView {
                total: 3,
                healthy: 1,
                degraded: 1,
                unhealthy: 0,
                unknown: 1,
            },
        );

        assert_eq!(status.total_accounts, 3);
        assert_eq!(status.enabled_accounts, 2);
        assert_eq!(status.disabled_accounts, 1);
        assert_eq!(status.provider_count, 2);
        assert_eq!(status.provider_groups[0].vendor, "anthropic");
        assert_eq!(status.provider_groups[1].vendor, "open_ai");
        assert_eq!(status.provider_groups[1].unknown_accounts, 1);
    }

    #[test]
    fn pools_view_uses_account_names_for_member_labels_and_falls_back_to_ids() {
        let alpha = account("Alpha", ProviderVendor::OpenAi, true);
        let missing_member = AccountId::generate();
        let pools = build_pools_view(
            std::slice::from_ref(&alpha),
            &[PoolView {
                id: PoolId::generate(),
                name: "Primary".to_string(),
                strategy: SelectionStrategy::Priority,
                members: vec![alpha.id, missing_member],
                fallback_pool_id: None,
            }],
        );

        assert_eq!(pools.pools[0].member_labels[0], "Alpha");
        assert_eq!(pools.pools[0].member_labels[1], missing_member.to_string());
    }

    #[test]
    fn providers_and_health_views_preserve_unknown_semantics() {
        let account = account("Unknown", ProviderVendor::Other("custom".to_string()), true);
        let providers = build_providers_view(
            std::slice::from_ref(&account),
            &[health(&account, "unknown")],
        );
        let health_view = build_health_view(
            HealthSummaryView {
                total: 1,
                healthy: 0,
                degraded: 0,
                unhealthy: 0,
                unknown: 1,
            },
            vec![health(&account, "unknown")],
        );

        assert_eq!(providers.groups[0].vendor, "custom");
        assert_eq!(
            providers.groups[0].accounts[0]
                .health
                .as_ref()
                .unwrap()
                .status,
            "unknown"
        );
        assert_eq!(health_view.summary.unknown, 1);
        assert_eq!(health_view.rows[0].status, "unknown");
    }

    #[test]
    fn routes_view_orders_rows_and_keeps_optional_relationships_bounded() {
        let fallback_id = RouteId::generate();
        let routes = build_routes_view(vec![
            RouteRow {
                route: RouteView {
                    id: RouteId::generate(),
                    logical_model: "z-model".to_string(),
                    target_pool_id: PoolId::generate(),
                    fallback_route_id: None,
                    capability_constraints: vec![],
                },
                target_pool_label: "pool-z".to_string(),
                fallback_route_label: None,
            },
            RouteRow {
                route: RouteView {
                    id: RouteId::generate(),
                    logical_model: "a-model".to_string(),
                    target_pool_id: PoolId::generate(),
                    fallback_route_id: Some(fallback_id),
                    capability_constraints: vec!["chat".to_string()],
                },
                target_pool_label: "pool-a".to_string(),
                fallback_route_label: Some("fallback-a".to_string()),
            },
        ]);

        assert_eq!(routes.rows[0].route.logical_model, "a-model");
        assert_eq!(routes.rows[0].target_pool_label, "pool-a");
        assert_eq!(
            routes.rows[0].fallback_route_label.as_deref(),
            Some("fallback-a")
        );
        assert_eq!(routes.rows[0].route.fallback_route_id, Some(fallback_id));
        assert_eq!(routes.rows[1].fallback_route_label, None);
        assert_eq!(routes.selected_index, 0);
    }

    #[test]
    fn routes_view_selection_wraps_and_preserves_fallback_to_ids_when_labels_missing() {
        let target_pool_id = PoolId::generate();
        let fallback_route_id = RouteId::generate();
        let mut routes = build_routes_view(vec![RouteRow {
            route: RouteView {
                id: RouteId::generate(),
                logical_model: "gpt-4o".to_string(),
                target_pool_id,
                fallback_route_id: Some(fallback_route_id),
                capability_constraints: vec![],
            },
            target_pool_label: target_pool_id.to_string(),
            fallback_route_label: Some(fallback_route_id.to_string()),
        }]);

        routes.select_next();
        assert_eq!(routes.selected_index, 0);
        routes.select_previous();
        assert_eq!(routes.selected_index, 0);
        assert_eq!(routes.rows[0].target_pool_label, target_pool_id.to_string());
        assert_eq!(
            routes.rows[0].fallback_route_label.as_deref(),
            Some(fallback_route_id.to_string().as_str())
        );
    }
}
