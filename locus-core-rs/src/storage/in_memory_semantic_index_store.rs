use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;

use crate::domain::contracts::{
    SemanticIndexStore, SemanticIndexStoreInitializer, TagEmbedding,
};
use crate::domain::models::{
    SemanticTagNodeRef, SemanticTagQueryFilter, SemanticTagRecord,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TagKey {
    tenant_id: String,
    sync_key: String,
    tag: String,
}

#[derive(Default)]
pub struct InMemorySemanticIndexStore {
    rows: RwLock<HashMap<TagKey, SemanticTagRecord>>,
}

impl InMemorySemanticIndexStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn canonical_tag(tag: &str) -> String {
        tag.trim().to_lowercase()
    }
}

#[async_trait]
impl SemanticIndexStoreInitializer for InMemorySemanticIndexStore {
    async fn initialize_async(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl SemanticIndexStore for InMemorySemanticIndexStore {
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

        let mut rows = self.rows.write().expect("semantic index lock poisoned");
        let prefix = (node_ref.tenant_id.clone(), node_ref.sync_key.clone());

        let existing_keys: Vec<TagKey> = rows
            .keys()
            .filter(|key| key.tenant_id == prefix.0 && key.sync_key == prefix.1)
            .cloned()
            .collect();

        for key in existing_keys {
            if !canonical_tags.contains(&key.tag) {
                rows.remove(&key);
            }
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

            let key = TagKey {
                tenant_id: node_ref.tenant_id.clone(),
                sync_key: node_ref.sync_key.clone(),
                tag: tag.clone(),
            };

            let prior = rows.get(&key);
            let record = SemanticTagRecord {
                tenant_id: node_ref.tenant_id.clone(),
                session_id: node_ref.session_id.clone(),
                node_id: node_ref.node_id.clone(),
                sync_key: node_ref.sync_key.clone(),
                tag,
                embedding: embedding_payload
                    .map(|value| value.vector.clone())
                    .or_else(|| prior.and_then(|record| record.embedding.clone())),
                embedding_model: embedding_payload
                    .map(|value| value.model.clone())
                    .or_else(|| prior.and_then(|record| record.embedding_model.clone())),
                embedding_dimensions: embedding_payload
                    .map(|value| value.vector.len())
                    .or_else(|| prior.and_then(|record| record.embedding_dimensions)),
                embedded_at: embedding_payload
                    .map(|_| now)
                    .or_else(|| prior.and_then(|record| record.embedded_at)),
                updated_at: now,
            };

            rows.insert(key, record);
        }

        Ok(())
    }

    async fn delete_node_tags_async(&self, tenant_id: &str, sync_key: &str) -> Result<()> {
        let mut rows = self.rows.write().expect("semantic index lock poisoned");
        rows.retain(|key, _| key.tenant_id != tenant_id || key.sync_key != sync_key);
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

        let rows = self.rows.read().expect("semantic index lock poisoned");
        let mut counts: HashMap<String, usize> = HashMap::new();

        for record in rows.values() {
            if record.tenant_id != tenant_id {
                continue;
            }
            if let Some(session) = session_id
                && record.session_id != session
            {
                continue;
            }
            if !canonical_tags.contains(&record.tag) {
                continue;
            }
            *counts.entry(record.sync_key.clone()).or_default() += 1;
        }

        let mut sync_keys: Vec<String> = counts
            .into_iter()
            .filter(|(_, count)| {
                if match_all {
                    *count >= canonical_tags.len()
                } else {
                    *count > 0
                }
            })
            .map(|(sync_key, _)| sync_key)
            .collect();

        sync_keys.sort();
        sync_keys.truncate(limit);
        Ok(sync_keys)
    }

    async fn find_tags_async(
        &self,
        tenant_id: &str,
        prefix: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>> {
        let rows = self.rows.read().expect("semantic index lock poisoned");
        let prefix_lower = prefix.map(str::to_lowercase);
        let mut tags: Vec<String> = rows
            .values()
            .filter(|record| record.tenant_id == tenant_id)
            .filter(|record| {
                prefix_lower
                    .as_ref()
                    .map(|prefix| record.tag.starts_with(prefix))
                    .unwrap_or(true)
            })
            .map(|record| record.tag.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        tags.sort();
        tags.truncate(limit);
        Ok(tags)
    }

    async fn query_tag_records_async(
        &self,
        filter: SemanticTagQueryFilter,
    ) -> Result<Vec<SemanticTagRecord>> {
        let rows = self.rows.read().expect("semantic index lock poisoned");
        let mut records: Vec<SemanticTagRecord> = rows
            .values()
            .filter(|record| {
                filter
                    .tenant_id
                    .as_ref()
                    .map(|tenant| tenant == &record.tenant_id)
                    .unwrap_or(true)
            })
            .filter(|record| {
                filter
                    .session_id
                    .as_ref()
                    .map(|session| session == &record.session_id)
                    .unwrap_or(true)
            })
            .filter(|record| {
                filter
                    .tags
                    .as_ref()
                    .map(|tags| {
                        tags.iter()
                            .map(|tag| Self::canonical_tag(tag))
                            .any(|tag| tag == record.tag)
                    })
                    .unwrap_or(true)
            })
            .filter(|record| {
                filter
                    .tag_prefix
                    .as_ref()
                    .map(|prefix| record.tag.starts_with(&prefix.to_lowercase()))
                    .unwrap_or(true)
            })
            .filter(|record| match filter.has_embedding {
                Some(true) => record.embedding.is_some(),
                Some(false) => record.embedding.is_none(),
                None => true,
            })
            .filter(|record| !filter.missing_embedding_only || record.embedding.is_none())
            .cloned()
            .collect();

        records.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        records.truncate(filter.limit.max(1));
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sync_replaces_removed_tags() {
        let store = InMemorySemanticIndexStore::new();
        let node_ref = SemanticTagNodeRef {
            tenant_id: "default".to_string(),
            session_id: "demo".to_string(),
            node_id: "node-1".to_string(),
            sync_key: "sync-1".to_string(),
        };

        store
            .sync_node_tags_async(
                node_ref.clone(),
                &["alpha".to_string(), "beta".to_string()],
                None,
            )
            .await
            .expect("initial sync");

        store
            .sync_node_tags_async(node_ref.clone(), &["alpha".to_string()], None)
            .await
            .expect("resync");

        let records = store
            .query_tag_records_async(SemanticTagQueryFilter {
                tenant_id: Some("default".to_string()),
                limit: 10,
                ..SemanticTagQueryFilter::default()
            })
            .await
            .expect("query");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].tag, "alpha");
    }

    #[tokio::test]
    async fn find_sync_keys_match_all() {
        let store = InMemorySemanticIndexStore::new();

        for (sync_key, tags) in [("a", vec!["x", "y"]), ("b", vec!["x"])] {
            store
                .sync_node_tags_async(
                    SemanticTagNodeRef {
                        tenant_id: "default".to_string(),
                        session_id: "demo".to_string(),
                        node_id: sync_key.to_string(),
                        sync_key: sync_key.to_string(),
                    },
                    &tags.into_iter().map(ToString::to_string).collect::<Vec<_>>(),
                    None,
                )
                .await
                .expect("sync");
        }

        let keys = store
            .find_sync_keys_by_tags_async(
                "default",
                &["x".to_string(), "y".to_string()],
                true,
                None,
                10,
            )
            .await
            .expect("lookup");

        assert_eq!(keys, vec!["a".to_string()]);
    }
}
