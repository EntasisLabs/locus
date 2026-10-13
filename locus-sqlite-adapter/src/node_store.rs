use std::collections::BTreeSet;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use locus_core_rs::derive_tenant_id_from_session;
use locus_core_rs::domain::contracts::{NodeStore, NodeStoreInitializer};
use locus_core_rs::domain::models::{
    AvecState, BatchRekeyResult, ChangeQueryResult, NodeDeleteRecord, NodeDeleteRequest,
    NodeDeleteResult, NodeDeleteStatus, NodeQuery, NodeUpsertResult, NodeUpsertStatus,
    ScopeRekeyResult, SessionPurgeRequest, SttpNode, SyncCheckpoint, SyncCursor,
};
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, Row, params_from_iter};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::db::SqliteDatabase;
use crate::rank::rank_nodes_hybrid;

const DEFAULT_TENANT: &str = "default";
const NODE_COLUMNS: &str = "\
id, tenant_id, session_id, raw, tier, timestamp_ms, compression_depth, parent_node_id, \
sync_key, updated_at_ms, source_metadata, context_summary, semantic_tags, semantic_links, \
embedding, embedding_model, embedding_dimensions, embedded_at_ms, psi, rho, kappa, \
user_stability, user_friction, user_logic, user_autonomy, user_psi, \
model_stability, model_friction, model_logic, model_autonomy, model_psi, \
comp_stability, comp_friction, comp_logic, comp_autonomy, comp_psi";

const NODE_WRITE_COLUMNS: &[&str] = &[
    "tenant_id",
    "session_id",
    "raw",
    "tier",
    "timestamp_ms",
    "compression_depth",
    "parent_node_id",
    "sync_key",
    "updated_at_ms",
    "source_metadata",
    "context_summary",
    "semantic_tags",
    "semantic_links",
    "embedding",
    "embedding_model",
    "embedding_dimensions",
    "embedded_at_ms",
    "psi",
    "rho",
    "kappa",
    "user_stability",
    "user_friction",
    "user_logic",
    "user_autonomy",
    "user_psi",
    "model_stability",
    "model_friction",
    "model_logic",
    "model_autonomy",
    "model_psi",
    "comp_stability",
    "comp_friction",
    "comp_logic",
    "comp_autonomy",
    "comp_psi",
];

/// SQLite-backed node, calibration, and checkpoint store.
pub struct SqliteNodeStore {
    db: Arc<SqliteDatabase>,
}

impl SqliteNodeStore {
    pub fn new(db: Arc<SqliteDatabase>) -> Self {
        Self { db }
    }

    fn backfill_legacy_rows(&self) -> Result<()> {
        self.db.with_connection(|conn| {
            let tx = conn
                .unchecked_transaction()
                .context("failed to open legacy backfill transaction")?;
            let rows = {
                let mut stmt = tx.prepare(
                    "SELECT id, session_id, sync_key FROM temporal_node \
                     WHERE tenant_id = '' OR sync_key = ''",
                )?;
                let collected = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                collected
            };

            for (id, session_id, sync_key) in rows {
                let tenant_id = derive_tenant_id_from_session(&session_id);
                let sync_key = if sync_key.trim().is_empty() {
                    format!("legacy:{id}")
                } else {
                    sync_key
                };
                tx.execute(
                    "UPDATE temporal_node SET tenant_id = ?1, sync_key = ?2 WHERE id = ?3",
                    (&tenant_id, &sync_key, &id),
                )?;
            }

            tx.commit().context("failed to commit legacy backfill")?;
            Ok(())
        })
    }
}

#[async_trait]
impl NodeStoreInitializer for SqliteNodeStore {
    async fn initialize_async(&self) -> Result<()> {
        self.db.initialize()?;
        self.backfill_legacy_rows()?;
        Ok(())
    }
}

#[async_trait]
impl NodeStore for SqliteNodeStore {
    async fn query_nodes_async(&self, query: NodeQuery) -> Result<Vec<SttpNode>> {
        self.db.with_connection(|conn| query_nodes(conn, &query))
    }

    async fn upsert_node_async(&self, node: SttpNode) -> Result<NodeUpsertResult> {
        self.db.with_connection(|conn| upsert_node(conn, node))
    }

    async fn get_by_resonance_async(
        &self,
        session_id: &str,
        current_avec: AvecState,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<SttpNode>> {
        self.db.with_connection(|conn| {
            resonance(
                conn,
                Some(session_id),
                current_avec,
                from_utc,
                to_utc,
                tiers,
                limit,
            )
        })
    }

    async fn get_by_resonance_global_async(
        &self,
        current_avec: AvecState,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<SttpNode>> {
        self.db.with_connection(|conn| {
            resonance(conn, None, current_avec, from_utc, to_utc, tiers, limit)
        })
    }

    async fn get_by_hybrid_async(
        &self,
        session_id: &str,
        current_avec: AvecState,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        query_embedding: Option<&[f32]>,
        alpha: f32,
        beta: f32,
        limit: usize,
    ) -> Result<Vec<SttpNode>> {
        let candidate_limit = limit.max(1).saturating_mul(5);
        let mut candidates = self
            .get_by_resonance_async(
                session_id,
                current_avec,
                from_utc,
                to_utc,
                tiers,
                candidate_limit,
            )
            .await?;
        rank_nodes_hybrid(&mut candidates, current_avec, query_embedding, alpha, beta);
        candidates.truncate(limit.max(1));
        Ok(candidates)
    }

    async fn get_by_hybrid_global_async(
        &self,
        current_avec: AvecState,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        query_embedding: Option<&[f32]>,
        alpha: f32,
        beta: f32,
        limit: usize,
    ) -> Result<Vec<SttpNode>> {
        let candidate_limit = limit.max(1).saturating_mul(5);
        let mut candidates = self
            .get_by_resonance_global_async(current_avec, from_utc, to_utc, tiers, candidate_limit)
            .await?;
        rank_nodes_hybrid(&mut candidates, current_avec, query_embedding, alpha, beta);
        candidates.truncate(limit.max(1));
        Ok(candidates)
    }

    async fn list_nodes_async(
        &self,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<Vec<SttpNode>> {
        self.query_nodes_async(NodeQuery {
            limit: limit.clamp(1, 200),
            session_id: session_id.map(str::to_string),
            from_utc: None,
            to_utc: None,
            tiers: None,
        })
        .await
    }

    async fn get_last_avec_async(&self, session_id: &str) -> Result<Option<AvecState>> {
        let session_id = session_id.to_string();
        self.db.with_connection(|conn| {
            let tenant_id = derive_tenant_id_from_session(&session_id);
            let avec = conn
                .query_row(
                    "SELECT stability, friction, logic, autonomy FROM calibration \
                     WHERE session_id = ?1 AND (tenant_id = ?2 OR tenant_id IS NULL OR tenant_id = '') \
                     ORDER BY created_at_ms DESC, rowid DESC LIMIT 1",
                    (&session_id, &tenant_id),
                    |row| {
                        Ok(AvecState {
                            stability: row.get::<_, f64>(0)? as f32,
                            friction: row.get::<_, f64>(1)? as f32,
                            logic: row.get::<_, f64>(2)? as f32,
                            autonomy: row.get::<_, f64>(3)? as f32,
                        })
                    },
                )
                .optional()?;
            Ok(avec)
        })
    }

    async fn get_trigger_history_async(&self, session_id: &str) -> Result<Vec<String>> {
        let session_id = session_id.to_string();
        self.db.with_connection(|conn| {
            let tenant_id = derive_tenant_id_from_session(&session_id);
            let mut stmt = conn.prepare(
                "SELECT trigger FROM calibration \
                 WHERE session_id = ?1 AND (tenant_id = ?2 OR tenant_id IS NULL OR tenant_id = '') \
                 ORDER BY created_at_ms ASC, rowid ASC",
            )?;
            let triggers = stmt
                .query_map((&session_id, &tenant_id), |row| row.get(0))?
                .collect::<Result<Vec<String>, _>>()?;
            Ok(triggers)
        })
    }

    async fn store_calibration_async(
        &self,
        session_id: &str,
        avec: AvecState,
        trigger: &str,
    ) -> Result<()> {
        let session_id = session_id.to_string();
        let trigger = trigger.to_string();
        self.db.with_connection(|conn| {
            let tenant_id = derive_tenant_id_from_session(&session_id);
            conn.execute(
                "INSERT INTO calibration (
                    id, tenant_id, session_id, stability, friction, logic, autonomy, psi, trigger, created_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                (
                    Uuid::new_v4().simple().to_string(),
                    tenant_id,
                    session_id,
                    avec.stability as f64,
                    avec.friction as f64,
                    avec.logic as f64,
                    avec.autonomy as f64,
                    avec.psi() as f64,
                    trigger,
                    Utc::now().timestamp_millis(),
                ),
            )?;
            Ok(())
        })
    }

    async fn query_changes_since_async(
        &self,
        session_id: &str,
        cursor: Option<SyncCursor>,
        limit: usize,
    ) -> Result<ChangeQueryResult> {
        let session_id = session_id.to_string();
        self.db.with_connection(|conn| {
            let capped_limit = limit.max(1);
            let tenant_id = derive_tenant_id_from_session(&session_id);
            let mut params = vec![Value::Text(session_id), Value::Text(tenant_id)];
            let cursor_clause = if let Some(cursor) = cursor {
                params.push(Value::Integer(cursor.updated_at.timestamp_millis()));
                params.push(Value::Text(cursor.sync_key));
                "AND (updated_at_ms > ?3 OR (updated_at_ms = ?3 AND sync_key > ?4))"
            } else {
                ""
            };
            params.push(Value::Integer((capped_limit + 1) as i64));
            let limit_index = params.len();
            let sql = format!(
                "SELECT {NODE_COLUMNS} FROM temporal_node \
                 WHERE session_id = ?1 AND (tenant_id = ?2 OR tenant_id IS NULL OR tenant_id = '') \
                 {cursor_clause} \
                 ORDER BY updated_at_ms ASC, sync_key ASC \
                 LIMIT ?{limit_index}"
            );
            let mut nodes = load_nodes(conn, &sql, &params)?;
            let has_more = nodes.len() > capped_limit;
            if has_more {
                nodes.truncate(capped_limit);
            }
            let next_cursor = nodes.last().map(|node| SyncCursor {
                updated_at: node.updated_at,
                sync_key: node.sync_key.clone(),
            });
            Ok(ChangeQueryResult {
                nodes,
                next_cursor,
                has_more,
            })
        })
    }

    async fn get_checkpoint_async(
        &self,
        session_id: &str,
        connector_id: &str,
    ) -> Result<Option<SyncCheckpoint>> {
        let session_id = session_id.to_string();
        let connector_id = connector_id.to_string();
        self.db.with_connection(|conn| {
            let tenant_id = derive_tenant_id_from_session(&session_id);
            let checkpoint = conn
                .query_row(
                    "SELECT session_id, connector_id, cursor_updated_at_ms, cursor_sync_key, metadata, updated_at_ms \
                     FROM sync_checkpoint \
                     WHERE session_id = ?1 AND connector_id = ?2 \
                       AND (tenant_id = ?3 OR tenant_id IS NULL OR tenant_id = '') \
                     LIMIT 1",
                    (&session_id, &connector_id, &tenant_id),
                    map_checkpoint,
                )
                .optional()?;
            Ok(checkpoint)
        })
    }

    async fn put_checkpoint_async(&self, checkpoint: SyncCheckpoint) -> Result<()> {
        self.db.with_connection(|conn| {
            let tenant_id = derive_tenant_id_from_session(&checkpoint.session_id);
            let metadata = json_text(checkpoint.metadata.as_ref())?;
            let (cursor_updated_at, cursor_sync_key) = match checkpoint.cursor {
                Some(cursor) => (
                    Some(cursor.updated_at.timestamp_millis()),
                    Some(cursor.sync_key),
                ),
                None => (None, None),
            };
            conn.execute(
                "INSERT INTO sync_checkpoint (
                    id, tenant_id, session_id, connector_id, cursor_updated_at_ms, cursor_sync_key, metadata, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(tenant_id, session_id, connector_id) DO UPDATE SET
                    cursor_updated_at_ms = excluded.cursor_updated_at_ms,
                    cursor_sync_key = excluded.cursor_sync_key,
                    metadata = excluded.metadata,
                    updated_at_ms = excluded.updated_at_ms",
                (
                    Uuid::new_v4().simple().to_string(),
                    tenant_id,
                    checkpoint.session_id,
                    checkpoint.connector_id,
                    cursor_updated_at,
                    cursor_sync_key,
                    metadata,
                    checkpoint.updated_at.timestamp_millis(),
                ),
            )?;
            Ok(())
        })
    }

    async fn batch_rekey_scopes_async(
        &self,
        node_ids: Vec<String>,
        target_tenant_id: &str,
        target_session_id: &str,
        dry_run: bool,
        allow_merge: bool,
    ) -> Result<BatchRekeyResult> {
        let target_tenant_id = normalize_tenant_id(target_tenant_id);
        let target_session_id = target_session_id.to_string();
        self.db.with_connection(|conn| {
            rekey(
                conn,
                node_ids,
                &target_tenant_id,
                &target_session_id,
                dry_run,
                allow_merge,
            )
        })
    }

    async fn delete_nodes_async(&self, request: NodeDeleteRequest) -> Result<NodeDeleteResult> {
        self.db.with_connection(|conn| delete_nodes(conn, request))
    }

    async fn purge_session_async(&self, request: SessionPurgeRequest) -> Result<NodeDeleteResult> {
        self.db.with_connection(|conn| purge_session(conn, request))
    }
}

fn query_nodes(conn: &Connection, query: &NodeQuery) -> Result<Vec<SttpNode>> {
    let mut filter = NodeFilter::default();
    if let Some(session_id) = query
        .session_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        filter.session(session_id);
    }
    filter.time_range(query.from_utc, query.to_utc);
    filter.tiers(query.tiers.as_deref());
    let (where_sql, mut params) = filter.finish();
    params.push(Value::Integer(query.limit.max(1) as i64));
    let limit_index = params.len();
    let sql = format!(
        "SELECT {NODE_COLUMNS} FROM temporal_node {where_sql} \
         ORDER BY timestamp_ms DESC LIMIT ?{limit_index}"
    );
    load_nodes(conn, &sql, &params)
}

fn resonance(
    conn: &Connection,
    session_id: Option<&str>,
    current_avec: AvecState,
    from_utc: Option<DateTime<Utc>>,
    to_utc: Option<DateTime<Utc>>,
    tiers: Option<&[String]>,
    limit: usize,
) -> Result<Vec<SttpNode>> {
    let mut filter = NodeFilter::default();
    if let Some(session_id) = session_id {
        filter.session(session_id);
    }
    filter.time_range(from_utc, to_utc);
    filter.tiers(tiers);
    let (where_sql, mut params) = filter.finish();
    params.push(Value::Real(current_avec.stability as f64));
    params.push(Value::Real(current_avec.friction as f64));
    params.push(Value::Real(current_avec.logic as f64));
    params.push(Value::Real(current_avec.autonomy as f64));
    params.push(Value::Integer(limit.max(1) as i64));
    let distance_start = params.len() - 4;
    let limit_index = params.len();
    let sql = format!(
        "SELECT {NODE_COLUMNS} FROM temporal_node {where_sql} \
         ORDER BY (ABS(model_stability - ?{distance_start}) + ABS(model_friction - ?{}) + \
                    ABS(model_logic - ?{}) + ABS(model_autonomy - ?{})) / 4.0 ASC, \
                  updated_at_ms DESC \
         LIMIT ?{limit_index}",
        distance_start + 1,
        distance_start + 2,
        distance_start + 3,
    );
    load_nodes(conn, &sql, &params)
}

fn upsert_node(conn: &Connection, mut candidate: SttpNode) -> Result<NodeUpsertResult> {
    let compression = resolved_compression(&candidate);
    let sync_key = if candidate.sync_key.trim().is_empty() {
        candidate.canonical_sync_key()
    } else {
        candidate.sync_key.trim().to_string()
    };
    let updated_at = candidate.updated_at;
    candidate.sync_key = sync_key.clone();
    let tenant_id = derive_tenant_id_from_session(&candidate.session_id);

    let tx = conn
        .unchecked_transaction()
        .context("failed to open upsert transaction")?;

    if let Some((node_id, existing)) =
        find_existing(&tx, &tenant_id, &candidate.session_id, &sync_key)?
    {
        if nodes_equivalent(&existing, &candidate, compression) {
            tx.commit()?;
            return Ok(NodeUpsertResult {
                node_id,
                sync_key,
                status: NodeUpsertStatus::Duplicate,
                updated_at: existing.updated_at,
            });
        }

        update_node(&tx, &node_id, &tenant_id, &candidate, compression)?;
        tx.commit()?;
        return Ok(NodeUpsertResult {
            node_id,
            sync_key,
            status: NodeUpsertStatus::Updated,
            updated_at,
        });
    }

    let node_id = Uuid::new_v4().simple().to_string();
    match insert_node(&tx, &node_id, &tenant_id, &candidate, compression) {
        Ok(()) => {}
        Err(error) if is_unique_conflict(&error) => {
            let Some((node_id, existing)) =
                find_existing(&tx, &tenant_id, &candidate.session_id, &sync_key)?
            else {
                return Err(error);
            };
            if nodes_equivalent(&existing, &candidate, compression) {
                tx.commit()?;
                return Ok(NodeUpsertResult {
                    node_id,
                    sync_key,
                    status: NodeUpsertStatus::Duplicate,
                    updated_at: existing.updated_at,
                });
            }
            update_node(&tx, &node_id, &tenant_id, &candidate, compression)?;
            tx.commit()?;
            return Ok(NodeUpsertResult {
                node_id,
                sync_key,
                status: NodeUpsertStatus::Updated,
                updated_at,
            });
        }
        Err(error) => return Err(error),
    }

    tx.commit()?;
    Ok(NodeUpsertResult {
        node_id,
        sync_key,
        status: NodeUpsertStatus::Created,
        updated_at,
    })
}

fn find_existing(
    conn: &Connection,
    tenant_id: &str,
    session_id: &str,
    sync_key: &str,
) -> Result<Option<(String, SttpNode)>> {
    let sql = format!(
        "SELECT {NODE_COLUMNS} FROM temporal_node \
         WHERE session_id = ?1 AND sync_key = ?2 \
         ORDER BY CASE WHEN tenant_id = ?3 THEN 0 ELSE 1 END \
         LIMIT 1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let row = stmt
        .query_row(
            params_from_iter(
                [
                    Value::Text(session_id.to_string()),
                    Value::Text(sync_key.to_string()),
                    Value::Text(tenant_id.to_string()),
                ]
                .iter(),
            ),
            |row| Ok((row.get::<_, String>("id")?, map_node(row)?)),
        )
        .optional()?;
    Ok(row)
}

fn insert_node(
    conn: &Connection,
    node_id: &str,
    tenant_id: &str,
    node: &SttpNode,
    compression: AvecState,
) -> Result<()> {
    let columns = std::iter::once("id")
        .chain(NODE_WRITE_COLUMNS.iter().copied())
        .collect::<Vec<_>>()
        .join(", ");
    let placeholders = (1..=NODE_WRITE_COLUMNS.len() + 1)
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("INSERT INTO temporal_node ({columns}) VALUES ({placeholders})");
    let params = node_values(node_id, tenant_id, node, compression)?;
    conn.execute(&sql, params_from_iter(params.iter()))?;
    Ok(())
}

fn update_node(
    conn: &Connection,
    node_id: &str,
    tenant_id: &str,
    node: &SttpNode,
    compression: AvecState,
) -> Result<()> {
    let assignments = NODE_WRITE_COLUMNS
        .iter()
        .enumerate()
        .map(|(index, column)| format!("{column} = ?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let id_index = NODE_WRITE_COLUMNS.len() + 1;
    let sql = format!("UPDATE temporal_node SET {assignments} WHERE id = ?{id_index}");
    let mut params = node_values(node_id, tenant_id, node, compression)?;
    params.remove(0);
    params.push(Value::Text(node_id.to_string()));
    conn.execute(&sql, params_from_iter(params.iter()))?;
    Ok(())
}

fn node_values(
    node_id: &str,
    tenant_id: &str,
    node: &SttpNode,
    compression: AvecState,
) -> Result<Vec<Value>> {
    Ok(vec![
        Value::Text(node_id.to_string()),
        Value::Text(tenant_id.to_string()),
        Value::Text(node.session_id.clone()),
        Value::Text(node.raw.clone()),
        Value::Text(node.tier.clone()),
        Value::Integer(node.timestamp.timestamp_millis()),
        Value::Integer(node.compression_depth as i64),
        optional_text(node.parent_node_id.clone()),
        Value::Text(node.sync_key.clone()),
        Value::Integer(node.updated_at.timestamp_millis()),
        optional_text(json_text(node.source_metadata.as_ref())?),
        optional_text(node.context_summary.clone()),
        optional_text(json_text(node.semantic_tags.as_ref())?),
        optional_text(json_text(node.semantic_links.as_ref())?),
        optional_text(json_text(node.embedding.as_ref())?),
        optional_text(node.embedding_model.clone()),
        node.embedding_dimensions
            .map(|value| Value::Integer(value as i64))
            .unwrap_or(Value::Null),
        node.embedded_at
            .map(|value| Value::Integer(value.timestamp_millis()))
            .unwrap_or(Value::Null),
        Value::Real(node.psi as f64),
        Value::Real(node.rho as f64),
        Value::Real(node.kappa as f64),
        Value::Real(node.user_avec.stability as f64),
        Value::Real(node.user_avec.friction as f64),
        Value::Real(node.user_avec.logic as f64),
        Value::Real(node.user_avec.autonomy as f64),
        Value::Real(node.user_avec.psi() as f64),
        Value::Real(node.model_avec.stability as f64),
        Value::Real(node.model_avec.friction as f64),
        Value::Real(node.model_avec.logic as f64),
        Value::Real(node.model_avec.autonomy as f64),
        Value::Real(node.model_avec.psi() as f64),
        Value::Real(compression.stability as f64),
        Value::Real(compression.friction as f64),
        Value::Real(compression.logic as f64),
        Value::Real(compression.autonomy as f64),
        Value::Real(compression.psi() as f64),
    ])
}

fn delete_nodes(conn: &Connection, request: NodeDeleteRequest) -> Result<NodeDeleteResult> {
    let tenant_id = normalize_tenant_id(&request.tenant_id);
    let session_id = request.session_id.trim();
    if session_id.is_empty() {
        return Err(anyhow!("session id cannot be empty"));
    }
    if request.sync_keys.is_empty() && request.node_ids.is_empty() {
        return Err(anyhow!("at least one sync key or node id is required"));
    }

    let mut records = Vec::new();
    let mut target_ids = BTreeSet::new();

    for sync_key in &request.sync_keys {
        let sync_key = sync_key.trim();
        if sync_key.is_empty() {
            continue;
        }
        let found = conn
            .query_row(
                "SELECT id, sync_key FROM temporal_node \
                 WHERE tenant_id = ?1 AND session_id = ?2 AND sync_key = ?3 LIMIT 1",
                (&tenant_id, session_id, sync_key),
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        match found {
            Some((node_id, _)) => {
                target_ids.insert(node_id);
            }
            None => records.push(NodeDeleteRecord {
                node_id: String::new(),
                sync_key: sync_key.to_string(),
                status: NodeDeleteStatus::NotFound,
                reason: Some("sync key not found in session scope".to_string()),
            }),
        }
    }

    for node_id in &request.node_ids {
        let Some(normalized) = normalize_temporal_node_id(node_id) else {
            records.push(NodeDeleteRecord {
                node_id: node_id.clone(),
                sync_key: String::new(),
                status: NodeDeleteStatus::NotFound,
                reason: Some("invalid node id".to_string()),
            });
            continue;
        };
        let found = conn
            .query_row(
                "SELECT id, sync_key, session_id FROM temporal_node WHERE id = ?1",
                [&normalized],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        match found {
            Some((id, _sync_key, row_session)) if row_session == session_id => {
                target_ids.insert(id);
            }
            Some((_, sync_key, _)) => records.push(NodeDeleteRecord {
                node_id: normalized,
                sync_key,
                status: NodeDeleteStatus::NotFound,
                reason: Some("node id not found in session scope".to_string()),
            }),
            None => records.push(NodeDeleteRecord {
                node_id: normalized,
                sync_key: String::new(),
                status: NodeDeleteStatus::NotFound,
                reason: Some("node id not found".to_string()),
            }),
        }
    }

    for node_id in &target_ids {
        if let Some(sync_key) = conn
            .query_row(
                "SELECT sync_key FROM temporal_node WHERE id = ?1",
                [node_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            let status = if request.dry_run {
                NodeDeleteStatus::Skipped
            } else {
                NodeDeleteStatus::Deleted
            };
            records.push(NodeDeleteRecord {
                node_id: node_id.clone(),
                sync_key,
                status,
                reason: request.dry_run.then(|| "would delete".to_string()),
            });
        }
    }

    if !request.dry_run {
        let tx = conn.unchecked_transaction()?;
        for node_id in &target_ids {
            tx.execute("DELETE FROM temporal_node WHERE id = ?1", [node_id])?;
        }
        tx.commit()?;
    }

    Ok(summarize_delete(request.dry_run, 0, 0, records))
}

fn purge_session(conn: &Connection, request: SessionPurgeRequest) -> Result<NodeDeleteResult> {
    let tenant_id = normalize_tenant_id(&request.tenant_id);
    let session_id = request.session_id.trim();
    if session_id.is_empty() {
        return Err(anyhow!("session id cannot be empty"));
    }

    let mut filter = NodeFilter::default();
    filter.exact_scope(&tenant_id, session_id);
    filter.tiers(request.tiers.as_deref());
    let (where_sql, params) = filter.finish();
    let sql = format!("SELECT id, sync_key FROM temporal_node {where_sql} ORDER BY sync_key ASC");
    let mut stmt = conn.prepare(&sql)?;
    let targets = stmt
        .query_map(params_from_iter(params.iter()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let records = targets
        .iter()
        .map(|(node_id, sync_key)| NodeDeleteRecord {
            node_id: node_id.clone(),
            sync_key: sync_key.clone(),
            status: if request.dry_run {
                NodeDeleteStatus::Skipped
            } else {
                NodeDeleteStatus::Deleted
            },
            reason: request.dry_run.then(|| "would delete".to_string()),
        })
        .collect::<Vec<_>>();

    let calibrations_deleted = if request.include_calibration {
        count_scope(conn, "calibration", &tenant_id, session_id)?
    } else {
        0
    };
    let checkpoints_deleted = if request.include_checkpoints {
        count_scope(conn, "sync_checkpoint", &tenant_id, session_id)?
    } else {
        0
    };

    if !request.dry_run {
        let tx = conn.unchecked_transaction()?;
        let delete_sql = format!("DELETE FROM temporal_node {where_sql}");
        tx.execute(&delete_sql, params_from_iter(params.iter()))?;
        tx.execute(
            "DELETE FROM semantic_tag_index WHERE tenant_id = ?1 AND session_id = ?2",
            (&tenant_id, session_id),
        )?;
        if request.include_calibration {
            tx.execute(
                "DELETE FROM calibration WHERE tenant_id = ?1 AND session_id = ?2",
                (&tenant_id, session_id),
            )?;
        }
        if request.include_checkpoints {
            tx.execute(
                "DELETE FROM sync_checkpoint WHERE tenant_id = ?1 AND session_id = ?2",
                (&tenant_id, session_id),
            )?;
        }
        tx.commit()?;
    }

    Ok(summarize_delete(
        request.dry_run,
        calibrations_deleted,
        checkpoints_deleted,
        records,
    ))
}

fn rekey(
    conn: &Connection,
    node_ids: Vec<String>,
    target_tenant_id: &str,
    target_session_id: &str,
    dry_run: bool,
    allow_merge: bool,
) -> Result<BatchRekeyResult> {
    if node_ids.is_empty() {
        return Err(anyhow!("at least one node id is required"));
    }
    if target_session_id.trim().is_empty() {
        return Err(anyhow!("target session id cannot be empty"));
    }

    let normalized_node_ids = node_ids
        .iter()
        .filter_map(|node_id| normalize_temporal_node_id(node_id))
        .collect::<Vec<_>>();
    if normalized_node_ids.is_empty() {
        return Err(anyhow!("no valid node ids were provided"));
    }

    let mut missing_node_ids = Vec::new();
    let mut scopes = Vec::new();
    for node_id in &normalized_node_ids {
        let found = conn
            .query_row(
                "SELECT tenant_id, session_id FROM temporal_node WHERE id = ?1",
                [node_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let Some((tenant_id, session_id)) = found else {
            missing_node_ids.push(node_id.clone());
            continue;
        };
        let tenant_id = if tenant_id.trim().is_empty() {
            derive_tenant_id_from_session(&session_id)
        } else {
            tenant_id
        };
        if !scopes.iter().any(|(tenant, session): &(String, String)| {
            tenant == &tenant_id && session == &session_id
        }) {
            scopes.push((tenant_id, session_id));
        }
    }

    let tx = conn.unchecked_transaction()?;
    let mut scope_results = Vec::new();
    let mut temporal_nodes_updated = 0usize;
    let mut calibrations_updated = 0usize;

    for (source_tenant_id, source_session_id) in scopes {
        let same_scope =
            source_tenant_id == target_tenant_id && source_session_id == target_session_id;
        let include_legacy = source_tenant_id == DEFAULT_TENANT;
        let temporal_nodes = count_rekey_scope(
            &tx,
            "temporal_node",
            &source_tenant_id,
            &source_session_id,
            include_legacy,
        )?;
        let calibrations = count_rekey_scope(
            &tx,
            "calibration",
            &source_tenant_id,
            &source_session_id,
            include_legacy,
        )?;
        let (target_temporal_nodes, target_calibrations) = if same_scope {
            (0, 0)
        } else {
            (
                count_rekey_scope(
                    &tx,
                    "temporal_node",
                    target_tenant_id,
                    target_session_id,
                    target_tenant_id == DEFAULT_TENANT,
                )?,
                count_rekey_scope(
                    &tx,
                    "calibration",
                    target_tenant_id,
                    target_session_id,
                    target_tenant_id == DEFAULT_TENANT,
                )?,
            )
        };
        let conflict =
            !allow_merge && !same_scope && (target_temporal_nodes > 0 || target_calibrations > 0);
        let mut applied = false;
        let message = if same_scope {
            Some("source and target scopes are identical".to_string())
        } else if conflict {
            Some("target scope already contains rows; set allow_merge=true to override".to_string())
        } else if dry_run {
            None
        } else {
            apply_rekey_scope(
                &tx,
                "temporal_node",
                &source_tenant_id,
                &source_session_id,
                target_tenant_id,
                target_session_id,
                include_legacy,
            )?;
            apply_rekey_scope(
                &tx,
                "calibration",
                &source_tenant_id,
                &source_session_id,
                target_tenant_id,
                target_session_id,
                include_legacy,
            )?;
            applied = true;
            temporal_nodes_updated += temporal_nodes;
            calibrations_updated += calibrations;
            None
        };

        scope_results.push(ScopeRekeyResult {
            source_tenant_id,
            source_session_id,
            target_tenant_id: target_tenant_id.to_string(),
            target_session_id: target_session_id.to_string(),
            temporal_nodes,
            calibrations,
            target_temporal_nodes,
            target_calibrations,
            applied,
            conflict,
            message,
        });
    }

    tx.commit()?;
    Ok(BatchRekeyResult {
        dry_run,
        requested_node_ids: normalized_node_ids.len(),
        resolved_node_ids: normalized_node_ids
            .len()
            .saturating_sub(missing_node_ids.len()),
        missing_node_ids,
        scopes: scope_results,
        temporal_nodes_updated,
        calibrations_updated,
    })
}

fn apply_rekey_scope(
    conn: &Connection,
    table: &str,
    source_tenant_id: &str,
    source_session_id: &str,
    target_tenant_id: &str,
    target_session_id: &str,
    include_legacy: bool,
) -> Result<()> {
    let sql = format!(
        "UPDATE {table} SET tenant_id = ?1, session_id = ?2 \
         WHERE session_id = ?3 AND (tenant_id = ?4 OR (?5 = 1 AND (tenant_id IS NULL OR tenant_id = '')))"
    );
    conn.execute(
        &sql,
        (
            target_tenant_id,
            target_session_id,
            source_session_id,
            source_tenant_id,
            include_legacy as i64,
        ),
    )?;
    Ok(())
}

fn count_rekey_scope(
    conn: &Connection,
    table: &str,
    tenant_id: &str,
    session_id: &str,
    include_legacy: bool,
) -> Result<usize> {
    let sql = format!(
        "SELECT COUNT(*) FROM {table} \
         WHERE session_id = ?1 AND (tenant_id = ?2 OR (?3 = 1 AND (tenant_id IS NULL OR tenant_id = '')))"
    );
    let count: i64 = conn.query_row(
        &sql,
        (session_id, tenant_id, include_legacy as i64),
        |row| row.get(0),
    )?;
    Ok(count as usize)
}

fn count_scope(conn: &Connection, table: &str, tenant_id: &str, session_id: &str) -> Result<usize> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?1 AND session_id = ?2");
    let count: i64 = conn.query_row(&sql, (tenant_id, session_id), |row| row.get(0))?;
    Ok(count as usize)
}

fn summarize_delete(
    dry_run: bool,
    calibrations_deleted: usize,
    checkpoints_deleted: usize,
    records: Vec<NodeDeleteRecord>,
) -> NodeDeleteResult {
    let deleted = records
        .iter()
        .filter(|record| record.status == NodeDeleteStatus::Deleted)
        .count();
    let skipped = records
        .iter()
        .filter(|record| record.status == NodeDeleteStatus::Skipped)
        .count();
    let not_found = records
        .iter()
        .filter(|record| record.status == NodeDeleteStatus::NotFound)
        .count();
    NodeDeleteResult {
        dry_run,
        deleted: if dry_run { skipped } else { deleted },
        blocked: 0,
        not_found,
        skipped: if dry_run { 0 } else { skipped },
        calibrations_deleted,
        checkpoints_deleted,
        records,
    }
}

#[derive(Default)]
struct NodeFilter {
    clauses: Vec<String>,
    params: Vec<Value>,
}

impl NodeFilter {
    fn session(&mut self, session_id: &str) {
        let index = self.params.len() + 1;
        self.clauses.push(format!(
            "session_id = ?{index} AND (tenant_id = ?{} OR tenant_id IS NULL OR tenant_id = '')",
            index + 1
        ));
        self.params.push(Value::Text(session_id.to_string()));
        self.params
            .push(Value::Text(derive_tenant_id_from_session(session_id)));
    }

    fn exact_scope(&mut self, tenant_id: &str, session_id: &str) {
        let index = self.params.len() + 1;
        self.clauses.push(format!(
            "tenant_id = ?{index} AND session_id = ?{}",
            index + 1
        ));
        self.params.push(Value::Text(tenant_id.to_string()));
        self.params.push(Value::Text(session_id.to_string()));
    }

    fn time_range(&mut self, from_utc: Option<DateTime<Utc>>, to_utc: Option<DateTime<Utc>>) {
        if let Some(from_utc) = from_utc {
            let index = self.params.len() + 1;
            self.clauses.push(format!("timestamp_ms >= ?{index}"));
            self.params
                .push(Value::Integer(from_utc.timestamp_millis()));
        }
        if let Some(to_utc) = to_utc {
            let index = self.params.len() + 1;
            self.clauses.push(format!("timestamp_ms <= ?{index}"));
            self.params.push(Value::Integer(to_utc.timestamp_millis()));
        }
    }

    fn tiers(&mut self, tiers: Option<&[String]>) {
        let Some(tiers) = normalized_tiers(tiers) else {
            return;
        };
        let start = self.params.len() + 1;
        let placeholders = (0..tiers.len())
            .map(|offset| format!("?{}", start + offset))
            .collect::<Vec<_>>()
            .join(", ");
        self.clauses
            .push(format!("lower(tier) IN ({placeholders})"));
        for tier in tiers {
            self.params.push(Value::Text(tier));
        }
    }

    fn finish(self) -> (String, Vec<Value>) {
        if self.clauses.is_empty() {
            (String::new(), self.params)
        } else {
            (format!("WHERE {}", self.clauses.join(" AND ")), self.params)
        }
    }
}

fn load_nodes(conn: &Connection, sql: &str, params: &[Value]) -> Result<Vec<SttpNode>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), map_node)?;
    rows.collect::<Result<Vec<_>, _>>()
        .context("failed to read temporal nodes")
}

fn map_node(row: &Row<'_>) -> rusqlite::Result<SttpNode> {
    let timestamp = from_millis(row.get("timestamp_ms")?);
    let updated_at = from_millis(row.get("updated_at_ms")?);
    let compression = AvecState {
        stability: real(row, "comp_stability")?,
        friction: real(row, "comp_friction")?,
        logic: real(row, "comp_logic")?,
        autonomy: real(row, "comp_autonomy")?,
    };
    let mut node = SttpNode {
        raw: row.get("raw")?,
        session_id: row.get("session_id")?,
        tier: row.get("tier")?,
        timestamp,
        compression_depth: row.get("compression_depth")?,
        parent_node_id: row.get("parent_node_id")?,
        sync_key: row.get("sync_key")?,
        updated_at,
        source_metadata: optional_json(row.get("source_metadata")?)?,
        context_summary: row.get("context_summary")?,
        semantic_tags: optional_json(row.get("semantic_tags")?)?,
        semantic_links: optional_json(row.get("semantic_links")?)?,
        embedding: optional_json(row.get("embedding")?)?,
        embedding_model: row.get("embedding_model")?,
        embedding_dimensions: row
            .get::<_, Option<i64>>("embedding_dimensions")?
            .map(|value| value as usize),
        embedded_at: row
            .get::<_, Option<i64>>("embedded_at_ms")?
            .map(from_millis),
        psi: real(row, "psi")?,
        rho: real(row, "rho")?,
        kappa: real(row, "kappa")?,
        user_avec: AvecState {
            stability: real(row, "user_stability")?,
            friction: real(row, "user_friction")?,
            logic: real(row, "user_logic")?,
            autonomy: real(row, "user_autonomy")?,
        },
        model_avec: AvecState {
            stability: real(row, "model_stability")?,
            friction: real(row, "model_friction")?,
            logic: real(row, "model_logic")?,
            autonomy: real(row, "model_autonomy")?,
        },
        compression_avec: Some(compression),
    };
    if node.sync_key.trim().is_empty() {
        node.sync_key = node.canonical_sync_key();
    }
    Ok(node)
}

fn map_checkpoint(row: &Row<'_>) -> rusqlite::Result<SyncCheckpoint> {
    let cursor_updated_at: Option<i64> = row.get("cursor_updated_at_ms")?;
    let cursor_sync_key: Option<String> = row.get("cursor_sync_key")?;
    let cursor = match (cursor_updated_at, cursor_sync_key) {
        (Some(updated_at), Some(sync_key)) => Some(SyncCursor {
            updated_at: from_millis(updated_at),
            sync_key,
        }),
        _ => None,
    };
    Ok(SyncCheckpoint {
        session_id: row.get("session_id")?,
        connector_id: row.get("connector_id")?,
        cursor,
        updated_at: from_millis(row.get("updated_at_ms")?),
        metadata: optional_json(row.get("metadata")?)?,
    })
}

fn nodes_equivalent(existing: &SttpNode, candidate: &SttpNode, compression: AvecState) -> bool {
    existing.raw == candidate.raw
        && existing.tier == candidate.tier
        && existing.timestamp == candidate.timestamp
        && existing.compression_depth == candidate.compression_depth
        && existing.parent_node_id == candidate.parent_node_id
        && existing.context_summary == candidate.context_summary
        && existing.semantic_tags == candidate.semantic_tags
        && existing.semantic_links == candidate.semantic_links
        && vectors_equivalent(
            existing.embedding.as_deref(),
            candidate.embedding.as_deref(),
        )
        && existing.embedding_model == candidate.embedding_model
        && existing.embedding_dimensions == candidate.embedding_dimensions
        && existing.embedded_at == candidate.embedded_at
        && avec_equivalent(&existing.user_avec, &candidate.user_avec)
        && avec_equivalent(&existing.model_avec, &candidate.model_avec)
        && existing
            .compression_avec
            .is_some_and(|stored| avec_equivalent(&stored, &compression))
        && floats_equivalent(existing.rho, candidate.rho)
        && floats_equivalent(existing.kappa, candidate.kappa)
        && floats_equivalent(existing.psi, candidate.psi)
        && json_text(existing.source_metadata.as_ref()).ok()
            == json_text(candidate.source_metadata.as_ref()).ok()
}

fn resolved_compression(node: &SttpNode) -> AvecState {
    match node.compression_avec {
        Some(avec) if avec.psi() != 0.0 => avec,
        _ => node.model_avec,
    }
}

fn avec_equivalent(left: &AvecState, right: &AvecState) -> bool {
    floats_equivalent(left.stability, right.stability)
        && floats_equivalent(left.friction, right.friction)
        && floats_equivalent(left.logic, right.logic)
        && floats_equivalent(left.autonomy, right.autonomy)
}

fn vectors_equivalent(left: Option<&[f32]>, right: Option<&[f32]>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| floats_equivalent(*left, *right))
        }
        _ => false,
    }
}

fn floats_equivalent(left: f32, right: f32) -> bool {
    (left - right).abs() <= f32::EPSILON
}

fn normalized_tiers(tiers: Option<&[String]>) -> Option<Vec<String>> {
    let tiers = tiers
        .unwrap_or(&[])
        .iter()
        .map(|tier| tier.trim().to_ascii_lowercase())
        .filter(|tier| !tier.is_empty())
        .collect::<Vec<_>>();
    if tiers.is_empty() { None } else { Some(tiers) }
}

fn normalize_tenant_id(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        DEFAULT_TENANT.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_temporal_node_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((table, id)) = trimmed.split_once(':') {
        if table != "temporal_node" {
            return None;
        }
        let id = id.trim();
        if id.is_empty() {
            None
        } else {
            Some(id.to_string())
        }
    } else {
        Some(trimmed.to_string())
    }
}

pub(crate) fn from_millis(value: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(value).unwrap_or_else(Utc::now)
}

fn real(row: &Row<'_>, column: &str) -> rusqlite::Result<f32> {
    Ok(row.get::<_, f64>(column)? as f32)
}

fn optional_text(value: Option<String>) -> Value {
    match value {
        Some(value) => Value::Text(value),
        None => Value::Null,
    }
}

pub(crate) fn json_text<T: serde::Serialize>(value: Option<&T>) -> Result<Option<String>> {
    value
        .map(serde_json::to_string)
        .transpose()
        .context("failed to encode sqlite json column")
}

pub(crate) fn optional_json<T: DeserializeOwned>(
    value: Option<String>,
) -> rusqlite::Result<Option<T>> {
    match value {
        None => Ok(None),
        Some(text) if text.trim().is_empty() => Ok(None),
        Some(text) => serde_json::from_str(&text).map(Some).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        }),
    }
}

fn is_unique_conflict(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<rusqlite::Error>()
        .is_some_and(|error| {
            matches!(
                error,
                rusqlite::Error::SqliteFailure(code, _)
                    if code.code == rusqlite::ErrorCode::ConstraintViolation
            )
        })
}
