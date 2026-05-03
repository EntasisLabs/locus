use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::domain::contracts::NodeStore;
use crate::domain::models::{AvecState, ListNodesResult, PsiRange, RetrieveResult, SttpNode};

pub struct ContextQueryService {
    store: Arc<dyn NodeStore>,
}

impl ContextQueryService {
    pub fn new(store: Arc<dyn NodeStore>) -> Self {
        Self { store }
    }

    pub async fn get_context_async(
        &self,
        session_id: &str,
        stability: f32,
        friction: f32,
        logic: f32,
        autonomy: f32,
        limit: usize,
    ) -> RetrieveResult {
        self.get_context_scoped_filtered_async(
            Some(session_id),
            stability,
            friction,
            logic,
            autonomy,
            None,
            None,
            None,
            limit,
        )
        .await
    }

    /// Retrieve context across all sessions (global memory mode).
    pub async fn get_context_global_async(
        &self,
        stability: f32,
        friction: f32,
        logic: f32,
        autonomy: f32,
        limit: usize,
    ) -> RetrieveResult {
        self.get_context_scoped_filtered_async(
            None, stability, friction, logic, autonomy, None, None, None, limit,
        )
        .await
    }

    pub async fn get_context_global_filtered_async(
        &self,
        stability: f32,
        friction: f32,
        logic: f32,
        autonomy: f32,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        limit: usize,
    ) -> RetrieveResult {
        self.get_context_scoped_filtered_async(
            None, stability, friction, logic, autonomy, from_utc, to_utc, tiers, limit,
        )
        .await
    }

    /// Retrieve context with an optional session scope.
    pub async fn get_context_scoped_async(
        &self,
        session_id: Option<&str>,
        stability: f32,
        friction: f32,
        logic: f32,
        autonomy: f32,
        limit: usize,
    ) -> RetrieveResult {
        self.get_context_scoped_filtered_async(
            session_id, stability, friction, logic, autonomy, None, None, None, limit,
        )
        .await
    }

    pub async fn get_context_scoped_filtered_async(
        &self,
        session_id: Option<&str>,
        stability: f32,
        friction: f32,
        logic: f32,
        autonomy: f32,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        limit: usize,
    ) -> RetrieveResult {
        let current = AvecState {
            stability,
            friction,
            logic,
            autonomy,
        };

        let nodes = match session_id {
            Some(session_id) => match self
                .store
                .get_by_resonance_async(session_id, current, from_utc, to_utc, tiers, limit)
                .await
            {
                Ok(nodes) => nodes,
                Err(_) => return empty_retrieve_result(),
            },
            None => match self
                .store
                .get_by_resonance_global_async(current, from_utc, to_utc, tiers, limit)
                .await
            {
                Ok(nodes) => nodes,
                Err(_) => return empty_retrieve_result(),
            },
        };

        to_retrieve_result(nodes)
    }

    pub async fn get_context_hybrid_async(
        &self,
        session_id: &str,
        stability: f32,
        friction: f32,
        logic: f32,
        autonomy: f32,
        query_embedding: Option<&[f32]>,
        alpha: f32,
        beta: f32,
        limit: usize,
    ) -> RetrieveResult {
        self.get_context_hybrid_scoped_filtered_async(
            Some(session_id),
            stability,
            friction,
            logic,
            autonomy,
            None,
            None,
            None,
            query_embedding,
            alpha,
            beta,
            limit,
        )
        .await
    }

    /// Retrieve hybrid context across all sessions (global memory mode).
    pub async fn get_context_hybrid_global_async(
        &self,
        stability: f32,
        friction: f32,
        logic: f32,
        autonomy: f32,
        query_embedding: Option<&[f32]>,
        alpha: f32,
        beta: f32,
        limit: usize,
    ) -> RetrieveResult {
        self.get_context_hybrid_scoped_filtered_async(
            None,
            stability,
            friction,
            logic,
            autonomy,
            None,
            None,
            None,
            query_embedding,
            alpha,
            beta,
            limit,
        )
        .await
    }

    pub async fn get_context_hybrid_global_filtered_async(
        &self,
        stability: f32,
        friction: f32,
        logic: f32,
        autonomy: f32,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        query_embedding: Option<&[f32]>,
        alpha: f32,
        beta: f32,
        limit: usize,
    ) -> RetrieveResult {
        self.get_context_hybrid_scoped_filtered_async(
            None,
            stability,
            friction,
            logic,
            autonomy,
            from_utc,
            to_utc,
            tiers,
            query_embedding,
            alpha,
            beta,
            limit,
        )
        .await
    }

    /// Retrieve hybrid context with an optional session scope.
    pub async fn get_context_hybrid_scoped_async(
        &self,
        session_id: Option<&str>,
        stability: f32,
        friction: f32,
        logic: f32,
        autonomy: f32,
        query_embedding: Option<&[f32]>,
        alpha: f32,
        beta: f32,
        limit: usize,
    ) -> RetrieveResult {
        self.get_context_hybrid_scoped_filtered_async(
            session_id,
            stability,
            friction,
            logic,
            autonomy,
            None,
            None,
            None,
            query_embedding,
            alpha,
            beta,
            limit,
        )
        .await
    }

    pub async fn get_context_hybrid_scoped_filtered_async(
        &self,
        session_id: Option<&str>,
        stability: f32,
        friction: f32,
        logic: f32,
        autonomy: f32,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        query_embedding: Option<&[f32]>,
        alpha: f32,
        beta: f32,
        limit: usize,
    ) -> RetrieveResult {
        let current = AvecState {
            stability,
            friction,
            logic,
            autonomy,
        };

        let nodes = match session_id {
            Some(session_id) => match self
                .store
                .get_by_hybrid_async(
                    session_id,
                    current,
                    from_utc,
                    to_utc,
                    tiers,
                    query_embedding,
                    alpha,
                    beta,
                    limit,
                )
                .await
            {
                Ok(nodes) => nodes,
                Err(_) => return empty_retrieve_result(),
            },
            None => match self
                .store
                .get_by_hybrid_global_async(
                    current,
                    from_utc,
                    to_utc,
                    tiers,
                    query_embedding,
                    alpha,
                    beta,
                    limit,
                )
                .await
            {
                Ok(nodes) => nodes,
                Err(_) => return empty_retrieve_result(),
            },
        };

        to_retrieve_result(nodes)
    }

    pub async fn list_nodes_async(
        &self,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<ListNodesResult> {
        let capped_limit = limit.clamp(1, 200);
        let nodes = self
            .store
            .list_nodes_async(capped_limit, session_id)
            .await?;
        Ok(ListNodesResult {
            retrieved: nodes.len(),
            nodes,
        })
    }
}

fn empty_retrieve_result() -> RetrieveResult {
    RetrieveResult {
        nodes: Vec::new(),
        retrieved: 0,
        psi_range: PsiRange {
            min: 0.0,
            max: 0.0,
            average: 0.0,
        },
    }
}

fn to_retrieve_result(nodes: Vec<SttpNode>) -> RetrieveResult {
    if nodes.is_empty() {
        return empty_retrieve_result();
    }

    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    let mut sum = 0.0f32;

    for node in &nodes {
        min = min.min(node.psi);
        max = max.max(node.psi);
        sum += node.psi;
    }

    let retrieved = nodes.len();

    RetrieveResult {
        retrieved,
        nodes,
        psi_range: PsiRange {
            min,
            max,
            average: sum / (retrieved as f32),
        },
    }
}
