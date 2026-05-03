use std::sync::Arc;

use chrono::Utc;

use crate::domain::contracts::{EmbeddingProvider, NodeStore, NodeValidator};
use crate::domain::models::StoreResult;
use crate::parsing::SttpNodeParser;

pub struct StoreContextService {
    store: Arc<dyn NodeStore>,
    validator: Arc<dyn NodeValidator>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    parser: SttpNodeParser,
}

impl StoreContextService {
    pub fn new(store: Arc<dyn NodeStore>, validator: Arc<dyn NodeValidator>) -> Self {
        Self {
            store,
            validator,
            embedding_provider: None,
            parser: SttpNodeParser::new(),
        }
    }

    pub fn with_embedding_provider(
        store: Arc<dyn NodeStore>,
        validator: Arc<dyn NodeValidator>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            store,
            validator,
            embedding_provider: Some(embedding_provider),
            parser: SttpNodeParser::new(),
        }
    }

    pub async fn store_async(&self, node: &str, session_id: &str) -> StoreResult {
        let validation = self.validator.validate(node);
        if !validation.is_valid {
            return StoreResult {
                node_id: String::new(),
                psi: 0.0,
                valid: false,
                validation_error: Some(format!(
                    "{}: {}",
                    validation.reason,
                    validation.error.unwrap_or_default()
                )),
            };
        }

        let parse_result = self.parser.try_parse(node, session_id);
        if !parse_result.success {
            return StoreResult {
                node_id: String::new(),
                psi: 0.0,
                valid: false,
                validation_error: Some(format!(
                    "ParseFailure: {}",
                    parse_result.error.unwrap_or_default()
                )),
            };
        }

        let mut parsed = match parse_result.node {
            Some(node) => node,
            None => {
                return StoreResult {
                    node_id: String::new(),
                    psi: 0.0,
                    valid: false,
                    validation_error: Some("ParseFailure: missing parsed node".to_string()),
                };
            }
        };

        if let Some(provider) = self.embedding_provider.as_ref() {
            if let Some(embedding_input) =
                build_embedding_input(parsed.context_summary.as_deref(), &parsed.session_id)
            {
                if let Ok(vector) = provider.embed_async(&embedding_input).await {
                    parsed.embedding_dimensions = Some(vector.len());
                    parsed.embedding_model = Some(provider.model_name().to_string());
                    parsed.embedding = Some(vector);
                    parsed.embedded_at = Some(Utc::now());
                }
            }
        }

        match self.store.store_async(parsed.clone()).await {
            Ok(node_id) => StoreResult {
                node_id,
                psi: parsed.psi,
                valid: true,
                validation_error: None,
            },
            Err(err) => StoreResult {
                node_id: String::new(),
                psi: 0.0,
                valid: false,
                validation_error: Some(format!("StoreFailure: {err}")),
            },
        }
    }
}

fn build_embedding_input(context_summary: Option<&str>, session_id: &str) -> Option<String> {
    let summary = context_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let session = session_id.trim();

    if summary.is_none() && session.is_empty() {
        return None;
    }

    Some(match summary {
        Some(summary) if !session.is_empty() => format!("{summary}\nsession_id:{session}"),
        Some(summary) => summary,
        None => format!("session_id:{session}"),
    })
}
