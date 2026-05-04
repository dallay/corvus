use crate::admin::handlers::{build_health_summary_view, list_health_account_views};
use crate::admin::types::{AccountView, HealthAccountView, HealthSummaryView, PoolView, RouteView};
use crate::domain::RookError;
use crate::registry::RookRegistry;
use crate::services::{
    account::AccountService as _, pool::PoolService as _, route::RouteService as _,
};
use crate::tui::view_models::{
    build_health_view, build_pools_view, build_providers_view, build_status_view, HealthViewModel,
    PoolsViewModel, ProvidersViewModel, RouteRow, RoutesViewModel, StatusViewModel,
};

#[derive(Clone)]
pub struct TuiQueryService {
    registry: RookRegistry,
}

impl TuiQueryService {
    pub fn new(registry: RookRegistry) -> Self {
        Self { registry }
    }

    pub async fn load_accounts(&self) -> Result<Vec<AccountView>, RookError> {
        Ok(self
            .registry
            .accounts()
            .list()
            .await
            .into_iter()
            .map(AccountView::from)
            .collect())
    }

    pub async fn load_pools(&self) -> Result<Vec<PoolView>, RookError> {
        Ok(self
            .registry
            .pools()
            .list()
            .await
            .into_iter()
            .map(PoolView::from)
            .collect())
    }

    pub async fn load_routes(&self) -> Result<Vec<RouteView>, RookError> {
        Ok(self
            .registry
            .routes()
            .list()
            .await
            .into_iter()
            .map(RouteView::from)
            .collect())
    }

    pub async fn load_route(
        &self,
        route_id: crate::domain::RouteId,
    ) -> Result<Option<RouteView>, RookError> {
        Ok(self
            .registry
            .routes()
            .get(route_id)
            .await
            .map(RouteView::from))
    }

    pub async fn load_health_rows(&self) -> Result<Vec<HealthAccountView>, RookError> {
        Ok(list_health_account_views(&self.registry).await)
    }

    pub async fn load_health_summary(&self) -> Result<HealthSummaryView, RookError> {
        Ok(build_health_summary_view(&self.registry).await)
    }

    pub async fn load_status_view(&self) -> Result<StatusViewModel, String> {
        let accounts = self.load_accounts().await.map_err(|err| err.to_string())?;
        let health_rows = self
            .load_health_rows()
            .await
            .map_err(|err| err.to_string())?;
        let health_summary = self
            .load_health_summary()
            .await
            .map_err(|err| err.to_string())?;
        Ok(build_status_view(&accounts, &health_rows, health_summary))
    }

    pub async fn load_providers_view(&self) -> Result<ProvidersViewModel, String> {
        let accounts = self.load_accounts().await.map_err(|err| err.to_string())?;
        let health_rows = self
            .load_health_rows()
            .await
            .map_err(|err| err.to_string())?;
        Ok(build_providers_view(&accounts, &health_rows))
    }

    pub async fn load_pools_view(&self) -> Result<PoolsViewModel, String> {
        let accounts = self.load_accounts().await.map_err(|err| err.to_string())?;
        let pools = self.load_pools().await.map_err(|err| err.to_string())?;
        Ok(build_pools_view(&accounts, &pools))
    }

    pub async fn load_health_view(&self) -> Result<HealthViewModel, String> {
        let summary = self
            .load_health_summary()
            .await
            .map_err(|err| err.to_string())?;
        let rows = self
            .load_health_rows()
            .await
            .map_err(|err| err.to_string())?;
        Ok(build_health_view(summary, rows))
    }

    pub async fn load_routes_view(&self) -> Result<RoutesViewModel, String> {
        let routes = self.load_routes().await.map_err(|err| err.to_string())?;
        let pools = self.load_pools().await.map_err(|err| err.to_string())?;
        let pool_labels = pools
            .into_iter()
            .map(|pool| (pool.id, pool.name))
            .collect::<std::collections::HashMap<_, _>>();
        let route_labels = routes
            .iter()
            .map(|route| (route.id, route.logical_model.clone()))
            .collect::<std::collections::HashMap<_, _>>();

        let rows = routes
            .into_iter()
            .map(|route| RouteRow {
                target_pool_label: pool_labels
                    .get(&route.target_pool_id)
                    .cloned()
                    .unwrap_or_else(|| route.target_pool_id.to_string()),
                fallback_route_label: route.fallback_route_id.map(|fallback_id| {
                    route_labels
                        .get(&fallback_id)
                        .cloned()
                        .unwrap_or_else(|| fallback_id.to_string())
                }),
                route,
            })
            .collect();

        Ok(crate::tui::view_models::build_routes_view(rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AccountId, ModelRoute, PoolId, ProviderAccount, ProviderPool, ProviderVendor, RouteId,
        SelectionStrategy,
    };
    use crate::services::health::HealthService as _;

    fn make_account(name: &str, vendor: ProviderVendor, enabled: bool) -> ProviderAccount {
        ProviderAccount {
            id: AccountId::generate(),
            vendor,
            display_name: name.to_string(),
            api_base_override: None,
            api_key: Some("sk-test".to_string()),
            enabled,
            weight: 1,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        }
    }

    fn make_route(
        logical_model: &str,
        target_pool_id: PoolId,
        fallback_route_id: Option<RouteId>,
        capability_constraints: Vec<&str>,
    ) -> ModelRoute {
        ModelRoute {
            id: RouteId::generate(),
            logical_model: logical_model.to_string(),
            target_pool_id,
            fallback_route_id,
            capability_constraints: capability_constraints
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }

    #[tokio::test]
    async fn query_service_loads_contract_bounded_status_providers_pools_and_health() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let openai = make_account("OpenAI", ProviderVendor::OpenAi, true);
        let openai_id = openai.id;
        let anthropic = make_account("Anthropic", ProviderVendor::Anthropic, false);
        let anthropic_id = anthropic.id;
        registry.accounts().create(openai).await.unwrap();
        registry.accounts().create(anthropic).await.unwrap();
        registry
            .pools()
            .create(ProviderPool {
                id: crate::domain::PoolId::generate(),
                name: "Primary".to_string(),
                strategy: SelectionStrategy::Priority,
                members: vec![openai_id, anthropic_id],
                fallback_pool_id: None,
            })
            .await
            .unwrap();
        registry.health().mark_success(openai_id).await;

        let query = TuiQueryService::new(registry);
        let status = query.load_status_view().await.unwrap();
        let providers = query.load_providers_view().await.unwrap();
        let pools = query.load_pools_view().await.unwrap();
        let health = query.load_health_view().await.unwrap();

        assert_eq!(status.total_accounts, 2);
        assert_eq!(status.disabled_accounts, 1);
        assert_eq!(providers.groups.len(), 2);
        assert_eq!(pools.pools[0].member_labels.len(), 2);
        assert!(pools.pools[0].member_labels.contains(&"OpenAI".to_string()));
        assert!(pools.pools[0]
            .member_labels
            .contains(&"Anthropic".to_string()));
        assert_eq!(health.summary.unknown, 1);
        assert_eq!(
            health
                .rows
                .iter()
                .find(|row| row.account_id == anthropic_id)
                .unwrap()
                .status,
            "unknown"
        );
    }

    #[tokio::test]
    async fn query_service_loads_routes_view_and_detail_without_inventing_missing_relationships() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let primary_pool_id = PoolId::generate();
        let backup_pool_id = PoolId::generate();
        registry
            .pools()
            .create(ProviderPool {
                id: primary_pool_id,
                name: "Primary".to_string(),
                strategy: SelectionStrategy::Priority,
                members: vec![],
                fallback_pool_id: None,
            })
            .await
            .unwrap();
        registry
            .pools()
            .create(ProviderPool {
                id: backup_pool_id,
                name: "Backup".to_string(),
                strategy: SelectionStrategy::Priority,
                members: vec![],
                fallback_pool_id: None,
            })
            .await
            .unwrap();

        let fallback_route = make_route("gpt-4o-fallback", backup_pool_id, None, vec![]);
        let fallback_route_id = fallback_route.id;
        let primary_route = make_route(
            "gpt-4o",
            primary_pool_id,
            Some(fallback_route_id),
            vec!["chat"],
        );
        let primary_route_id = primary_route.id;
        let no_fallback_route = make_route("claude-3-5-sonnet", primary_pool_id, None, vec![]);
        let no_fallback_route_id = no_fallback_route.id;

        registry.routes().create(fallback_route).await.unwrap();
        registry.routes().create(primary_route).await.unwrap();
        registry.routes().create(no_fallback_route).await.unwrap();

        let query = TuiQueryService::new(registry);
        let routes = query.load_routes_view().await.unwrap();
        let primary_detail = query.load_route(primary_route_id).await.unwrap().unwrap();
        let no_fallback_detail = query
            .load_route(no_fallback_route_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(routes.rows.len(), 3);
        assert_eq!(primary_detail.logical_model, "gpt-4o");
        assert_eq!(primary_detail.target_pool_id, primary_pool_id);
        assert_eq!(primary_detail.fallback_route_id, Some(fallback_route_id));
        assert_eq!(
            primary_detail.capability_constraints,
            vec!["chat".to_string()]
        );

        let no_fallback_row = routes
            .rows
            .iter()
            .find(|row| row.route.id == no_fallback_route_id)
            .unwrap();
        assert_eq!(no_fallback_row.target_pool_label, "Primary");
        assert_eq!(no_fallback_row.fallback_route_label, None);
        assert_eq!(no_fallback_detail.fallback_route_id, None);
    }
}
