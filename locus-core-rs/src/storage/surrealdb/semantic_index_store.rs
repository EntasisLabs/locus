use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::domain::contracts::{
    SemanticIndexStore, SemanticIndexStoreInitializer, TagEmbedding,
};
use crate::domain::models::{
    SemanticTagNodeRef, SemanticTagQueryFilter, SemanticTagRecord,
};
use crate::storage::surrealdb::client::{QueryParams, SurrealDbClient};
use crate::storage::surrealdb::models::{
    SurrealSemanticTagRecord, SurrealSyncKeyRecord, SurrealTagVocabularyRecord,
};
use crate::storage::surrealdb::raw_queries;

pub struct SurrealDbSemanticIndexStore {
    client: Arc<dyn SurrealDbClient>,
}

impl SurrealDbSemanticIndexStore {
    pub fn new(client: Arc<dyn SurrealDbClient>) -> Self {
        Self { client }
    }

    fn canonical_tag(tag: &str) -> String {
        tag.trim().to_lowercase()
    }

    fn map_record(record: SurrealSemanticTagRecord) -> SemanticTagRecord {
        SemanticTagRecord {
            tenant_id: record.tenant_id,
            session_id: record.session_id,
            node_id: record.node_id,
            sync_key: record.sync_key,
            tag: record.tag,
            embedding: record.embedding,
            embedding_model: record.embedding_model,
            embedding_dimensions: record.embedding_dimensions,
            embedded_at: parse_optional_timestamp(record.embedded_at.as_deref()),
            updated_at: parse_optional_timestamp(record.updated_at.as_deref())
                .unwrap_or_else(Utc::now),
        }
    }
}

#[async_trait]
impl SemanticIndexStoreInitializer for SurrealDbSemanticIndexStore {
    async fn initialize_async(&self) -> Result<()> {
        self.client
            .raw_query(raw_queries::INIT_SCHEMA_QUERY, QueryParams::new())
            .await?;
        Ok(())
    }
}

#[async_trait]
impl SemanticIndexStore for SurrealDbSemanticIndexStore {
    async fn sync_node_tags_async(
        &self,
        node_ref: SemanticTagNodeRef,
        tags: &[String],
        embeddings: Option<&HashMap<String, TagEmbedding>>,
    ) -> Result<()> {
        let now = Utc::now();
        let canonical_tags: HashSet<String> = tags
            .iter()
            .map(|tag| Self::canonical_tag(tag))
            .filter(|tag| !tag.is_empty())
            .collect();

        let mut list_params = QueryParams::new();
        list_params.insert("tenant_id".to_string(), json!(&node_ref.tenant_id));
        list_params.insert("sync_key".to_string(), json!(&node_ref.sync_key));

        let existing_rows = self
            .client
            .raw_query(raw_queries::LIST_TAGS_FOR_SYNC_KEY_QUERY, list_params)
            .await?;
        let existing_tags: HashSet<String> = decode_rows::<SurrealTagVocabularyRecord>(existing_rows)?
            .into_iter()
            .map(|record| record.tag)
            .collect();

        for existing_tag in existing_tags {
            if canonical_tags.contains(&existing_tag) {
                continue;
            }

            let mut delete_params = QueryParams::new();
            delete_params.insert("tenant_id".to_string(), json!(&node_ref.tenant_id));
            delete_params.insert("sync_key".to_string(), json!(&node_ref.sync_key));
            delete_params.insert("tag".to_string(), json!(&existing_tag));
            self.client
                .raw_query(
                    r#"
                    DELETE semantic_tag_index
                    WHERE tenant_id = $tenant_id AND sync_key = $sync_key AND tag = $tag;
                    "#,
                    delete_params,
                )
                .await?;
        }

        for tag in canonical_tags {
            let embedding_payload = embeddings.and_then(|map| {
                map.get(&tag)
                    .or_else(|| map.get(&tag.to_lowercase()))
                    .or_else(|| {
                        map.iter()
                            .find(|(candidate, _)| {
                                candidate.trim().eq_ignore_ascii_case(tag.as_str())
                            })
                            .map(|(_, value)| value)
                    })
            });

            let mut params = QueryParams::new();
            params.insert("tenant_id".to_string(), json!(&node_ref.tenant_id));
            params.insert("session_id".to_string(), json!(&node_ref.session_id));
            params.insert("node_id".to_string(), json!(&node_ref.node_id));
            params.insert("sync_key".to_string(), json!(&node_ref.sync_key));
            params.insert("tag".to_string(), json!(&tag));
            params.insert("updated_at".to_string(), json!(now.to_rfc3339()));

            if let Some(payload) = embedding_payload {
                params.insert("embedding".to_string(), json!(payload.vector));
                params.insert("embedding_model".to_string(), json!(payload.model));
                params.insert(
                    "embedding_dimensions".to_string(),
                    json!(payload.vector.len()),
                );
                params.insert("embedded_at".to_string(), json!(now.to_rfc3339()));
                self.client
                    .raw_query(raw_queries::UPSERT_TAG_ROW_QUERY, params)
                    .await?;
            } else {
                self.client
                    .raw_query(raw_queries::UPSERT_TAG_ROW_META_QUERY, params)
                    .await?;
            }
        }

        Ok(())
    }

    async fn delete_node_tags_async(&self, tenant_id: &str, sync_key: &str) -> Result<()> {
        let mut params = QueryParams::new();
        params.insert("tenant_id".to_string(), json!(tenant_id));
        params.insert("sync_key".to_string(), json!(sync_key));
        self.client
            .raw_query(raw_queries::DELETE_TAG_ROWS_FOR_SYNC_KEY_QUERY, params)
            .await?;
        Ok(())
    }

    async fn find_sync_keys_by_tags_async(
        &self,
        tenant_id: &str,
        tags: &[String],
        match_all: bool,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>> {
        let canonical_tags: Vec<String> = tags
            .iter()
            .map(|tag| Self::canonical_tag(tag))
            .filter(|tag| !tag.is_empty())
            .collect();

        if canonical_tags.is_empty() {
            return Ok(Vec::new());
        }

        let capped_limit = limit.max(1);
        let mut params = QueryParams::new();
        params.insert("tenant_id".to_string(), json!(tenant_id));
        params.insert("tags".to_string(), json!(canonical_tags));
        params.insert(
            "session_id".to_string(),
            session_id
                .map(|value| json!(value))
                .unwrap_or(Value::Null),
        );
        params.insert("limit".to_string(), json!(capped_limit));

        let query = raw_queries::find_sync_keys_by_tags_query(match_all);
        let rows = self.client.raw_query(&query, params).await?;
        let records = decode_rows::<SurrealSyncKeyRecord>(rows)?;
        Ok(records.into_iter().map(|record| record.sync_key).collect())
    }

    async fn find_tags_async(
        &self,
        tenant_id: &str,
        prefix: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>> {
        let capped_limit = limit.max(1);
        let mut clauses = vec!["tenant_id = $tenant_id".to_string()];
        if prefix.is_some() {
            clauses.push("string::starts_with(tag, $prefix)".to_string());
        }
        let where_clause = clauses.join(" AND ");

        let mut params = QueryParams::new();
        params.insert("tenant_id".to_string(), json!(tenant_id));
        if let Some(prefix) = prefix {
            params.insert("prefix".to_string(), json!(prefix.to_lowercase()));
        }

        let query = raw_queries::find_tags_vocabulary_query(&where_clause, capped_limit);
        let rows = self.client.raw_query(&query, params).await?;
        let records = decode_rows::<SurrealTagVocabularyRecord>(rows)?;
        Ok(records.into_iter().map(|record| record.tag).collect())
    }

    async fn query_tag_records_async(
        &self,
        filter: SemanticTagQueryFilter,
    ) -> Result<Vec<SemanticTagRecord>> {
        let capped_limit = filter.limit.max(1);
        let mut clauses = Vec::new();
        let mut params = QueryParams::new();

        if let Some(tenant_id) = &filter.tenant_id {
            clauses.push("tenant_id = $tenant_id".to_string());
            params.insert("tenant_id".to_string(), json!(tenant_id));
        }
        if let Some(session_id) = &filter.session_id {
            clauses.push("session_id = $session_id".to_string());
            params.insert("session_id".to_string(), json!(session_id));
        }
        if let Some(tags) = &filter.tags {
            let canonical: Vec<String> = tags.iter().map(|tag| Self::canonical_tag(tag)).collect();
            clauses.push("tag IN $tags".to_string());
            params.insert("tags".to_string(), json!(canonical));
        }
        if let Some(prefix) = &filter.tag_prefix {
            clauses.push("string::starts_with(tag, $tag_prefix)".to_string());
            params.insert("tag_prefix".to_string(), json!(prefix.to_lowercase()));
        }
        if let Some(has_embedding) = filter.has_embedding {
            clauses.push(if has_embedding {
                "embedding IS NOT NONE".to_string()
            } else {
                "embedding IS NONE".to_string()
            });
        }
        if filter.missing_embedding_only {
            clauses.push("embedding IS NONE".to_string());
        }

        let where_clause = if clauses.is_empty() {
            "true".to_string()
        } else {
            clauses.join(" AND ")
        };

        let query = raw_queries::query_tag_records_query(&where_clause, capped_limit);
        let rows = self.client.raw_query(&query, params).await?;
        Ok(decode_rows::<SurrealSemanticTagRecord>(rows)?
            .into_iter()
            .map(Self::map_record)
            .collect())
    }
}

fn parse_optional_timestamp(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
}

fn decode_rows<T>(rows: Vec<Value>) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    rows.into_iter()
        .map(serde_json::from_value)
        .collect::<std::result::Result<Vec<T>, _>>()
        .map_err(Into::into)
}
