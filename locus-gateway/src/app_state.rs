use std::sync::Arc;

use locus_core::application::services::{
    CalibrationService, ContextQueryService, MonthlyRollupService, MoodCatalogService,
    RekeyScopeService, StoreContextService,
};
use locus_core::domain::contracts::{EmbeddingProvider, NodeStore};

use crate::providers::AvecScorer;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) node_store: Arc<dyn NodeStore>,
    pub(crate) embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    pub(crate) avec_scorer: Option<Arc<dyn AvecScorer>>,
    pub(crate) calibration: Arc<CalibrationService>,
    pub(crate) context_query: Arc<ContextQueryService>,
    pub(crate) mood_catalog: Arc<MoodCatalogService>,
    pub(crate) store_context: Arc<StoreContextService>,
    pub(crate) monthly_rollup: Arc<MonthlyRollupService>,
    pub(crate) rekey_scope: Arc<RekeyScopeService>,
}
