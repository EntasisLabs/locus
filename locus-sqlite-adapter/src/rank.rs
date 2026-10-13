use locus_core_rs::domain::models::{AvecState, SttpNode};

pub(crate) fn rank_nodes_hybrid(
    nodes: &mut [SttpNode],
    current_avec: AvecState,
    query_embedding: Option<&[f32]>,
    alpha: f32,
    beta: f32,
) {
    let alpha = alpha.clamp(0.0, 1.0);
    let beta = beta.clamp(0.0, 1.0);

    nodes.sort_by(|left, right| {
        let left_score = hybrid_score(left, current_avec, query_embedding, alpha, beta);
        let right_score = hybrid_score(right, current_avec, query_embedding, alpha, beta);

        right_score
            .total_cmp(&left_score)
            .then_with(|| right.updated_at.cmp(&left.updated_at))
    });
}

fn hybrid_score(
    node: &SttpNode,
    target_avec: AvecState,
    query_embedding: Option<&[f32]>,
    alpha: f32,
    beta: f32,
) -> f32 {
    let resonance = (1.0 - avec_distance(&node.model_avec, &target_avec)).clamp(0.0, 1.0);

    let semantic = match (query_embedding, node.embedding.as_deref()) {
        (Some(query), Some(node_vec)) => {
            cosine_similarity(query, node_vec).map(|value| (value + 1.0) / 2.0)
        }
        _ => None,
    };

    match semantic {
        Some(value) => (alpha * resonance + beta * value).clamp(0.0, 1.0),
        None => resonance,
    }
}

pub(crate) fn avec_distance(left: &AvecState, right: &AvecState) -> f32 {
    ((left.stability - right.stability).abs()
        + (left.friction - right.friction).abs()
        + (left.logic - right.logic).abs()
        + (left.autonomy - right.autonomy).abs())
        / 4.0
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return None;
    }

    let mut dot = 0.0f32;
    let mut left_norm = 0.0f32;
    let mut right_norm = 0.0f32;

    for (l, r) in left.iter().zip(right.iter()) {
        dot += l * r;
        left_norm += l * l;
        right_norm += r * r;
    }

    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        return None;
    }

    Some(dot / (left_norm.sqrt() * right_norm.sqrt()))
}
