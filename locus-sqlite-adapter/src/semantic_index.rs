use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use locus_core_rs::domain::contracts::{
    SemanticIndexStore, SemanticIndexStoreInitializer, TagEmbedding,
};
use locus_core_rs::domain::models::{
    SemanticTagNodeRef, SemanticTagQueryFilter, SemanticTagRecord,
};
use rusqlite::params_from_iter;
use rusqlite::types::Value;

use crate::db::SqliteDatabase;
use crate::node_store::{from_millis, json_text, optional_json};

/// SQLite-backed per-tag vocabulary and optional tag embeddings.
pub struct SqliteSemanticIndexStore {
    db: Arc<SqliteDatabase>,
}

impl SqliteSemanticIndexStore {
    pub fn new(db: Arc<SqliteDatabase>) -> Self {
        Self { db }
    }

    fn canonical_tag(tag: &str) -> String {
        tag.trim().to_lowercase()
    }
}

#[async_trait]
impl SemanticIndexStoreInitializer for SqliteSemanticIndexStore {
    async fn initialize_async(&self) -> Result<()> {
        self.db.initialize()
    }
}

#[async_trait]
impl SemanticIndexStore for SqliteSemanticIndexStore {
    async fn sync_node_tags_async(
        &self,
        node_ref: SemanticTagNodeRef,
        tags: &[String],
        embeddings: Option<&HashMap<String, TagEmbedding>>,
    ) -> Result<()> {
        let tags = tags.to_vec();
        let embeddings = embeddings.cloned();
        self.db.with_connection(|conn| {
            let now = Utc::now().timestamp_millis();
            let canonical_tags: HashSet<String> = tags
                .iter()
                .map(|tag| SqliteSemanticIndexStore::canonical_tag(tag))
                .filter(|tag| !tag.is_empty())
                .collect();

            let tx = conn
                .unchecked_transaction()
                .context("failed to open tag sync transaction")?;
            let existing = {
                let mut stmt = tx.prepare(
                    "SELECT tag, embedding, embedding_model, embedding_dimensions, embedded_at_ms \
                     FROM semantic_tag_index WHERE tenant_id = ?1 AND sync_key = ?2",
                )?;
                let rows = stmt.query_map((&node_ref.tenant_id, &node_ref.sync_key), |row| {
                    Ok(ExistingTag {
                        tag: row.get(0)?,
                        embedding: row.get(1)?,
                        embedding_model: row.get(2)?,
                        embedding_dimensions: row.get(3)?,
                        embedded_at_ms: row.get(4)?,
                    })
                })?;
                rows.collect::<Result<Vec<_>, _>>()?
            };

            for prior in &existing {
                if !canonical_tags.contains(&prior.tag) {
                    tx.execute(
                        "DELETE FROM semantic_tag_index \
                         WHERE tenant_id = ?1 AND sync_key = ?2 AND tag = ?3",
                        (&node_ref.tenant_id, &node_ref.sync_key, &prior.tag),
                    )?;
                }
            }

            for tag in &canonical_tags {
                let prior = existing.iter().find(|row| &row.tag == tag);
                let embedding_payload = embeddings
                    .as_ref()
                    .and_then(|map| lookup_embedding(map, tag));
                let embedding = embedding_payload
                    .map(|value| json_text(Some(&value.vector)))
                    .transpose()?
                    .flatten()
                    .or_else(|| prior.and_then(|row| row.embedding.clone()));
                let embedding_model = embedding_payload
                    .map(|value| value.model.clone())
                    .or_else(|| prior.and_then(|row| row.embedding_model.clone()));
                let embedding_dimensions = embedding_payload
                    .map(|value| value.vector.len() as i64)
                    .or(prior.and_then(|row| row.embedding_dimensions));
                let embedded_at_ms = if embedding_payload.is_some() {
                    Some(now)
                } else {
                    prior.and_then(|row| row.embedded_at_ms)
                };

                tx.execute(
                    "INSERT INTO semantic_tag_index (
                        tenant_id, session_id, node_id, sync_key, tag, embedding, embedding_model,
                        embedding_dimensions, embedded_at_ms, updated_at_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                     ON CONFLICT(tenant_id, sync_key, tag) DO UPDATE SET
                        session_id = excluded.session_id,
                        node_id = excluded.node_id,
                        embedding = excluded.embedding,
                        embedding_model = excluded.embedding_model,
                        embedding_dimensions = excluded.embedding_dimensions,
                        embedded_at_ms = excluded.embedded_at_ms,
                        updated_at_ms = excluded.updated_at_ms",
                    (
                        &node_ref.tenant_id,
                        &node_ref.session_id,
                        &node_ref.node_id,
                        &node_ref.sync_key,
                        tag,
                        embedding,
                        embedding_model,
                        embedding_dimensions,
                        embedded_at_ms,
                        now,
                    ),
                )?;
            }

            tx.commit()?;
            Ok(())
        })
    }

    async fn delete_node_tags_async(&self, tenant_id: &str, sync_key: &str) -> Result<()> {
        let tenant_id = tenant_id.to_string();
        let sync_key = sync_key.to_string();
        self.db.with_connection(|conn| {
            conn.execute(
                "DELETE FROM semantic_tag_index WHERE tenant_id = ?1 AND sync_key = ?2",
                (&tenant_id, &sync_key),
            )?;
            Ok(())
        })
    }

    async fn find_sync_keys_by_tags_async(
        &self,
        tenant_id: &str,
        tags: &[String],
        match_all: bool,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>> {
        let tenant_id = tenant_id.to_string();
        let tags = tags.to_vec();
        let session_id = session_id.map(str::to_string);
        self.db.with_connection(|conn| {
            let canonical_tags: Vec<String> = tags
                .iter()
                .map(|tag| SqliteSemanticIndexStore::canonical_tag(tag))
                .filter(|tag| !tag.is_empty())
                .collect();
            if canonical_tags.is_empty() {
                return Ok(Vec::new());
            }

            let mut params = vec![Value::Text(tenant_id)];
            let mut sql =
                String::from("SELECT sync_key, tag FROM semantic_tag_index WHERE tenant_id = ?1");
            if let Some(session_id) = session_id {
                params.push(Value::Text(session_id));
                sql.push_str(" AND session_id = ?2");
            }
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;

            let mut counts: HashMap<String, usize> = HashMap::new();
            for row in rows {
                let (sync_key, tag) = row?;
                if canonical_tags.iter().any(|candidate| candidate == &tag) {
                    *counts.entry(sync_key).or_default() += 1;
                }
            }

            let mut sync_keys = counts
                .into_iter()
                .filter(|(_, count)| {
                    if match_all {
                        *count >= canonical_tags.len()
                    } else {
                        *count > 0
                    }
                })
                .map(|(sync_key, _)| sync_key)
                .collect::<Vec<_>>();
            sync_keys.sort();
            sync_keys.truncate(limit);
            Ok(sync_keys)
        })
    }

    async fn find_tags_async(
        &self,
        tenant_id: &str,
        prefix: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>> {
        let tenant_id = tenant_id.to_string();
        let prefix = prefix.map(|value| value.to_lowercase());
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT tag FROM semantic_tag_index WHERE tenant_id = ?1 ORDER BY tag ASC",
            )?;
            let mut tags = stmt
                .query_map([&tenant_id], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            if let Some(prefix) = prefix {
                tags.retain(|tag| tag.starts_with(&prefix));
            }
            tags.truncate(limit);
            Ok(tags)
        })
    }

    async fn query_tag_records_async(
        &self,
        filter: SemanticTagQueryFilter,
    ) -> Result<Vec<SemanticTagRecord>> {
        self.db.with_connection(|conn| {
            let mut clauses = Vec::new();
            let mut params = Vec::new();
            if let Some(tenant_id) = filter.tenant_id.as_ref() {
                params.push(Value::Text(tenant_id.clone()));
                clauses.push(format!("tenant_id = ?{}", params.len()));
            }
            if let Some(session_id) = filter.session_id.as_ref() {
                params.push(Value::Text(session_id.clone()));
                clauses.push(format!("session_id = ?{}", params.len()));
            }
            if let Some(tags) = filter.tags.as_ref() {
                let canonical = tags
                    .iter()
                    .map(|tag| SqliteSemanticIndexStore::canonical_tag(tag))
                    .filter(|tag| !tag.is_empty())
                    .collect::<Vec<_>>();
                if canonical.is_empty() {
                    return Ok(Vec::new());
                }
                let start = params.len() + 1;
                let placeholders = (0..canonical.len())
                    .map(|offset| format!("?{}", start + offset))
                    .collect::<Vec<_>>()
                    .join(", ");
                clauses.push(format!("tag IN ({placeholders})"));
                for tag in canonical {
                    params.push(Value::Text(tag));
                }
            }
            if let Some(prefix) = filter.tag_prefix.as_ref() {
                params.push(Value::Text(like_prefix(&prefix.to_lowercase())));
                clauses.push(format!("tag LIKE ?{} ESCAPE '\\'", params.len()));
            }
            match filter.has_embedding {
                Some(true) => clauses.push("embedding IS NOT NULL".to_string()),
                Some(false) => clauses.push("embedding IS NULL".to_string()),
                None => {}
            }
            if filter.missing_embedding_only {
                clauses.push("embedding IS NULL".to_string());
            }
            params.push(Value::Integer(filter.limit.max(1) as i64));
            let where_sql = if clauses.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", clauses.join(" AND "))
            };
            let sql = format!(
                "SELECT tenant_id, session_id, node_id, sync_key, tag, embedding, embedding_model, \
                        embedding_dimensions, embedded_at_ms, updated_at_ms \
                 FROM semantic_tag_index {where_sql} \
                 ORDER BY updated_at_ms DESC LIMIT ?{}",
                params.len()
            );
            let mut stmt = conn.prepare(&sql)?;
            let records = stmt
                .query_map(params_from_iter(params.iter()), map_tag_record)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(records)
        })
    }
}

struct ExistingTag {
    tag: String,
    embedding: Option<String>,
    embedding_model: Option<String>,
    embedding_dimensions: Option<i64>,
    embedded_at_ms: Option<i64>,
}

fn lookup_embedding<'a>(
    embeddings: &'a HashMap<String, TagEmbedding>,
    tag: &str,
) -> Option<&'a TagEmbedding> {
    embeddings
        .get(tag)
        .or_else(|| embeddings.get(&tag.to_lowercase()))
        .or_else(|| {
            embeddings
                .iter()
                .find(|(candidate, _)| candidate.trim().eq_ignore_ascii_case(tag))
                .map(|(_, value)| value)
        })
}

fn map_tag_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<SemanticTagRecord> {
    let embedded_at: Option<i64> = row.get("embedded_at_ms")?;
    Ok(SemanticTagRecord {
        tenant_id: row.get("tenant_id")?,
        session_id: row.get("session_id")?,
        node_id: row.get("node_id")?,
        sync_key: row.get("sync_key")?,
        tag: row.get("tag")?,
        embedding: optional_json(row.get("embedding")?)?,
        embedding_model: row.get("embedding_model")?,
        embedding_dimensions: row
            .get::<_, Option<i64>>("embedding_dimensions")?
            .map(|value| value as usize),
        embedded_at: embedded_at.map(from_millis),
        updated_at: from_millis(row.get("updated_at_ms")?),
    })
}

fn like_prefix(prefix: &str) -> String {
    let mut escaped = String::new();
    for ch in prefix.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped.push('%');
    escaped
}
