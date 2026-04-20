//! CRUD operations for [`ModelRoute`] backed by the `model_routes` table.

use crate::db::SqliteDb;
use crate::domain::{ModelRoute, PoolId, RouteId, RookError};
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

// ── Row mapping ───────────────────────────────────────────────────────────────

/// Minimal JSON structure stored in the `policy` column.
///
/// The PRD notes that full [`RoutingPolicy`](crate::domain::RoutingPolicy)
/// wiring is a future concern. For now we persist only `capability_constraints`
/// so it survives a round-trip through the DB without loss.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct StoredPolicy {
    #[serde(default)]
    capability_constraints: Vec<String>,
}

fn row_to_route(row: &sqlx::sqlite::SqliteRow) -> Result<ModelRoute, RookError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RookError::Registry(format!("missing route id: {e}")))?;
    let id = RouteId::new(
        Uuid::parse_str(&id_str)
            .map_err(|e| RookError::Registry(format!("invalid route UUID: {e}")))?,
    );

    let logical_model: String = row
        .try_get("logical_model")
        .map_err(|e| RookError::Registry(format!("missing logical_model: {e}")))?;

    let target_str: String = row
        .try_get("target_pool_id")
        .map_err(|e| RookError::Registry(format!("missing target_pool_id: {e}")))?;
    let target_pool_id = PoolId::new(
        Uuid::parse_str(&target_str)
            .map_err(|e| {
                RookError::Registry(format!("invalid target_pool_id UUID: {e}"))
            })?,
    );

    let fallback_str: Option<String> = row
        .try_get("fallback_route_id")
        .map_err(|e| RookError::Registry(format!("missing fallback_route_id: {e}")))?;
    let fallback_route_id = fallback_str
        .map(|s| {
            Uuid::parse_str(&s)
                .map(RouteId::new)
                .map_err(|e| {
                    RookError::Registry(format!("invalid fallback_route_id UUID: {e}"))
                })
        })
        .transpose()?;

    let policy_json: String = row
        .try_get("policy")
        .map_err(|e| RookError::Registry(format!("missing policy: {e}")))?;
    let policy: StoredPolicy = serde_json::from_str(&policy_json)
        .map_err(|e| RookError::Registry(format!("invalid policy JSON: {e}; policy_json={policy_json}")))?;

    Ok(ModelRoute {
        id,
        logical_model,
        target_pool_id,
        fallback_route_id,
        capability_constraints: policy.capability_constraints,
    })
}

// ── CRUD impl ─────────────────────────────────────────────────────────────────

impl SqliteDb {
    /// Persist a new [`ModelRoute`].
    pub async fn insert_route(&self, route: &ModelRoute) -> Result<(), RookError> {
        let id = route.id.to_string();
        let target_pool_id = route.target_pool_id.to_string();
        let fallback_route_id = route.fallback_route_id.as_ref().map(|r| r.to_string());

        // Validate that fallback_route_id exists if provided
        if let Some(fallback_id) = &route.fallback_route_id {
            let exists = sqlx::query("SELECT 1 FROM model_routes WHERE id = ?")
                .bind(fallback_id.to_string())
                .fetch_optional(self.pool())
                .await
                .map_err(|e| RookError::Registry(format!("failed to validate fallback_route_id: {e}")))?;

            if exists.is_none() {
                return Err(RookError::Registry(format!(
                    "fallback_route_id {} does not exist",
                    fallback_id
                )));
            }
        }

        let policy = StoredPolicy {
            capability_constraints: route.capability_constraints.clone(),
        };
        let policy_json = serde_json::to_string(&policy)
            .map_err(|e| RookError::Registry(format!("failed to serialize policy: {e}")))?;

        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO model_routes \
             (id, logical_model, target_pool_id, fallback_route_id, policy, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&route.logical_model)
        .bind(&target_pool_id)
        .bind(&fallback_route_id)
        .bind(&policy_json)
        .bind(&now)
        .bind(&now)
        .execute(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("insert_route failed: {e}")))?;

        Ok(())
    }

    /// Fetch a [`ModelRoute`] by its ID.
    pub async fn get_route(&self, id: &RouteId) -> Result<Option<ModelRoute>, RookError> {
        let id_str = id.to_string();
        let row = sqlx::query(
            "SELECT id, logical_model, target_pool_id, fallback_route_id, policy, \
             created_at, updated_at \
             FROM model_routes WHERE id = ?",
        )
        .bind(&id_str)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("get_route failed: {e}")))?;

        row.map(|r| row_to_route(&r)).transpose()
    }

    /// Find a [`ModelRoute`] by logical model name (e.g., `"gpt-4o"`).
    pub async fn find_route_by_model(
        &self,
        logical_model: &str,
    ) -> Result<Option<ModelRoute>, RookError> {
        let row = sqlx::query(
            "SELECT id, logical_model, target_pool_id, fallback_route_id, policy, \
             created_at, updated_at \
             FROM model_routes WHERE logical_model = ?",
        )
        .bind(logical_model)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("find_route_by_model failed: {e}")))?;

        row.map(|r| row_to_route(&r)).transpose()
    }

    /// Return all [`ModelRoute`]s ordered by logical model name.
    pub async fn list_routes(&self) -> Result<Vec<ModelRoute>, RookError> {
        let rows = sqlx::query(
            "SELECT id, logical_model, target_pool_id, fallback_route_id, policy, \
             created_at, updated_at \
             FROM model_routes ORDER BY logical_model ASC",
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("list_routes failed: {e}")))?;

        rows.iter().map(row_to_route).collect()
    }

    /// Delete a [`ModelRoute`] by ID.
    ///
    /// Returns `true` if a row was deleted, `false` if not found.
    pub async fn delete_route(&self, id: &RouteId) -> Result<bool, RookError> {
        let id_str = id.to_string();

        // Check if any routes reference this one as a fallback
        let referencing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM model_routes WHERE fallback_route_id = ? LIMIT 1"
        )
        .bind(&id_str)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("failed to check route references: {e}")))?;

        if let Some((referring_id,)) = referencing {
            return Err(RookError::Registry(format!(
                "cannot delete route {}: referenced by route {} as fallback",
                id, referring_id
            )));
        }

        let result = sqlx::query("DELETE FROM model_routes WHERE id = ?")
            .bind(&id_str)
            .execute(self.pool())
            .await
            .map_err(|e| RookError::Registry(format!("delete_route failed: {e}")))?;

        Ok(result.rows_affected() > 0)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AccountId, PoolId, ProviderAccount, ProviderPool, ProviderVendor, RouteId,
        SelectionStrategy,
    };

    async fn make_db_with_pool() -> (SqliteDb, PoolId) {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let account = ProviderAccount {
            id: AccountId::generate(),
            display_name: "OpenAI".to_string(),
            vendor: ProviderVendor::OpenAi,
            api_base_override: None,
            enabled: true,
            weight: 100,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        };
        db.insert_account(&account).await.unwrap();

        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "Main Pool".to_string(),
            strategy: SelectionStrategy::RoundRobin,
            members: vec![account.id],
            fallback_pool_id: None,
        };
        db.insert_pool(&pool).await.unwrap();

        (db, pool.id)
    }

    fn make_route(target_pool_id: PoolId) -> ModelRoute {
        ModelRoute {
            id: RouteId::generate(),
            logical_model: "gpt-4o".to_string(),
            target_pool_id,
            fallback_route_id: None,
            capability_constraints: vec![],
        }
    }

    #[tokio::test]
    async fn insert_and_get_route_round_trips() {
        let (db, pool_id) = make_db_with_pool().await;
        let route = make_route(pool_id);

        db.insert_route(&route).await.unwrap();

        let fetched = db.get_route(&route.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, route.id);
        assert_eq!(fetched.logical_model, route.logical_model);
        assert_eq!(fetched.target_pool_id, route.target_pool_id);
        assert_eq!(fetched.fallback_route_id, route.fallback_route_id);
        assert_eq!(fetched.capability_constraints, route.capability_constraints);
    }

    #[tokio::test]
    async fn insert_route_with_capability_constraints_round_trips() {
        let (db, pool_id) = make_db_with_pool().await;
        let route = ModelRoute {
            id: RouteId::generate(),
            logical_model: "claude-3-sonnet".to_string(),
            target_pool_id: pool_id,
            fallback_route_id: None,
            capability_constraints: vec!["vision".to_string(), "function_calling".to_string()],
        };
        db.insert_route(&route).await.unwrap();

        let fetched = db.get_route(&route.id).await.unwrap().unwrap();
        assert_eq!(fetched.capability_constraints, route.capability_constraints);
    }

    #[tokio::test]
    async fn find_route_by_model_finds_correct_route() {
        let (db, pool_id) = make_db_with_pool().await;
        let route = make_route(pool_id);
        db.insert_route(&route).await.unwrap();

        let found = db.find_route_by_model("gpt-4o").await.unwrap().unwrap();
        assert_eq!(found.id, route.id);
    }

    #[tokio::test]
    async fn find_route_by_model_returns_none_for_unknown_model() {
        let (db, _) = make_db_with_pool().await;
        let result = db.find_route_by_model("claude-3-opus").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_routes_returns_all_inserted() {
        let (db, pool_id) = make_db_with_pool().await;

        let r1 = make_route(pool_id);
        let r2 = ModelRoute {
            id: RouteId::generate(),
            logical_model: "claude-3-sonnet".to_string(),
            target_pool_id: pool_id,
            fallback_route_id: None,
            capability_constraints: vec!["vision".to_string()],
        };

        db.insert_route(&r1).await.unwrap();
        db.insert_route(&r2).await.unwrap();

        let routes = db.list_routes().await.unwrap();
        assert_eq!(routes.len(), 2);

        let ids: Vec<_> = routes.iter().map(|r| r.id).collect();
        assert!(ids.contains(&r1.id));
        assert!(ids.contains(&r2.id));
    }

    #[tokio::test]
    async fn delete_route_returns_true_and_removes_row() {
        let (db, pool_id) = make_db_with_pool().await;
        let route = make_route(pool_id);
        db.insert_route(&route).await.unwrap();

        let deleted = db.delete_route(&route.id).await.unwrap();
        assert!(deleted);

        assert!(db.get_route(&route.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_route_returns_false_for_missing_id() {
        let (db, _) = make_db_with_pool().await;
        let missing = RouteId::generate();
        let deleted = db.delete_route(&missing).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn insert_route_with_nonexistent_fallback_fails() {
        let (db, pool_id) = make_db_with_pool().await;
        let route = ModelRoute {
            id: RouteId::generate(),
            logical_model: "gpt-4o".to_string(),
            target_pool_id: pool_id,
            fallback_route_id: Some(RouteId::generate()), // doesn't exist
            capability_constraints: vec![],
        };

        let result = db.insert_route(&route).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn delete_route_referenced_as_fallback_fails() {
        let (db, pool_id) = make_db_with_pool().await;

        // Create fallback route first
        let fallback_route = ModelRoute {
            id: RouteId::generate(),
            logical_model: "gpt-4o-fallback".to_string(),
            target_pool_id: pool_id,
            fallback_route_id: None,
            capability_constraints: vec![],
        };
        db.insert_route(&fallback_route).await.unwrap();

        // Create primary route that references the fallback
        let primary_route = ModelRoute {
            id: RouteId::generate(),
            logical_model: "gpt-4o".to_string(),
            target_pool_id: pool_id,
            fallback_route_id: Some(fallback_route.id),
            capability_constraints: vec![],
        };
        db.insert_route(&primary_route).await.unwrap();

        // Try to delete the fallback route - should fail
        let result = db.delete_route(&fallback_route.id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("referenced by"));
    }

    #[tokio::test]
    async fn insert_route_with_existing_fallback_succeeds() {
        let (db, pool_id) = make_db_with_pool().await;

        // Create fallback route first
        let fallback_route = ModelRoute {
            id: RouteId::generate(),
            logical_model: "gpt-4o-fallback".to_string(),
            target_pool_id: pool_id,
            fallback_route_id: None,
            capability_constraints: vec![],
        };
        db.insert_route(&fallback_route).await.unwrap();

        // Create primary route that references the fallback
        let primary_route = ModelRoute {
            id: RouteId::generate(),
            logical_model: "gpt-4o".to_string(),
            target_pool_id: pool_id,
            fallback_route_id: Some(fallback_route.id),
            capability_constraints: vec![],
        };
        db.insert_route(&primary_route).await.unwrap();

        // Verify it was created
        let fetched = db.get_route(&primary_route.id).await.unwrap().unwrap();
        assert_eq!(fetched.fallback_route_id, Some(fallback_route.id));
    }
}