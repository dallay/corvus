use super::embeddings::EmbeddingProvider;
use super::traits::{Memory, MemoryCategory, MemoryEntry};
use super::vector;
use crate::config::MemoryConfig;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Local;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use tokio::sync::OnceCell;

pub struct SurrealMemory {
    client: OnceCell<Surreal<Client>>,
    ws_endpoint: String,
    namespace: String,
    database: String,
    username: Option<String>,
    password: Option<String>,
    token: Option<String>,
    embedder: Arc<dyn EmbeddingProvider>,
    vector_weight: f32,
    keyword_weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntryWrite {
    key: String,
    content: String,
    category: String,
    timestamp: String,
    session_id: Option<String>,
    embedding: Option<Vec<f32>>,
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct EntryRow {
    id: Thing,
    key: String,
    content: String,
    category: String,
    timestamp: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    embedding: Option<Vec<f32>>,
    #[serde(default)]
    updated_at: Option<String>,
}

impl SurrealMemory {
    pub fn new(
        _workspace_dir: &Path,
        config: &MemoryConfig,
        embedder: Arc<dyn EmbeddingProvider>,
        vector_weight: f32,
        keyword_weight: f32,
    ) -> Result<Self> {
        let raw_endpoint = config
            .surreal
            .url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:8000".to_string());
        let endpoint = parse_endpoint(&raw_endpoint)?;
        validate_endpoint_security(&endpoint, config.surreal.allow_http_loopback)?;

        let ws_endpoint = normalize_ws_endpoint(endpoint)?;
        let namespace = non_empty_or_default(config.surreal.namespace.clone(), "corvus");
        let database = non_empty_or_default(config.surreal.database.clone(), "memory");
        let username = trim_optional(config.surreal.username.clone());
        let password = trim_optional(config.surreal.password.clone());
        let token = trim_optional(config.surreal.token.clone());

        if username.as_deref() == Some("root") && !is_loopback_host_from_endpoint(&ws_endpoint) {
            tracing::warn!(
                "SurrealDB backend is configured with root credentials on a non-loopback host"
            );
        }

        Ok(Self {
            client: OnceCell::new(),
            ws_endpoint,
            namespace,
            database,
            username,
            password,
            token,
            embedder,
            vector_weight,
            keyword_weight,
        })
    }

    async fn client(&self) -> Result<&Surreal<Client>> {
        self.client
            .get_or_try_init(|| {
                let ws_endpoint = self.ws_endpoint.clone();
                let namespace = self.namespace.clone();
                let database = self.database.clone();
                let username = self.username.clone();
                let password = self.password.clone();
                let token = self.token.clone();

                async move {
                    let db = Surreal::new::<Ws>(ws_endpoint.as_str())
                        .await
                        .context("failed to connect to SurrealDB")?;

                    if let Some(jwt) = token {
                        db.authenticate(jwt)
                            .await
                            .context("failed SurrealDB token authentication")?;
                    } else if let (Some(user), Some(pass)) = (username, password) {
                        db.signin(Root {
                            username: &user,
                            password: &pass,
                        })
                        .await
                        .context("failed SurrealDB username/password authentication")?;
                    }

                    db.use_ns(namespace)
                        .use_db(database)
                        .await
                        .context("failed selecting SurrealDB namespace/database")?;

                    Self::ensure_schema(&db).await?;
                    Ok(db)
                }
            })
            .await
    }

    async fn ensure_schema(db: &Surreal<Client>) -> Result<()> {
        let statements = [
            "DEFINE TABLE memory_entries SCHEMALESS;",
            "DEFINE TABLE memory_events SCHEMALESS;",
            "DEFINE TABLE memory_relations SCHEMALESS;",
            "DEFINE INDEX idx_memory_entries_key ON TABLE memory_entries COLUMNS key UNIQUE;",
            "DEFINE INDEX idx_memory_entries_category ON TABLE memory_entries COLUMNS category;",
            "DEFINE INDEX idx_memory_entries_session ON TABLE memory_entries COLUMNS session_id;",
            "DEFINE INDEX idx_memory_entries_updated ON TABLE memory_entries COLUMNS updated_at;",
        ];

        for statement in statements {
            if let Err(error) = db.query(statement).await {
                let message = error.to_string();
                if message.to_ascii_lowercase().contains("already exists") {
                    tracing::debug!("SurrealDB schema statement already exists: {statement}");
                    continue;
                }
                tracing::warn!("SurrealDB schema statement failed: {statement}; error: {message}");
                return Err(error).context("failed applying SurrealDB schema");
            }
        }

        Ok(())
    }

    fn category_to_str(category: &MemoryCategory) -> String {
        match category {
            MemoryCategory::Core => "core".to_string(),
            MemoryCategory::Daily => "daily".to_string(),
            MemoryCategory::Conversation => "conversation".to_string(),
            MemoryCategory::Custom(value) => value.clone(),
        }
    }

    fn str_to_category(value: &str) -> MemoryCategory {
        match value {
            "core" => MemoryCategory::Core,
            "daily" => MemoryCategory::Daily,
            "conversation" => MemoryCategory::Conversation,
            custom => MemoryCategory::Custom(custom.to_string()),
        }
    }

    fn record_id_for_key(key: &str) -> String {
        let digest = Sha256::digest(key.as_bytes());
        hex::encode(digest)
    }

    fn row_to_entry(row: EntryRow, score: Option<f64>) -> MemoryEntry {
        MemoryEntry {
            id: row.id.to_string(),
            key: row.key,
            content: row.content,
            category: Self::str_to_category(&row.category),
            timestamp: row.updated_at.unwrap_or(row.timestamp),
            session_id: row.session_id,
            score,
        }
    }

    async fn list_rows(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EntryRow>> {
        let db = self.client().await?;
        let mut query = "SELECT * FROM memory_entries".to_string();
        let mut clauses = Vec::new();
        if category.is_some() {
            clauses.push("category = $category");
        }
        if session_id.is_some() {
            clauses.push("session_id = $session_id");
        }
        if !clauses.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&clauses.join(" AND "));
        }
        query.push_str(" ORDER BY updated_at DESC LIMIT $limit;");

        let mut request = db.query(query).bind(("limit", limit));
        if let Some(value) = category {
            request = request.bind(("category", Self::category_to_str(value)));
        }
        if let Some(sid) = session_id {
            request = request.bind(("session_id", sid.to_string()));
        }

        let mut response = request.await.context("failed to list SurrealDB entries")?;
        let rows: Vec<EntryRow> = response
            .take(0)
            .context("failed to decode SurrealDB entries")?;
        Ok(rows)
    }

    async fn recall_rows(
        &self,
        query_lower: &str,
        session_id: Option<&str>,
    ) -> Result<Vec<EntryRow>> {
        let db = self.client().await?;
        let mut request = db.query(
            "SELECT * FROM memory_entries
             WHERE (
                string::contains(string::lowercase(key), $query)
                OR string::contains(string::lowercase(content), $query)
             )
             AND ($session_id = NONE OR session_id = $session_id)
             ORDER BY updated_at DESC
             LIMIT 500;",
        );
        request = request.bind(("query", query_lower.to_string()));
        request = request.bind(("session_id", session_id.map(str::to_string)));

        let mut response = request
            .await
            .context("failed to recall SurrealDB entries")?;
        let rows: Vec<EntryRow> = response
            .take(0)
            .context("failed to decode SurrealDB recall entries")?;
        Ok(rows)
    }

    async fn count_rows(&self) -> Result<usize> {
        #[derive(Debug, Deserialize)]
        struct CountRow {
            count: usize,
        }

        let db = self.client().await?;
        let mut response = db
            .query("SELECT count() AS count FROM memory_entries GROUP ALL;")
            .await
            .context("failed to count SurrealDB entries")?;
        let rows: Vec<CountRow> = response
            .take(0)
            .context("failed to decode SurrealDB count result")?;
        Ok(rows.first().map_or(0, |row| row.count))
    }

    async fn log_event(
        &self,
        action: &str,
        entry_id: &str,
        key: &str,
        category: &MemoryCategory,
        session_id: Option<&str>,
    ) -> Result<()> {
        let db = self.client().await?;
        db.query(
            "CREATE memory_events CONTENT {
                action: $action,
                entry_id: $entry_id,
                key: $key,
                category: $category,
                session_id: $session_id,
                at: $at
            };",
        )
        .bind(("action", action.to_string()))
        .bind(("entry_id", entry_id.to_string()))
        .bind(("key", key.to_string()))
        .bind(("category", Self::category_to_str(category)))
        .bind(("session_id", session_id.map(str::to_string)))
        .bind(("at", Local::now().to_rfc3339()))
        .await
        .context("failed to log SurrealDB episodic event")?;
        Ok(())
    }
}

#[async_trait]
impl Memory for SurrealMemory {
    fn name(&self) -> &str {
        "surreal"
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> Result<()> {
        if key.trim().is_empty() {
            anyhow::bail!("memory key cannot be empty");
        }
        if content.trim().is_empty() {
            anyhow::bail!("memory content cannot be empty");
        }

        let record_id = Self::record_id_for_key(key);
        let db = self.client().await?;

        let embedding = if self.embedder.dimensions() > 0 {
            Some(
                self.embedder
                    .embed_one(content)
                    .await
                    .context("failed generating embedding for SurrealDB memory")?,
            )
        } else {
            None
        };

        let now = Local::now().to_rfc3339();
        let payload = EntryWrite {
            key: key.to_string(),
            content: content.to_string(),
            category: Self::category_to_str(&category),
            timestamp: now.clone(),
            session_id: session_id.map(str::to_string),
            embedding,
            updated_at: now,
        };

        let category_value = Self::category_to_str(&category);
        let category_node = format!("category:{category_value}");
        let session_node = session_id.map(|sid| format!("session:{sid}"));

        let mut tx = db.query(
            "BEGIN TRANSACTION;
             LET $existing = SELECT id FROM memory_entries WHERE key = $key LIMIT 1;
             LET $action = IF array::len($existing) > 0 THEN \"update\" ELSE \"store\" END;
             UPSERT type::thing(\"memory_entries\", $record_id) CONTENT $payload;
             CREATE memory_events CONTENT {
                action: $action,
                entry_id: $record_id,
                key: $key,
                category: $category,
                session_id: $session_id,
                at: $at
             };
             CREATE memory_relations CONTENT {
                relation_type: \"entry_category\",
                from_entry_id: $record_id,
                to_node_id: $category_node,
                session_id: $session_id,
                at: $at
             };
             IF $session_id != NONE THEN (
                CREATE memory_relations CONTENT {
                   relation_type: \"entry_session\",
                   from_entry_id: $record_id,
                   to_node_id: $session_node,
                   session_id: $session_id,
                   at: $at
                };
                LET $previous = SELECT id FROM memory_entries
                                WHERE session_id = $session_id AND key != $key
                                ORDER BY updated_at DESC
                                LIMIT 1;
                IF array::len($previous) > 0 THEN (
                   CREATE memory_relations CONTENT {
                      relation_type: \"entry_previous\",
                      from_entry_id: $record_id,
                      to_node_id: string::concat(\"entry:\", <string>$previous[0].id),
                      session_id: $session_id,
                      at: $at
                   };
                );
             );
             COMMIT TRANSACTION;",
        );
        tx = tx.bind(("key", key.to_string()));
        tx = tx.bind(("record_id", record_id.clone()));
        tx = tx.bind(("payload", payload));
        tx = tx.bind(("category", category_value));
        tx = tx.bind(("session_id", session_id.map(str::to_string)));
        tx = tx.bind(("at", Local::now().to_rfc3339()));
        tx = tx.bind(("category_node", category_node));
        tx = tx.bind(("session_node", session_node));
        tx.await.context("failed transactional SurrealDB store")?;

        Ok(())
    }

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let query_lower = query.to_lowercase();
        let terms: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|term| !term.is_empty())
            .collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }

        let keyword_candidates = self.recall_rows(&query_lower, session_id).await?;
        let vector_candidates = self.list_rows(None, session_id, 500).await?;
        let mut candidate_by_id: HashMap<String, EntryRow> =
            HashMap::with_capacity(keyword_candidates.len() + vector_candidates.len());
        for row in keyword_candidates.into_iter().chain(vector_candidates) {
            let id = row.id.to_string();
            candidate_by_id.entry(id).or_insert(row);
        }

        let mut vector_results: Vec<(String, f32)> = Vec::new();
        let mut keyword_results: Vec<(String, f32)> = Vec::new();
        let mut row_by_id: HashMap<String, EntryRow> =
            HashMap::with_capacity(candidate_by_id.len());
        let mut searchable_by_id: HashMap<String, String> =
            HashMap::with_capacity(candidate_by_id.len());

        let query_embedding = if self.embedder.dimensions() > 0 {
            Some(
                self.embedder
                    .embed_one(query)
                    .await
                    .context("failed generating recall embedding for SurrealDB")?,
            )
        } else {
            None
        };

        for mut row in candidate_by_id.into_values() {
            let id = row.id.to_string();
            let searchable = format!("{} {}", row.key.to_lowercase(), row.content.to_lowercase());

            let keyword_matches = terms
                .iter()
                .filter(|term| searchable.contains(**term))
                .count();
            if keyword_matches > 0 {
                #[allow(clippy::cast_possible_truncation)]
                let mut score = keyword_matches as f32 / terms.len() as f32;
                if searchable.contains(&query_lower) {
                    score = (score + 0.2).min(1.0);
                }
                keyword_results.push((id.clone(), score));
            }

            if let Some(ref qv) = query_embedding {
                if row.embedding.is_none() {
                    match self.embedder.embed_one(&row.content).await {
                        Ok(generated) => {
                            row.embedding = Some(generated);
                        }
                        Err(error) => {
                            tracing::debug!(
                                "failed generating candidate embedding in surreal recall for key '{}': {error}",
                                row.key
                            );
                        }
                    }
                }
                if let Some(ev) = row.embedding.as_ref() {
                    let sim = vector::cosine_similarity(qv, ev);
                    if sim > 0.0 {
                        vector_results.push((id.clone(), sim));
                    }
                }
            }

            searchable_by_id.insert(id.clone(), searchable);
            row_by_id.insert(id, row);
        }

        let merged = vector::hybrid_merge(
            &vector_results,
            &keyword_results,
            self.vector_weight,
            self.keyword_weight,
            limit,
        );

        if merged.is_empty() {
            let mut fallback = Vec::new();
            for (id, row) in row_by_id {
                if searchable_by_id
                    .get(&id)
                    .is_some_and(|searchable| searchable.contains(&query_lower))
                {
                    fallback.push(Self::row_to_entry(row, Some(1.0)));
                    if fallback.len() >= limit {
                        break;
                    }
                }
            }
            return Ok(fallback);
        }

        let mut output = Vec::with_capacity(merged.len());
        for item in merged {
            if let Some(row) = row_by_id.remove(&item.id) {
                output.push(Self::row_to_entry(row, Some(f64::from(item.final_score))));
            }
        }
        Ok(output)
    }

    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
        let db = self.client().await?;
        let record_id = Self::record_id_for_key(key);

        let row: Option<EntryRow> = db
            .select(("memory_entries", record_id.as_str()))
            .await
            .context("failed selecting SurrealDB memory entry")?;
        Ok(row.map(|value| Self::row_to_entry(value, None)))
    }

    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        let rows = self.list_rows(category, session_id, 1_000).await?;
        let entries = rows
            .into_iter()
            .map(|row| Self::row_to_entry(row, None))
            .collect();
        Ok(entries)
    }

    async fn forget(&self, key: &str) -> Result<bool> {
        let db = self.client().await?;
        let record_id = Self::record_id_for_key(key);

        let deleted: Option<EntryRow> = db
            .delete(("memory_entries", record_id.as_str()))
            .await
            .context("failed deleting SurrealDB memory entry")?;

        if deleted.is_some() {
            self.log_event(
                "forget",
                &record_id,
                key,
                &MemoryCategory::Custom("deleted".into()),
                None,
            )
            .await?;
        }

        Ok(deleted.is_some())
    }

    async fn count(&self) -> Result<usize> {
        self.count_rows().await
    }

    async fn health_check(&self) -> bool {
        let Ok(db) = self.client().await else {
            return false;
        };
        db.query("RETURN 1;").await.is_ok()
    }
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn non_empty_or_default(value: Option<String>, default_value: &str) -> String {
    trim_optional(value).unwrap_or_else(|| default_value.to_string())
}

fn parse_endpoint(raw_endpoint: &str) -> Result<Url> {
    let raw = raw_endpoint.trim();
    if raw.is_empty() {
        anyhow::bail!("SurrealDB endpoint URL cannot be empty");
    }

    Url::parse(raw)
        .or_else(|_| Url::parse(&format!("http://{raw}")))
        .with_context(|| format!("invalid SurrealDB endpoint URL '{raw_endpoint}'"))
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host == "[::1]"
}

fn validate_endpoint_security(endpoint: &Url, allow_http_loopback: bool) -> Result<()> {
    let scheme = endpoint.scheme();
    let host = endpoint.host_str().unwrap_or_default();

    match scheme {
        "https" | "wss" => Ok(()),
        "http" | "ws" => {
            if !allow_http_loopback {
                anyhow::bail!(
                    "refusing insecure SurrealDB URL: {scheme} is disabled (set \
                     memory.surreal.allow_http_loopback=true for localhost only)"
                );
            }
            if !is_loopback_host(host) {
                anyhow::bail!(
                    "refusing insecure SurrealDB URL over {scheme} for non-loopback host '{host}'"
                );
            }
            Ok(())
        }
        other => anyhow::bail!(
            "unsupported SurrealDB URL scheme '{other}', expected one of http/https/ws/wss"
        ),
    }
}

fn normalize_ws_endpoint(mut endpoint: Url) -> Result<String> {
    match endpoint.scheme() {
        "http" => endpoint
            .set_scheme("ws")
            .map_err(|_| anyhow::anyhow!("failed converting http endpoint to ws"))?,
        "https" => endpoint
            .set_scheme("wss")
            .map_err(|_| anyhow::anyhow!("failed converting https endpoint to wss"))?,
        "ws" | "wss" => {}
        other => anyhow::bail!(
            "unsupported SurrealDB URL scheme '{other}', expected one of http/https/ws/wss"
        ),
    }

    if endpoint.path().is_empty() || endpoint.path() == "/" {
        endpoint.set_path("/rpc");
    }

    Ok(endpoint.to_string())
}

fn is_loopback_host_from_endpoint(endpoint: &str) -> bool {
    Url::parse(endpoint)
        .ok()
        .and_then(|url| url.host_str().map(ToOwned::to_owned))
        .is_some_and(|host| is_loopback_host(&host))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MemoryConfig, SurrealMemoryConfig};
    use crate::memory::embeddings::NoopEmbedding;
    use tempfile::TempDir;

    #[test]
    fn normalize_http_to_ws_rpc() {
        let url = parse_endpoint("http://127.0.0.1:8000").unwrap();
        let normalized = normalize_ws_endpoint(url).unwrap();
        assert_eq!(normalized, "ws://127.0.0.1:8000/rpc");
    }

    #[test]
    fn reject_plain_http_non_loopback() {
        let endpoint = parse_endpoint("http://10.0.0.7:8000").unwrap();
        let err = validate_endpoint_security(&endpoint, true).unwrap_err();
        assert!(err.to_string().contains("non-loopback"));
    }

    #[test]
    fn reject_plain_ws_non_loopback() {
        let endpoint = parse_endpoint("ws://10.0.0.7:8000/rpc").unwrap();
        let err = validate_endpoint_security(&endpoint, true).unwrap_err();
        assert!(err.to_string().contains("non-loopback"));
    }

    #[test]
    fn reject_http_when_loopback_override_disabled() {
        let endpoint = parse_endpoint("http://127.0.0.1:8000").unwrap();
        let err = validate_endpoint_security(&endpoint, false).unwrap_err();
        assert!(err.to_string().contains("http is disabled"));
    }

    #[tokio::test]
    async fn backend_smoke_store_and_get_when_test_server_available() {
        let Ok(endpoint) = std::env::var("CORVUS_TEST_SURREALDB_URL") else {
            return;
        };

        let tmp = TempDir::new().unwrap();
        let mut config = MemoryConfig {
            backend: "surreal".to_string(),
            ..MemoryConfig::default()
        };
        config.surreal = SurrealMemoryConfig {
            url: Some(endpoint),
            namespace: Some(format!("test_ns_{}", uuid::Uuid::new_v4())),
            database: Some(format!("test_db_{}", uuid::Uuid::new_v4())),
            username: std::env::var("CORVUS_TEST_SURREALDB_USERNAME").ok(),
            password: std::env::var("CORVUS_TEST_SURREALDB_PASSWORD").ok(),
            token: std::env::var("CORVUS_TEST_SURREALDB_TOKEN").ok(),
            allow_http_loopback: true,
        };

        let memory =
            SurrealMemory::new(tmp.path(), &config, Arc::new(NoopEmbedding), 0.7, 0.3).unwrap();

        memory
            .store(
                "smoke_key",
                "surreal smoke content",
                MemoryCategory::Core,
                Some("s1"),
            )
            .await
            .unwrap();
        let got = memory.get("smoke_key").await.unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().key, "smoke_key");
        assert!(memory.count().await.unwrap() >= 1);
    }
}
