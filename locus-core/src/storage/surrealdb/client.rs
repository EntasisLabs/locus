use std::collections::BTreeMap;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

pub type QueryParams = BTreeMap<String, Value>;

#[async_trait]
pub trait SurrealDbClient: Send + Sync {
    async fn raw_query(&self, query: &str, parameters: QueryParams) -> Result<Vec<Value>>;
}
