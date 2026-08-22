use std::sync::Arc;

use locus_core_rs::application::services::StoreContextService;
use locus_core_rs::application::validation::TreeSitterValidator;
use locus_core_rs::domain::contracts::{
    NodeStore, NodeStoreInitializer, NodeValidator, SemanticIndexStore,
    SemanticIndexStoreInitializer,
};
use locus_core_rs::parsing::SttpNodeParser;
use locus_core_rs::storage::{InMemoryNodeStore, InMemorySemanticIndexStore};
use locus_sdk::application::memory_find::MemoryFindService;
use locus_sdk::application::memory_recall::MemoryRecallService;
use locus_sdk::application::memory_schema::MemorySchemaService;
use locus_sdk::domain::compression::ManualCompressionRequest;
use locus_sdk::interface::dto::{MemoryFindRequestDto, MemoryRecallRequestDto};
use locus_sdk::{
    application::manual_compression::{DefaultManualCompressionLexiconProvider, ManualCompressionService},
    domain::memory::{MemoryFindRequest, MemoryRecallRequest},
};
use wasm_bindgen::prelude::*;

use crate::dto::{find_response, recall_response, store_response};
use crate::json;

pub struct LocusClient {
    store_context: Arc<StoreContextService>,
    memory_find: Arc<MemoryFindService>,
    memory_recall: Arc<MemoryRecallService>,
}

impl LocusClient {
    pub fn in_memory() -> Self {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        let semantic_index: Arc<dyn SemanticIndexStore> = Arc::new(InMemorySemanticIndexStore::new());
        Self::from_store_traits(store, semantic_index)
    }

    #[cfg(feature = "surreal")]
    pub async fn connect_surreal(config: SurrealConnectConfig) -> Result<Self, JsValue> {
        use locus_core_rs::storage::{SurrealDbNodeStore, SurrealDbSemanticIndexStore};
        use locus_surreal_adapter::RuntimeSurrealDbClient;

        let runtime = locus_core_rs::storage::SurrealDbRuntimeOptions {
            root_dir: String::new(),
            use_remote: config.use_remote,
            endpoint: config.endpoint,
            namespace: config.namespace,
            database: config.database,
        };

        let client = Arc::new(
            RuntimeSurrealDbClient::connect(
                &runtime,
                config.user.as_deref(),
                config.password.as_deref(),
            )
            .await
            .map_err(|err| JsValue::from_str(&err.to_string()))?,
        );

        let store = Arc::new(SurrealDbNodeStore::new(client.clone()));
        let initializer: Arc<dyn NodeStoreInitializer> = store.clone();
        initializer
            .initialize_async()
            .await
            .map_err(|err| JsValue::from_str(&err.to_string()))?;

        let semantic_index = Arc::new(SurrealDbSemanticIndexStore::new(client));
        let semantic_initializer: Arc<dyn SemanticIndexStoreInitializer> = semantic_index.clone();
        semantic_initializer
            .initialize_async()
            .await
            .map_err(|err| JsValue::from_str(&err.to_string()))?;

        let store: Arc<dyn NodeStore> = store;
        let semantic_index: Arc<dyn SemanticIndexStore> = semantic_index;
        Ok(Self::from_store_traits(store, semantic_index))
    }

    fn from_store_traits(
        store: Arc<dyn NodeStore>,
        semantic_index: Arc<dyn SemanticIndexStore>,
    ) -> Self {
        let validator: Arc<dyn NodeValidator> = Arc::new(TreeSitterValidator::new());
        let parser = SttpNodeParser::new();
        let store_context = Arc::new(
            StoreContextService::new(store.clone(), validator, parser)
                .with_semantic_index(semantic_index.clone()),
        );
        let memory_find = Arc::new(
            MemoryFindService::new(store.clone()).with_semantic_index(semantic_index.clone()),
        );
        let memory_recall = Arc::new(
            MemoryRecallService::new(store).with_semantic_index(semantic_index),
        );

        Self {
            store_context,
            memory_find,
            memory_recall,
        }
    }

    pub async fn store(&self, raw: &str, session_id: &str) -> Result<JsValue, JsValue> {
        let result = self.store_context.store_async(raw, session_id).await;
        json::to_value(&store_response(result))
    }

    pub async fn find(&self, request: JsValue) -> Result<JsValue, JsValue> {
        let dto: MemoryFindRequestDto = json::from_value(request)?;
        let request: MemoryFindRequest = dto.into();
        let result = self
            .memory_find
            .execute(&request)
            .await
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        json::to_value(&find_response(result))
    }

    pub async fn recall(&self, request: JsValue) -> Result<JsValue, JsValue> {
        let dto: MemoryRecallRequestDto = json::from_value(request)?;
        let request: MemoryRecallRequest = dto.into();
        let result = self
            .memory_recall
            .execute(&request)
            .await
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        json::to_value(&recall_response(result))
    }
}

#[cfg(feature = "surreal")]
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurrealConnectConfig {
    pub endpoint: String,
    pub namespace: String,
    pub database: String,
    #[serde(default)]
    pub use_remote: bool,
    pub user: Option<String>,
    pub password: Option<String>,
}

#[wasm_bindgen]
pub struct WasmLocusClient {
    inner: LocusClient,
}

#[wasm_bindgen]
impl WasmLocusClient {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: LocusClient::in_memory(),
        }
    }

    pub async fn store(&self, raw: &str, session_id: &str) -> Result<JsValue, JsValue> {
        self.inner.store(raw, session_id).await
    }

    pub async fn find(&self, request: JsValue) -> Result<JsValue, JsValue> {
        self.inner.find(request).await
    }

    pub async fn recall(&self, request: JsValue) -> Result<JsValue, JsValue> {
        self.inner.recall(request).await
    }
}

#[cfg(feature = "surreal")]
#[wasm_bindgen]
pub async fn connect_surreal_client(config: JsValue) -> Result<WasmLocusClient, JsValue> {
    let config: SurrealConnectConfig = json::from_value(config)?;
    let inner = LocusClient::connect_surreal(config).await?;
    Ok(WasmLocusClient { inner })
}

pub fn memory_schema_value() -> Result<JsValue, JsValue> {
    let schema = MemorySchemaService::new().execute();
    json::to_value(&crate::dto::schema_response(schema))
}

pub fn compress_text_value(request: JsValue) -> Result<JsValue, JsValue> {
    let request: ManualCompressionRequest = json::from_value(request)?;
    let service =
        ManualCompressionService::with_lexicon_provider(DefaultManualCompressionLexiconProvider);
    let result = service.execute(&request);
    json::to_value(&crate::dto::compression_response(result))
}
