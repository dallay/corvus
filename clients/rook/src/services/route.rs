//! Route service — port and in-memory implementation for [`ModelRoute`]
//! lifecycle management and logical-model resolution.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use crate::domain::{ModelRoute, RookError, RouteId};

// ── Port ─────────────────────────────────────────────────────────────────────

/// Port for managing [`ModelRoute`] lifecycle and resolution.
pub trait RouteService: Send + Sync {
    /// Return all routes.
    fn list(&self) -> impl Future<Output = Vec<ModelRoute>> + Send;

    /// Return a single route by ID, or `None` if not found.
    fn get(&self, id: RouteId) -> impl Future<Output = Option<ModelRoute>> + Send;

    /// Resolve a logical model name to its active route, or `None` if no route
    /// is configured for that model.
    fn resolve(
        &self,
        logical_model: &str,
    ) -> impl Future<Output = Option<ModelRoute>> + Send;

    /// Persist a new route and return its assigned [`RouteId`].
    fn create(
        &self,
        route: ModelRoute,
    ) -> impl Future<Output = Result<RouteId, RookError>> + Send;

    /// Overwrite an existing route.
    ///
    /// Returns [`RookError::Registry`] if the ID is unknown.
    fn update(&self, route: ModelRoute) -> impl Future<Output = Result<(), RookError>> + Send;

    /// Remove a route by ID.
    ///
    /// Returns [`RookError::Registry`] if the ID is unknown.
    fn delete(&self, id: RouteId) -> impl Future<Output = Result<(), RookError>> + Send;
}

// ── In-memory implementation ──────────────────────────────────────────────────

/// In-memory [`RouteService`] backed by a `HashMap`.
///
/// No persistence — used for tests and bootstrap scenarios.
#[derive(Debug, Default)]
pub struct InMemoryRouteService {
    pub(crate) store: Arc<Mutex<HashMap<RouteId, ModelRoute>>>,
}

impl InMemoryRouteService {
    /// Create an empty service.
    pub fn new() -> Self {
        Self::default()
    }
}

impl RouteService for InMemoryRouteService {
    async fn list(&self) -> Vec<ModelRoute> {
        self.store
            .lock()
            .map(|g| g.values().cloned().collect())
            .unwrap_or_default()
    }

    async fn get(&self, id: RouteId) -> Option<ModelRoute> {
        self.store.lock().ok()?.get(&id).cloned()
    }

    async fn resolve(&self, logical_model: &str) -> Option<ModelRoute> {
        let guard = self.store.lock().ok()?;
        let matches: Vec<ModelRoute> = guard
            .values()
            .filter(|r| r.logical_model == logical_model)
            .cloned()
            .collect();

        // Only return a route if exactly one is found
        if matches.len() == 1 { Some(matches[0].clone()) } else { None }
    }

    async fn create(&self, route: ModelRoute) -> Result<RouteId, RookError> {
        let id = route.id;
        let mut guard = self
            .store
            .lock()
            .map_err(|e| RookError::Registry(e.to_string()))?;

        // Check for duplicate route ID
        if guard.contains_key(&id) {
            return Err(RookError::Registry(format!("duplicate route id {}", id)));
        }

        // Check for duplicate logical_model
        for existing in guard.values() {
            if existing.logical_model == route.logical_model {
                return Err(RookError::Registry(format!(
                    "route for logical_model '{}' already exists",
                    route.logical_model
                )));
            }
        }

        guard.insert(id, route);
        Ok(id)
    }

    async fn update(&self, route: ModelRoute) -> Result<(), RookError> {
        let mut guard =
            self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;

        // Verify route exists
        if !guard.contains_key(&route.id) {
            return Err(RookError::Registry(format!("route {} not found", route.id)));
        }

        // Check that no OTHER route has the same logical_model
        for (existing_id, existing_route) in guard.iter() {
            if *existing_id != route.id && existing_route.logical_model == route.logical_model {
                return Err(RookError::Registry(format!(
                    "route for logical_model '{}' already exists (id: {})",
                    route.logical_model, existing_id
                )));
            }
        }

        guard.insert(route.id, route);
        Ok(())
    }

    async fn delete(&self, id: RouteId) -> Result<(), RookError> {
        let mut guard =
            self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        if guard.remove(&id).is_none() {
            return Err(RookError::Registry(format!("route {id} not found")));
        }
        Ok(())
    }
}

// ── SQLite implementation ─────────────────────────────────────────────────────

/// SQLite-backed [`RouteService`].
///
/// Delegates all storage to the Rook [`crate::db::SqliteDb`] connection pool.
#[derive(Clone, Debug)]
pub struct SqliteRouteService {
    db: crate::db::SqliteDb,
}

impl SqliteRouteService {
    /// Wrap an existing [`crate::db::SqliteDb`].
    pub fn new(db: crate::db::SqliteDb) -> Self {
        Self { db }
    }
}

impl RouteService for SqliteRouteService {
    async fn list(&self) -> Vec<ModelRoute> {
        self.db.list_routes().await.unwrap_or_default()
    }

    async fn get(&self, id: RouteId) -> Option<ModelRoute> {
        self.db.get_route(&id).await.ok().flatten()
    }

    async fn resolve(&self, logical_model: &str) -> Option<ModelRoute> {
        self.db.find_route_by_model(logical_model).await.ok().flatten()
    }

    async fn create(&self, route: ModelRoute) -> Result<RouteId, RookError> {
        let id = route.id;
        self.db.insert_route(&route).await?;
        Ok(id)
    }

    async fn update(&self, route: ModelRoute) -> Result<(), RookError> {
        // No update_route in db layer — implement as delete + re-insert.
        self.db.delete_route(&route.id).await.map(|_| ())?;
        self.db.insert_route(&route).await
    }

    async fn delete(&self, id: RouteId) -> Result<(), RookError> {
        self.db.delete_route(&id).await.map(|_| ())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::PoolId;

    fn make_route(logical_model: &str) -> ModelRoute {
        ModelRoute {
            id: RouteId::generate(),
            logical_model: logical_model.to_owned(),
            target_pool_id: PoolId::generate(),
            fallback_route_id: None,
            capability_constraints: vec![],
        }
    }

    #[tokio::test]
    async fn crud_round_trip() {
        let svc = InMemoryRouteService::new();
        let route = make_route("gpt-4o");
        let id = route.id;

        // Create
        let returned_id = svc.create(route.clone()).await.unwrap();
        assert_eq!(returned_id, id);

        // Read
        let fetched = svc.get(id).await.unwrap();
        assert_eq!(fetched.logical_model, "gpt-4o");

        // List
        assert_eq!(svc.list().await.len(), 1);

        // Update
        let mut updated = fetched.clone();
        updated.logical_model = "gpt-4o-mini".to_owned();
        svc.update(updated).await.unwrap();
        assert_eq!(svc.get(id).await.unwrap().logical_model, "gpt-4o-mini");

        // Delete
        svc.delete(id).await.unwrap();
        assert!(svc.get(id).await.is_none());
        assert!(svc.list().await.is_empty());
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let svc = InMemoryRouteService::new();
        assert!(svc.get(RouteId::generate()).await.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent_returns_error() {
        let svc = InMemoryRouteService::new();
        let err = svc.delete(RouteId::generate()).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn update_nonexistent_returns_error() {
        let svc = InMemoryRouteService::new();
        let route = make_route("claude-3-opus");
        let err = svc.update(route).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn resolve_finds_route_by_logical_model() {
        let svc = InMemoryRouteService::new();
        let route = make_route("claude-3-5-sonnet");
        svc.create(route).await.unwrap();

        let found = svc.resolve("claude-3-5-sonnet").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().logical_model, "claude-3-5-sonnet");
    }

    #[tokio::test]
    async fn resolve_returns_none_for_unknown_model() {
        let svc = InMemoryRouteService::new();
        assert!(svc.resolve("no-such-model").await.is_none());
    }

    #[tokio::test]
    async fn create_duplicate_route_id_returns_error() {
        let svc = InMemoryRouteService::new();
        let route = make_route("gpt-4o");

        // First create should succeed
        svc.create(route.clone()).await.unwrap();

        // Second create with same ID should fail
        let err = svc.create(route).await.unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }

    #[tokio::test]
    async fn create_duplicate_logical_model_returns_error() {
        let svc = InMemoryRouteService::new();
        let route1 = make_route("gpt-4o");
        let mut route2 = make_route("gpt-4o");
        route2.id = RouteId::generate(); // different ID, same logical_model

        // First create should succeed
        svc.create(route1).await.unwrap();

        // Second create with same logical_model should fail
        let err = svc.create(route2).await.unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn update_with_duplicate_logical_model_returns_error() {
        let svc = InMemoryRouteService::new();
        let route1 = make_route("gpt-4o");
        let route2 = make_route("claude-3-opus");

        svc.create(route1.clone()).await.unwrap();
        svc.create(route2.clone()).await.unwrap();

        // Try to update route2 to have the same logical_model as route1
        let mut updated_route2 = route2.clone();
        updated_route2.logical_model = "gpt-4o".to_string();

        let err = svc.update(updated_route2).await.unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn resolve_with_multiple_matches_returns_none() {
        let svc = InMemoryRouteService::new();

        // This shouldn't happen in practice due to create() validation,
        // but resolve should handle it gracefully
        let route1 = make_route("gpt-4o");
        let mut route2 = make_route("gpt-4o");
        route2.id = RouteId::generate();

        // Manually insert both (bypassing create validation)
        {
            let mut guard = svc.store.lock().unwrap();
            guard.insert(route1.id, route1);
            guard.insert(route2.id, route2);
        }

        // resolve should return None when multiple routes match
        assert!(svc.resolve("gpt-4o").await.is_none());
    }
}
