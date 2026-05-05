use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use clap::Parser;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use locus_core::domain::models::{
    self as core_models, ConfidenceBandSummary, MonthlyRollupRequest, NumericRange, PsiRange,
};
use locus_core::domain::contracts::EmbeddingProvider;
use locus_sdk::application::memory_find::MemoryFindService;
use locus_sdk::application::memory_recall::MemoryRecallService;
use locus_sdk::application::memory_transform::MemoryTransformService;
use locus_sdk::domain::memory::{
    FallbackPolicy, MemoryFilter, MemoryFindRequest, MemoryPage, MemoryRecallRequest,
    MemoryScope, MemoryScoring, MemoryTransformOperation, MemoryTransformRequest,
};
use locus_sdk::infrastructure::registry::InMemoryAiProviderRegistry;
use locus_sdk::infrastructure::sttp_native::embedding_provider_adapter::SttpEmbeddingProviderAdapter;

use crate::app_state::AppState;
use crate::constants::{
    DEFAULT_HYBRID_ALPHA, DEFAULT_HYBRID_BETA, FILE_DESCRIPTOR_SET, TENANT_SCAN_LIMIT,
};
use crate::gateway_args::GatewayArgs;
use crate::http_models::*;
use crate::orchestration::{build_state, parse_cors_allowed_origins, shutdown_signal};
#[cfg(test)]
use crate::orchestration::build_in_memory_state;
use crate::providers::resolve_query_embedding;
use crate::tenant::{
    display_session_id, normalize_node_for_tenant, normalize_tenant_value, resolve_grpc_tenant,
    resolve_http_tenant, scope_session_id,
};

pub mod proto {
    tonic::include_proto!("sttp.v1");
}

pub(crate) async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = GatewayArgs::parse();

    if args.http_port == args.grpc_port {
        return Err(anyhow!(
            "--http-port and --grpc-port must be different values"
        ));
    }

    let state = Arc::new(build_state(&args).await?);

    let base_router = Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/calibrate", post(calibrate_handler))
        .route("/api/v1/store", post(store_context_handler))
        .route("/api/store", post(store_context_handler))
        .route("/store", post(store_context_handler))
        .route("/api/v1/avec/score", post(score_avec_handler))
        .route("/api/avec/score", post(score_avec_handler))
        .route("/avec/score", post(score_avec_handler))
        .route("/api/v1/session/rename", post(rename_session_handler))
        .route("/api/session/rename", post(rename_session_handler))
        .route("/session/rename", post(rename_session_handler))
        .route("/api/v1/context", post(get_context_handler))
        .route(
            "/api/v1/context/embeddings",
            post(get_embedding_context_handler),
        )
        .route(
            "/api/context/embeddings",
            post(get_embedding_context_handler),
        )
        .route("/context/embeddings", post(get_embedding_context_handler))
        .route("/api/v1/nodes", get(list_nodes_handler))
        .route("/api/nodes", get(list_nodes_handler))
        .route("/nodes", get(list_nodes_handler))
        .route("/api/v1/graph", get(graph_handler))
        .route("/api/graph", get(graph_handler))
        .route("/graph", get(graph_handler))
        .route("/api/v1/moods", get(get_moods_handler))
        .route("/api/v1/rekey", post(batch_rekey_handler))
        .route(
            "/api/v1/rollups/monthly",
            post(create_monthly_rollup_handler),
        )
        .route(
            "/api/v1/embeddings/migration/preview",
            post(preview_embedding_migration_handler),
        )
        .route(
            "/api/v1/embeddings/migration/run",
            post(run_embedding_migration_handler),
        )
        .with_state(state.clone());

    let http_router = if args.cors_enabled {
        let allowed_origins = parse_cors_allowed_origins(&args.cors_allowed_origins)?;
        let cors_base = CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::OPTIONS])
            .allow_headers(Any);

        let cors = match allowed_origins {
            CorsAllowedOrigins::Any => cors_base.allow_origin(Any),
            CorsAllowedOrigins::Explicit(origins) => cors_base.allow_origin(origins),
        };

        base_router.layer(cors)
    } else {
        base_router
    };

    let grpc_service = GrpcGatewayService::new(state);

    let grpc_addr = SocketAddr::from(([0, 0, 0, 0], args.grpc_port));
    let http_listener = TcpListener::bind(("0.0.0.0", args.http_port)).await?;

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()?;

    info!(
        http_port = args.http_port,
        grpc_port = args.grpc_port,
        cors_enabled = args.cors_enabled,
        cors_allowed_origins = %args.cors_allowed_origins,
        "Starting locus-gateway"
    );

    let http_server =
        axum::serve(http_listener, http_router).with_graceful_shutdown(shutdown_signal());
    let grpc_server = Server::builder()
        .add_service(reflection_service)
        .add_service(
            proto::sttp_gateway_service_server::SttpGatewayServiceServer::new(grpc_service),
        )
        .serve_with_shutdown(grpc_addr, shutdown_signal());

    let (http_result, grpc_result) = tokio::join!(http_server, grpc_server);
    if let Err(err) = http_result {
        error!(error = %err, "HTTP server exited with error");
        return Err(err.into());
    }
    if let Err(err) = grpc_result {
        error!(error = %err, "gRPC server exited with error");
        return Err(err.into());
    }

    Ok(())
}

async fn health_handler() -> Json<Value> {
    Json(json!({ "status": "ok", "transport": "http+grpc" }))
}

async fn calibrate_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CalibrateSessionHttpRequest>,
) -> ApiResult<CalibrationResultDto> {
    let tenant = resolve_http_tenant(request.tenant_id.as_deref(), &headers);
    let scoped_session_id = scope_session_id(&tenant, &request.session_id);

    let trigger = request
        .trigger
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("manual");

    let result = state
        .calibration
        .calibrate_async(
            &scoped_session_id,
            request.stability,
            request.friction,
            request.logic,
            request.autonomy,
            trigger,
        )
        .await
        .map_err(internal_error)?;

    Ok(Json(CalibrationResultDto {
        previous_avec: to_avec_dto(result.previous_avec),
        delta: result.delta,
        drift_classification: format!("{:?}", result.drift_classification),
        trigger: result.trigger,
        trigger_history: result.trigger_history,
        is_first_calibration: result.is_first_calibration,
    }))
}

async fn store_context_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<StoreContextHttpRequest>,
) -> ApiResult<StoreResultDto> {
    let tenant = resolve_http_tenant(request.tenant_id.as_deref(), &headers);
    let scoped_session_id = scope_session_id(&tenant, &request.session_id);

    let result = state
        .store_context
        .store_async(&request.node, &scoped_session_id)
        .await;

    Ok(Json(StoreResultDto {
        node_id: result.node_id,
        psi: result.psi,
        valid: result.valid,
        validation_error: result.validation_error,
        duplicate_skipped: false,
        upsert_status: if result.valid {
            "created".to_string()
        } else {
            "skipped".to_string()
        },
    }))
}

async fn score_avec_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ScoreAvecHttpRequest>,
) -> ApiResult<ScoreAvecResultDto> {
    let _tenant = resolve_http_tenant(request.tenant_id.as_deref(), &headers);

    if request.text.trim().is_empty() {
        return Err(bad_request("text cannot be empty"));
    }

    let scorer = state.avec_scorer.as_ref().ok_or_else(|| {
        bad_request("AVEC scoring is disabled; enable LOCUS_GATEWAY_AVEC_SCORING_ENABLED")
    })?;

    let avec = scorer
        .score_async(request.text.trim())
        .await
        .map_err(internal_error)?;

    Ok(Json(ScoreAvecResultDto {
        provider: scorer.provider_name().to_string(),
        model: scorer.model_name().to_string(),
        avec: to_avec_dto(avec),
    }))
}

async fn rename_session_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<RenameSessionHttpRequest>,
) -> ApiResult<RenameSessionResultDto> {
    let tenant = resolve_http_tenant(request.tenant_id.as_deref(), &headers);
    let source_session_id = request.source_session_id.trim();
    let target_session_id = request.target_session_id.trim();

    if source_session_id.is_empty() || target_session_id.is_empty() {
        return Err(bad_request(
            "sourceSessionId and targetSessionId are required",
        ));
    }

    if source_session_id == target_session_id {
        return Ok(Json(RenameSessionResultDto {
            source_session_id: source_session_id.to_string(),
            target_session_id: target_session_id.to_string(),
            moved_nodes: 0,
            moved_calibrations: 0,
            scopes_applied: 0,
        }));
    }

    let scoped_source_session_id = scope_session_id(&tenant, source_session_id);
    let scoped_target_session_id = scope_session_id(&tenant, target_session_id);

    let source_nodes = state
        .node_store
        .query_nodes_async(core_models::NodeQuery {
            limit: 10_000,
            session_id: Some(scoped_source_session_id.clone()),
            from_utc: None,
            to_utc: None,
            tiers: None,
        })
        .await
        .map_err(internal_error)?;

    if source_nodes.is_empty() {
        return Err(bad_request(format!(
            "source session not found: {source_session_id}"
        )));
    }

    let mut anchor_node_ids = Vec::with_capacity(source_nodes.len());
    for node in source_nodes {
        let upsert = state
            .node_store
            .upsert_node_async(node)
            .await
            .map_err(internal_error)?;
        anchor_node_ids.push(upsert.node_id);
    }
    anchor_node_ids.sort();
    anchor_node_ids.dedup();

    let rekey_result = state
        .rekey_scope
        .rekey_async(
            anchor_node_ids,
            &tenant,
            &scoped_target_session_id,
            false,
            request.allow_merge.unwrap_or(false),
        )
        .await
        .map_err(internal_error)?;

    if let Some(conflict) = rekey_result.scopes.iter().find(|scope| scope.conflict) {
        return Err(bad_request(
            conflict
                .message
                .clone()
                .unwrap_or_else(|| "target session already exists".to_string()),
        ));
    }

    let scopes_applied = rekey_result
        .scopes
        .iter()
        .filter(|scope| scope.applied)
        .count();

    Ok(Json(RenameSessionResultDto {
        source_session_id: source_session_id.to_string(),
        target_session_id: target_session_id.to_string(),
        moved_nodes: rekey_result.temporal_nodes_updated,
        moved_calibrations: rekey_result.calibrations_updated,
        scopes_applied,
    }))
}

async fn get_context_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<GetContextHttpRequest>,
) -> ApiResult<RetrieveResultDto> {
    let tenant = resolve_http_tenant(request.tenant_id.as_deref(), &headers);
    let scoped_session_id = scope_session_id(&tenant, &request.session_id);

    let limit = request.limit.unwrap_or(5);
    let tiers = normalize_request_tiers(request.tiers.as_deref());
    let query_embedding = resolve_query_embedding(
        state.embedding_provider.as_ref(),
        request.query_text.as_deref(),
        request.query_embedding.as_deref(),
    )
    .await;
    let recall_service = MemoryRecallService::new(state.node_store.clone());
    let recall_result = recall_service
        .execute(&MemoryRecallRequest {
            scope: MemoryScope {
                tenant_id: None,
                session_ids: Some(vec![scoped_session_id]),
                tiers,
                from_utc: request.from_utc,
                to_utc: request.to_utc,
            },
            page: MemoryPage {
                limit,
                cursor: None,
            },
            scoring: MemoryScoring {
                alpha: request
                    .alpha
                    .unwrap_or(DEFAULT_HYBRID_ALPHA)
                    .clamp(0.0, 1.0),
                beta: request.beta.unwrap_or(DEFAULT_HYBRID_BETA).clamp(0.0, 1.0),
                fallback_policy: FallbackPolicy::Never,
                ..Default::default()
            },
            current_avec: Some(core_models::AvecState {
                stability: request.stability,
                friction: request.friction,
                logic: request.logic,
                autonomy: request.autonomy,
            }),
            query_text: request.query_text,
            query_embedding,
            ..Default::default()
        })
        .await
        .map_err(internal_error)?;

    Ok(Json(RetrieveResultDto {
        nodes: recall_result.nodes.iter().map(to_node_dto).collect(),
        retrieved: recall_result.retrieved,
        psi_range: PsiRangeDto {
            min: recall_result.psi_range.min,
            max: recall_result.psi_range.max,
            average: recall_result.psi_range.average,
        },
    }))
}

async fn get_embedding_context_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<GetEmbeddingContextHttpRequest>,
) -> ApiResult<RetrieveResultDto> {
    let tenant = resolve_http_tenant(request.tenant_id.as_deref(), &headers);
    let scoped_session_id = scope_session_id(&tenant, &request.session_id);
    let limit = request.limit.unwrap_or(5);
    let tiers = normalize_request_tiers(request.tiers.as_deref());

    let rag_embedding = resolve_query_embedding(
        state.embedding_provider.as_ref(),
        request.rag_query_text.as_deref(),
        request.rag_embedding.as_deref(),
    )
    .await;

    let avec_embedding = resolve_query_embedding(
        state.embedding_provider.as_ref(),
        request.avec_query_text.as_deref(),
        request.avec_embedding.as_deref(),
    )
    .await;

    let fused_embedding = fuse_weighted_embeddings(
        rag_embedding.as_deref(),
        avec_embedding.as_deref(),
        request.rag_weight.unwrap_or(0.7),
        request.avec_weight.unwrap_or(0.3),
    )?;

    if fused_embedding.is_empty() {
        return Err(bad_request(
            "Provide ragEmbedding/ragQueryText and/or avecEmbedding/avecQueryText",
        ));
    }

    let result = state
        .context_query
        .get_context_hybrid_scoped_filtered_async(
            Some(&scoped_session_id),
            request.stability,
            request.friction,
            request.logic,
            request.autonomy,
            request.from_utc,
            request.to_utc,
            tiers.as_deref(),
            Some(fused_embedding.as_slice()),
            request
                .alpha
                .unwrap_or(DEFAULT_HYBRID_ALPHA)
                .clamp(0.0, 1.0),
            request.beta.unwrap_or(DEFAULT_HYBRID_BETA).clamp(0.0, 1.0),
            limit,
        )
        .await;

    Ok(Json(RetrieveResultDto {
        nodes: result.nodes.iter().map(to_node_dto).collect(),
        retrieved: result.retrieved,
        psi_range: PsiRangeDto {
            min: result.psi_range.min,
            max: result.psi_range.max,
            average: result.psi_range.average,
        },
    }))
}

fn fuse_weighted_embeddings(
    rag_embedding: Option<&[f32]>,
    avec_embedding: Option<&[f32]>,
    rag_weight: f32,
    avec_weight: f32,
) -> Result<Vec<f32>, (StatusCode, Json<ErrorResponse>)> {
    let rag_weight = rag_weight.clamp(0.0, 1.0);
    let avec_weight = avec_weight.clamp(0.0, 1.0);

    match (rag_embedding, avec_embedding) {
        (Some(rag), Some(avec)) => {
            if rag.len() != avec.len() {
                return Err(bad_request(
                    "rag embedding and avec embedding must have the same dimensions",
                ));
            }

            let sum = rag_weight + avec_weight;
            let denom = if sum > f32::EPSILON { sum } else { 1.0 };
            let fused = rag
                .iter()
                .zip(avec.iter())
                .map(|(r, a)| ((r * rag_weight) + (a * avec_weight)) / denom)
                .collect::<Vec<_>>();
            Ok(fused)
        }
        (Some(rag), None) => Ok(rag.to_vec()),
        (None, Some(avec)) => Ok(avec.to_vec()),
        (None, None) => Ok(Vec::new()),
    }
}

async fn list_nodes_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListNodesQuery>,
) -> ApiResult<ListNodesResultDto> {
    let tenant = resolve_http_tenant(query.tenant_id.as_deref(), &headers);
    let requested_limit = query.limit.unwrap_or(50).clamp(1, TENANT_SCAN_LIMIT);
    let scoped_session_filter = query
        .session_id
        .as_deref()
        .map(|session_id| scope_session_id(&tenant, session_id));
    let backend_limit = if scoped_session_filter.is_some() {
        requested_limit
    } else {
        TENANT_SCAN_LIMIT
    };

    let find_service = MemoryFindService::new(state.node_store.clone());
    let result = find_service
        .execute(&MemoryFindRequest {
            scope: MemoryScope {
                tenant_id: None,
                session_ids: scoped_session_filter.map(|session| vec![session]),
                tiers: None,
                from_utc: None,
                to_utc: None,
            },
            page: MemoryPage {
                limit: backend_limit,
                cursor: None,
            },
            ..Default::default()
        })
        .await
        .map_err(internal_error)?;

    let nodes = result
        .nodes
        .into_iter()
        .filter_map(|node| normalize_node_for_tenant(node, &tenant))
        .take(requested_limit)
        .collect::<Vec<_>>();

    Ok(Json(ListNodesResultDto {
        nodes: nodes.iter().map(to_node_dto).collect(),
        retrieved: nodes.len(),
    }))
}

async fn graph_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<GraphQuery>,
) -> ApiResult<GraphResponse> {
    let tenant = resolve_http_tenant(query.tenant_id.as_deref(), &headers);
    let capped_limit = query.limit.unwrap_or(1000).clamp(1, 5000);
    let scoped_session_filter = query
        .session_id
        .as_deref()
        .map(|session_id| scope_session_id(&tenant, session_id));
    let backend_limit = if scoped_session_filter.is_some() {
        capped_limit
    } else {
        TENANT_SCAN_LIMIT
    };

    let find_service = MemoryFindService::new(state.node_store.clone());
    let result = find_service
        .execute(&MemoryFindRequest {
            scope: MemoryScope {
                tenant_id: None,
                session_ids: scoped_session_filter.map(|session| vec![session]),
                tiers: None,
                from_utc: None,
                to_utc: None,
            },
            page: MemoryPage {
                limit: backend_limit,
                cursor: None,
            },
            ..Default::default()
        })
        .await
        .map_err(internal_error)?;

    let mut ordered_nodes = result
        .nodes
        .into_iter()
        .filter_map(|node| normalize_node_for_tenant(node, &tenant))
        .take(capped_limit)
        .collect::<Vec<_>>();
    ordered_nodes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    #[derive(Clone)]
    struct SessionGroup {
        id: String,
        label: String,
        nodes: Vec<core_models::SttpNode>,
        node_count: usize,
        avg_psi: f32,
        last_modified: DateTime<Utc>,
        size: usize,
    }

    let mut grouped_map: BTreeMap<String, Vec<core_models::SttpNode>> = BTreeMap::new();
    for node in &ordered_nodes {
        grouped_map
            .entry(node.session_id.clone())
            .or_default()
            .push(node.clone());
    }

    let mut grouped = grouped_map
        .into_iter()
        .map(|(id, mut nodes)| {
            nodes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            let node_count = nodes.len();
            let avg_psi = if node_count == 0 {
                0.0
            } else {
                nodes.iter().map(|n| n.psi).sum::<f32>() / node_count as f32
            };
            let last_modified = nodes.first().map(|n| n.timestamp).unwrap_or_else(Utc::now);
            let size = 16 + std::cmp::min(28, node_count * 2);

            SessionGroup {
                label: id.clone(),
                id,
                nodes,
                node_count,
                avg_psi,
                last_modified,
                size,
            }
        })
        .collect::<Vec<_>>();

    grouped.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    let node_by_id = ordered_nodes
        .iter()
        .map(|node| (graph_node_id(node), node.clone()))
        .collect::<HashMap<_, _>>();

    let sessions = grouped
        .iter()
        .map(|session| {
            json!({
                "id": format!("s:{}", session.id),
                "label": session.label,
                "nodeCount": session.node_count,
                "avgPsi": session.avg_psi,
                "lastModified": session.last_modified.to_rfc3339(),
                "size": session.size
            })
        })
        .collect::<Vec<_>>();

    let nodes = ordered_nodes
        .iter()
        .map(|node| {
            json!({
                "id": graph_node_id(node),
                "sessionId": node.session_id,
                "label": format!("{} {}", node.tier, node.timestamp.format("%m-%d %H:%M")),
                "tier": node.tier,
                "timestamp": node.timestamp.to_rfc3339(),
                "psi": node.psi,
                "parentNodeId": node.parent_node_id,
                "size": 9
            })
        })
        .collect::<Vec<_>>();

    let mut edges = Vec::new();

    for i in 0..grouped.len().saturating_sub(1) {
        edges.push(json!({
            "id": format!("t-{i}"),
            "source": format!("s:{}", grouped[i].id),
            "target": format!("s:{}", grouped[i + 1].id),
            "kind": "timeline"
        }));
    }

    for i in 0..grouped.len() {
        let from = &grouped[i];
        let mut nearest: Option<usize> = None;
        let mut nearest_distance = f32::MAX;

        for (j, other) in grouped.iter().enumerate() {
            if i == j {
                continue;
            }
            let distance = (from.avg_psi - other.avg_psi).abs();
            if distance < nearest_distance {
                nearest_distance = distance;
                nearest = Some(j);
            }
        }

        if let Some(nearest_index) = nearest {
            if i < nearest_index {
                edges.push(json!({
                    "id": format!("s-{i}-{nearest_index}"),
                    "source": format!("s:{}", from.id),
                    "target": format!("s:{}", grouped[nearest_index].id),
                    "kind": "similarity"
                }));
            }
        }
    }

    for session in &grouped {
        for i in 0..session.nodes.len() {
            let current = &session.nodes[i];
            let current_id = graph_node_id(current);

            edges.push(json!({
                "id": format!("m-{}-{i}", session.id),
                "source": format!("s:{}", session.id),
                "target": current_id,
                "kind": "membership"
            }));

            if i + 1 < session.nodes.len() {
                let older = &session.nodes[i + 1];
                edges.push(json!({
                    "id": format!("nt-{}-{i}", session.id),
                    "source": current_id,
                    "target": graph_node_id(older),
                    "kind": "node_timeline"
                }));
            }

            if let Some(parent) = current.parent_node_id.as_ref() {
                if node_by_id.contains_key(parent) {
                    edges.push(json!({
                        "id": format!("l-{}-{i}", session.id),
                        "source": current_id,
                        "target": parent,
                        "kind": "lineage"
                    }));
                }
            }
        }
    }

    Ok(Json(GraphResponse {
        sessions,
        nodes,
        edges,
        retrieved: ordered_nodes.len(),
    }))
}

async fn get_moods_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetMoodsQuery>,
) -> ApiResult<MoodCatalogResultDto> {
    let result = state.mood_catalog.get(
        query.target_mood.as_deref(),
        query.blend.unwrap_or(1.0),
        query.current_stability,
        query.current_friction,
        query.current_logic,
        query.current_autonomy,
    );

    Ok(Json(to_mood_catalog_dto(result)))
}

async fn create_monthly_rollup_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateMonthlyRollupHttpRequest>,
) -> ApiResult<MonthlyRollupResultDto> {
    let tenant = resolve_http_tenant(request.tenant_id.as_deref(), &headers);

    let rollup_request = MonthlyRollupRequest {
        session_id: scope_session_id(&tenant, &request.session_id),
        start_utc: request.start_date_utc,
        end_utc: request.end_date_utc,
        source_session_id: request
            .source_session_id
            .map(|session_id| scope_session_id(&tenant, &session_id)),
        parent_node_id: request.parent_node_id,
        persist: request.persist.unwrap_or(true),
        limit: request.limit.unwrap_or(5000),
    };

    let result = state.monthly_rollup.create_async(rollup_request).await;
    Ok(Json(to_monthly_rollup_dto(result)))
}

async fn batch_rekey_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<BatchRekeyHttpRequest>,
) -> ApiResult<BatchRekeyResultDto> {
    if request.node_ids.is_empty() {
        return Err(bad_request("nodeIds must contain at least one value"));
    }

    if request.target_session_id.trim().is_empty() {
        return Err(bad_request("targetSessionId cannot be empty"));
    }

    let target_tenant = resolve_http_tenant(request.target_tenant_id.as_deref(), &headers);
    let scoped_target_session = scope_session_id(&target_tenant, request.target_session_id.trim());

    let result = state
        .rekey_scope
        .rekey_async(
            request.node_ids,
            &target_tenant,
            &scoped_target_session,
            request.dry_run.unwrap_or(true),
            request.allow_merge.unwrap_or(false),
        )
        .await
        .map_err(internal_error)?;

    Ok(Json(to_batch_rekey_dto(result)))
}

async fn preview_embedding_migration_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<EmbeddingMigrationPreviewHttpRequest>,
) -> ApiResult<EmbeddingMigrationPreviewResultDto> {
    let tenant = resolve_http_tenant(request.tenant_id.as_deref(), &headers);
    let sample_limit = request.sample_limit.unwrap_or(20).clamp(1, 200);
    let max_nodes = request.max_nodes.unwrap_or(5_000).clamp(1, 50_000);
    let (scope, filter, sync_keys) = scoped_memory_filter(request.filter, &tenant);

    let find_service = MemoryFindService::new(state.node_store.clone());
    let find_result = find_service
        .execute(&MemoryFindRequest {
            scope,
            filter,
            page: MemoryPage {
                limit: max_nodes,
                cursor: None,
            },
            ..Default::default()
        })
        .await
        .map_err(internal_error)?;

    let mut nodes = find_result.nodes;
    if let Some(sync_keys) = sync_keys {
        nodes.retain(|node| sync_keys.iter().any(|key| key == &node.sync_key));
    }

    let total_candidates = nodes.len();
    let sample = nodes
        .into_iter()
        .take(sample_limit)
        .map(|sample| EmbeddingMigrationSampleDto {
            sync_key: sample.sync_key,
            session_id: display_session_id(&sample.session_id),
            tier: sample.tier,
            has_embedding: sample
                .embedding
                .as_ref()
                .is_some_and(|values| !values.is_empty()),
            embedding_model: sample.embedding_model,
            embedding_dimensions: sample.embedding_dimensions,
            embedded_at: sample.embedded_at,
            updated_at: sample.updated_at,
            context_summary: sample.context_summary,
        })
        .collect::<Vec<_>>();

    Ok(Json(EmbeddingMigrationPreviewResultDto {
        total_candidates,
        sample,
        provider_available: state.embedding_provider.is_some(),
        provider_model: state
            .embedding_provider
            .as_ref()
            .map(|provider| provider.model_name().to_string()),
    }))
}

async fn run_embedding_migration_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<EmbeddingMigrationRunHttpRequest>,
) -> ApiResult<EmbeddingMigrationRunResultDto> {
    let tenant = resolve_http_tenant(request.tenant_id.as_deref(), &headers);
    let mode = match request
        .mode
        .unwrap_or(EmbeddingMigrationModeHttp::MissingOnly)
    {
        EmbeddingMigrationModeHttp::MissingOnly => MemoryTransformOperation::EmbedBackfill,
        EmbeddingMigrationModeHttp::ReindexAll => MemoryTransformOperation::ReindexEmbeddings,
    };
    let dry_run = request.dry_run.unwrap_or(true);
    let batch_size = request.batch_size.unwrap_or(100).clamp(1, 500);
    let max_nodes = request.max_nodes.unwrap_or(5_000).clamp(1, 50_000);
    let (scope, filter, _sync_keys) = scoped_memory_filter(request.filter, &tenant);

    let providers = build_gateway_provider_registry(state.embedding_provider.clone());
    let transform_service = MemoryTransformService::new(state.node_store.clone(), providers);

    let result = transform_service
        .execute(&MemoryTransformRequest {
            scope,
            filter,
            operation: mode,
            dry_run,
            batch_size,
            max_nodes,
            provider_id: state
                .embedding_provider
                .as_ref()
                .map(|_| "gateway-embedding".to_string()),
            model: state
                .embedding_provider
                .as_ref()
                .map(|provider| provider.model_name().to_string()),
        })
        .await
        .map_err(internal_error)?;

    Ok(Json(to_embedding_migration_run_dto(
        result,
        mode,
        dry_run,
        state
            .embedding_provider
            .as_ref()
            .map(|provider| provider.model_name().to_string()),
    )))
}

fn bad_request(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

fn internal_error(error: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
}

fn graph_node_id(node: &core_models::SttpNode) -> String {
    format!(
        "n:{}|{}|{}|{:.4}",
        node.session_id,
        node.timestamp.to_rfc3339(),
        node.compression_depth,
        node.psi
    )
}

fn to_avec_dto(value: core_models::AvecState) -> AvecStateDto {
    AvecStateDto {
        stability: value.stability,
        friction: value.friction,
        logic: value.logic,
        autonomy: value.autonomy,
        psi: value.psi(),
    }
}

fn to_node_dto(value: &core_models::SttpNode) -> SttpNodeDto {
    SttpNodeDto {
        raw: value.raw.clone(),
        session_id: display_session_id(&value.session_id),
        tier: value.tier.clone(),
        timestamp: value.timestamp,
        compression_depth: value.compression_depth,
        parent_node_id: value.parent_node_id.clone(),
        user_avec: to_avec_dto(value.user_avec),
        model_avec: to_avec_dto(value.model_avec),
        compression_avec: value.compression_avec.map(to_avec_dto),
        rho: value.rho,
        kappa: value.kappa,
        psi: value.psi,
        sync_key: value.sync_key.clone(),
        synthetic_id: graph_node_id(value),
    }
}

fn to_mood_catalog_dto(result: core_models::MoodCatalogResult) -> MoodCatalogResultDto {
    MoodCatalogResultDto {
        presets: result
            .presets
            .into_iter()
            .map(|preset| MoodPresetDto {
                name: preset.name,
                description: preset.description,
                avec: to_avec_dto(preset.avec),
            })
            .collect(),
        apply_guide: result.apply_guide,
        swap_preview: result.swap_preview.map(|preview| MoodSwapPreviewDto {
            target_mood: preview.target_mood,
            blend: preview.blend,
            current: to_avec_dto(preview.current),
            target: to_avec_dto(preview.target),
            blended: to_avec_dto(preview.blended),
        }),
    }
}

fn to_monthly_rollup_dto(result: core_models::MonthlyRollupResult) -> MonthlyRollupResultDto {
    MonthlyRollupResultDto {
        success: result.success,
        node_id: result.node_id,
        raw_node: result.raw_node,
        error: result.error,
        source_nodes: result.source_nodes,
        parent_reference: result.parent_reference,
        user_average: to_avec_dto(result.user_average),
        model_average: to_avec_dto(result.model_average),
        compression_average: to_avec_dto(result.compression_average),
        rho_range: to_numeric_range_dto(result.rho_range),
        kappa_range: to_numeric_range_dto(result.kappa_range),
        psi_range: to_numeric_range_dto(result.psi_range),
        rho_bands: to_confidence_bands_dto(result.rho_bands),
        kappa_bands: to_confidence_bands_dto(result.kappa_bands),
    }
}

fn to_batch_rekey_dto(result: core_models::BatchRekeyResult) -> BatchRekeyResultDto {
    let updated_scopes = result.scopes.iter().filter(|scope| scope.applied).count();
    let conflict_scopes = result.scopes.iter().filter(|scope| scope.conflict).count();

    BatchRekeyResultDto {
        dry_run: result.dry_run,
        requested_node_ids: result.requested_node_ids,
        resolved_node_ids: result.resolved_node_ids,
        missing_node_ids: result.missing_node_ids,
        scopes: result
            .scopes
            .into_iter()
            .map(|scope| ScopeRekeyResultDto {
                source_tenant_id: scope.source_tenant_id,
                source_session_id: display_session_id(&scope.source_session_id),
                target_tenant_id: scope.target_tenant_id,
                target_session_id: display_session_id(&scope.target_session_id),
                temporal_nodes: scope.temporal_nodes,
                calibrations: scope.calibrations,
                target_temporal_nodes: scope.target_temporal_nodes,
                target_calibrations: scope.target_calibrations,
                applied: scope.applied,
                conflict: scope.conflict,
                message: scope.message,
            })
            .collect(),
        temporal_nodes_updated: result.temporal_nodes_updated,
        calibrations_updated: result.calibrations_updated,
        updated_scopes,
        conflict_scopes,
    }
}

fn scoped_memory_filter(
    request_filter: Option<EmbeddingMigrationFilterHttp>,
    tenant: &str,
) -> (MemoryScope, MemoryFilter, Option<Vec<String>>) {
    let filter = request_filter.unwrap_or(EmbeddingMigrationFilterHttp {
        session_id: None,
        from_utc: None,
        to_utc: None,
        tiers: None,
        has_embedding: None,
        embedding_model: None,
        sync_keys: None,
    });

    (
        MemoryScope {
            tenant_id: None,
            session_ids: filter
                .session_id
                .map(|session_id| vec![scope_session_id(tenant, &session_id)]),
            tiers: filter.tiers,
            from_utc: filter.from_utc,
            to_utc: filter.to_utc,
        },
        MemoryFilter {
            has_embedding: filter.has_embedding,
            embedding_model: filter.embedding_model,
            ..Default::default()
        },
        filter.sync_keys,
    )
}

fn build_gateway_provider_registry(
    provider: Option<Arc<dyn EmbeddingProvider>>,
) -> Arc<InMemoryAiProviderRegistry> {
    let mut registry = InMemoryAiProviderRegistry::new();
    if let Some(provider) = provider {
        registry.register(SttpEmbeddingProviderAdapter::new("gateway-embedding", provider));
    }
    Arc::new(registry)
}

fn to_embedding_migration_run_dto(
    result: locus_sdk::domain::memory::MemoryTransformResult,
    mode: MemoryTransformOperation,
    dry_run: bool,
    provider_model: Option<String>,
) -> EmbeddingMigrationRunResultDto {
    EmbeddingMigrationRunResultDto {
        scanned: result.scanned,
        selected: result.selected,
        updated: result.updated,
        skipped: result.skipped,
        failed: result.failed,
        duplicate: result.duplicate,
        started_at: result.started_at,
        completed_at: result.completed_at,
        provider_model,
        dry_run,
        mode: match mode {
            MemoryTransformOperation::EmbedBackfill => "missing_only".to_string(),
            MemoryTransformOperation::ReindexEmbeddings => "reindex_all".to_string(),
        },
        failure_reasons: result.failures,
    }
}

fn to_numeric_range_dto(value: NumericRange) -> NumericRangeDto {
    NumericRangeDto {
        min: value.min,
        max: value.max,
        average: value.average,
    }
}

fn to_confidence_bands_dto(value: ConfidenceBandSummary) -> ConfidenceBandSummaryDto {
    ConfidenceBandSummaryDto {
        low: value.low,
        medium: value.medium,
        high: value.high,
    }
}

#[derive(Clone)]
struct GrpcGatewayService {
    state: Arc<AppState>,
}

impl GrpcGatewayService {
    fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl proto::sttp_gateway_service_server::SttpGatewayService for GrpcGatewayService {
    async fn calibrate_session(
        &self,
        request: Request<proto::CalibrateSessionRequest>,
    ) -> Result<Response<proto::CalibrateSessionReply>, Status> {
        let tenant = resolve_grpc_tenant(request.metadata());
        let request = request.into_inner();
        let trigger = if request.trigger.trim().is_empty() {
            "manual"
        } else {
            &request.trigger
        };
        let scoped_session_id = scope_session_id(&tenant, &request.session_id);

        let result = self
            .state
            .calibration
            .calibrate_async(
                &scoped_session_id,
                request.stability,
                request.friction,
                request.logic,
                request.autonomy,
                trigger,
            )
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        let reply = proto::CalibrateSessionReply {
            previous_avec: Some(to_grpc_avec(result.previous_avec)),
            delta: result.delta,
            drift_classification: format!("{:?}", result.drift_classification),
            trigger: result.trigger,
            trigger_history: result.trigger_history,
            is_first_calibration: result.is_first_calibration,
        };

        Ok(Response::new(reply))
    }

    async fn store_context(
        &self,
        request: Request<proto::StoreContextRequest>,
    ) -> Result<Response<proto::StoreContextReply>, Status> {
        let tenant = resolve_grpc_tenant(request.metadata());
        let request = request.into_inner();
        let scoped_session_id = scope_session_id(&tenant, &request.session_id);

        let result = self
            .state
            .store_context
            .store_async(&request.node, &scoped_session_id)
            .await;

        let reply = proto::StoreContextReply {
            node_id: result.node_id,
            psi: result.psi,
            valid: result.valid,
            validation_error: result.validation_error,
        };

        Ok(Response::new(reply))
    }

    async fn get_context(
        &self,
        request: Request<proto::GetContextRequest>,
    ) -> Result<Response<proto::GetContextReply>, Status> {
        let tenant = resolve_grpc_tenant(request.metadata());
        let request = request.into_inner();
        let scoped_session_id = scope_session_id(&tenant, &request.session_id);

        let query_embedding = resolve_query_embedding(
            self.state.embedding_provider.as_ref(),
            request.query_text.as_deref(),
            if request.query_embedding.is_empty() {
                None
            } else {
                Some(request.query_embedding.as_slice())
            },
        )
        .await;

        let limit = if request.limit <= 0 {
            5
        } else {
            request.limit as usize
        };
        let tiers = normalize_request_tiers(Some(&request.tiers));
        let recall_service = MemoryRecallService::new(self.state.node_store.clone());
        let recall_result = recall_service
            .execute(&MemoryRecallRequest {
                scope: MemoryScope {
                    tenant_id: None,
                    session_ids: Some(vec![scoped_session_id]),
                    tiers,
                    from_utc: timestamp_from_proto_optional(request.from_utc)?,
                    to_utc: timestamp_from_proto_optional(request.to_utc)?,
                },
                page: MemoryPage {
                    limit,
                    cursor: None,
                },
                scoring: MemoryScoring {
                    alpha: request
                        .alpha
                        .unwrap_or(DEFAULT_HYBRID_ALPHA)
                        .clamp(0.0, 1.0),
                    beta: request.beta.unwrap_or(DEFAULT_HYBRID_BETA).clamp(0.0, 1.0),
                    fallback_policy: FallbackPolicy::Never,
                    ..Default::default()
                },
                current_avec: Some(core_models::AvecState {
                    stability: request.stability,
                    friction: request.friction,
                    logic: request.logic,
                    autonomy: request.autonomy,
                }),
                query_text: request.query_text,
                query_embedding,
                ..Default::default()
            })
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        let nodes = recall_result
            .nodes
            .iter()
            .cloned()
            .filter_map(|node| normalize_node_for_tenant(node, &tenant))
            .collect::<Vec<_>>();

        let reply = proto::GetContextReply {
            nodes: nodes.iter().map(to_grpc_node).collect(),
            retrieved: clamp_usize_to_i32(nodes.len()),
            psi_range: Some(to_grpc_psi_range(recall_result.psi_range)),
        };

        Ok(Response::new(reply))
    }

    async fn get_embedding_context(
        &self,
        request: Request<proto::GetEmbeddingContextRequest>,
    ) -> Result<Response<proto::GetContextReply>, Status> {
        let tenant = resolve_grpc_tenant(request.metadata());
        let request = request.into_inner();
        let scoped_session_id = scope_session_id(&tenant, &request.session_id);

        let rag_embedding = resolve_query_embedding(
            self.state.embedding_provider.as_ref(),
            request.rag_query_text.as_deref(),
            if request.rag_embedding.is_empty() {
                None
            } else {
                Some(request.rag_embedding.as_slice())
            },
        )
        .await;

        let avec_embedding = resolve_query_embedding(
            self.state.embedding_provider.as_ref(),
            request.avec_query_text.as_deref(),
            if request.avec_embedding.is_empty() {
                None
            } else {
                Some(request.avec_embedding.as_slice())
            },
        )
        .await;

        let fused_embedding = fuse_weighted_embeddings(
            rag_embedding.as_deref(),
            avec_embedding.as_deref(),
            request.rag_weight.unwrap_or(0.7),
            request.avec_weight.unwrap_or(0.3),
        )
        .map_err(|(_, payload)| Status::invalid_argument(payload.0.error))?;

        if fused_embedding.is_empty() {
            return Err(Status::invalid_argument(
                "Provide rag_embedding/rag_query_text and/or avec_embedding/avec_query_text",
            ));
        }

        let limit = if request.limit <= 0 {
            5
        } else {
            request.limit as usize
        };
        let tiers = normalize_request_tiers(Some(&request.tiers));

        let result = self
            .state
            .context_query
            .get_context_hybrid_scoped_filtered_async(
                Some(&scoped_session_id),
                request.stability,
                request.friction,
                request.logic,
                request.autonomy,
                timestamp_from_proto_optional(request.from_utc)?,
                timestamp_from_proto_optional(request.to_utc)?,
                tiers.as_deref(),
                Some(fused_embedding.as_slice()),
                request
                    .alpha
                    .unwrap_or(DEFAULT_HYBRID_ALPHA)
                    .clamp(0.0, 1.0),
                request.beta.unwrap_or(DEFAULT_HYBRID_BETA).clamp(0.0, 1.0),
                limit,
            )
            .await;

        let nodes = result
            .nodes
            .iter()
            .cloned()
            .filter_map(|node| normalize_node_for_tenant(node, &tenant))
            .collect::<Vec<_>>();

        let reply = proto::GetContextReply {
            nodes: nodes.iter().map(to_grpc_node).collect(),
            retrieved: clamp_usize_to_i32(nodes.len()),
            psi_range: Some(to_grpc_psi_range(result.psi_range)),
        };

        Ok(Response::new(reply))
    }

    async fn list_nodes(
        &self,
        request: Request<proto::ListNodesRequest>,
    ) -> Result<Response<proto::ListNodesReply>, Status> {
        let tenant = resolve_grpc_tenant(request.metadata());
        let request = request.into_inner();
        let requested_limit = if request.limit <= 0 {
            50
        } else {
            request.limit as usize
        }
        .clamp(1, TENANT_SCAN_LIMIT);
        let scoped_session_filter = request
            .session_id
            .as_deref()
            .map(|session_id| scope_session_id(&tenant, session_id));
        let backend_limit = if scoped_session_filter.is_some() {
            requested_limit
        } else {
            TENANT_SCAN_LIMIT
        };

        let find_service = MemoryFindService::new(self.state.node_store.clone());
        let result = find_service
            .execute(&MemoryFindRequest {
                scope: MemoryScope {
                    tenant_id: None,
                    session_ids: scoped_session_filter.map(|session| vec![session]),
                    tiers: None,
                    from_utc: None,
                    to_utc: None,
                },
                page: MemoryPage {
                    limit: backend_limit,
                    cursor: None,
                },
                ..Default::default()
            })
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        let nodes = result
            .nodes
            .into_iter()
            .filter_map(|node| normalize_node_for_tenant(node, &tenant))
            .take(requested_limit)
            .collect::<Vec<_>>();

        let reply = proto::ListNodesReply {
            nodes: nodes.iter().map(to_grpc_node).collect(),
            retrieved: clamp_usize_to_i32(nodes.len()),
        };

        Ok(Response::new(reply))
    }

    async fn get_moods(
        &self,
        request: Request<proto::GetMoodsRequest>,
    ) -> Result<Response<proto::GetMoodsReply>, Status> {
        let request = request.into_inner();
        let result = self.state.mood_catalog.get(
            request.target_mood.as_deref(),
            request.blend,
            request.current_stability,
            request.current_friction,
            request.current_logic,
            request.current_autonomy,
        );

        let reply = proto::GetMoodsReply {
            presets: result
                .presets
                .into_iter()
                .map(|preset| proto::MoodPreset {
                    name: preset.name,
                    description: preset.description,
                    avec: Some(to_grpc_avec(preset.avec)),
                })
                .collect(),
            apply_guide: result.apply_guide,
            swap_preview: result.swap_preview.map(|preview| proto::MoodSwapPreview {
                target_mood: preview.target_mood,
                blend: preview.blend,
                current: Some(to_grpc_avec(preview.current)),
                target: Some(to_grpc_avec(preview.target)),
                blended: Some(to_grpc_avec(preview.blended)),
            }),
        };

        Ok(Response::new(reply))
    }

    async fn batch_rekey(
        &self,
        request: Request<proto::BatchRekeyRequest>,
    ) -> Result<Response<proto::BatchRekeyReply>, Status> {
        let metadata_tenant = resolve_grpc_tenant(request.metadata());
        let request = request.into_inner();

        if request.node_ids.is_empty() {
            return Err(Status::invalid_argument(
                "node_ids must contain at least one value",
            ));
        }

        if request.target_session_id.trim().is_empty() {
            return Err(Status::invalid_argument(
                "target_session_id cannot be empty",
            ));
        }

        let target_tenant = request
            .target_tenant_id
            .as_deref()
            .and_then(normalize_tenant_value)
            .unwrap_or(metadata_tenant);
        let scoped_target_session =
            scope_session_id(&target_tenant, request.target_session_id.trim());

        let result = self
            .state
            .rekey_scope
            .rekey_async(
                request.node_ids,
                &target_tenant,
                &scoped_target_session,
                request.dry_run.unwrap_or(true),
                request.allow_merge.unwrap_or(false),
            )
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        Ok(Response::new(to_grpc_batch_rekey_reply(result)))
    }

    async fn create_monthly_rollup(
        &self,
        request: Request<proto::CreateMonthlyRollupRequest>,
    ) -> Result<Response<proto::CreateMonthlyRollupReply>, Status> {
        let tenant = resolve_grpc_tenant(request.metadata());
        let request = request.into_inner();

        let start_utc = timestamp_from_proto(request.start_utc)?;
        let end_utc = timestamp_from_proto(request.end_utc)?;

        let monthly_request = MonthlyRollupRequest {
            session_id: scope_session_id(&tenant, &request.session_id),
            start_utc,
            end_utc,
            source_session_id: request
                .source_session_id
                .map(|session_id| scope_session_id(&tenant, &session_id)),
            parent_node_id: request.parent_node_id,
            persist: request.persist,
            limit: if request.limit <= 0 {
                5000
            } else {
                request.limit as usize
            },
        };

        let result = self
            .state
            .monthly_rollup
            .create_async(monthly_request)
            .await;

        let reply = proto::CreateMonthlyRollupReply {
            success: result.success,
            node_id: result.node_id,
            raw_node: result.raw_node,
            error: result.error,
            source_nodes: clamp_usize_to_i32(result.source_nodes),
            parent_reference: result.parent_reference,
            user_average: Some(to_grpc_avec(result.user_average)),
            model_average: Some(to_grpc_avec(result.model_average)),
            compression_average: Some(to_grpc_avec(result.compression_average)),
            rho_range: Some(to_grpc_numeric_range(result.rho_range)),
            kappa_range: Some(to_grpc_numeric_range(result.kappa_range)),
            psi_range: Some(to_grpc_numeric_range(result.psi_range)),
            rho_bands: Some(to_grpc_confidence_bands(result.rho_bands)),
            kappa_bands: Some(to_grpc_confidence_bands(result.kappa_bands)),
        };

        Ok(Response::new(reply))
    }
}

fn to_grpc_avec(value: core_models::AvecState) -> proto::AvecState {
    proto::AvecState {
        stability: value.stability,
        friction: value.friction,
        logic: value.logic,
        autonomy: value.autonomy,
        psi: value.psi(),
    }
}

fn to_grpc_node(value: &core_models::SttpNode) -> proto::SttpNode {
    proto::SttpNode {
        raw: value.raw.clone(),
        session_id: display_session_id(&value.session_id),
        tier: value.tier.clone(),
        timestamp: Some(timestamp_to_proto(value.timestamp)),
        compression_depth: value.compression_depth,
        parent_node_id: value.parent_node_id.clone(),
        user_avec: Some(to_grpc_avec(value.user_avec)),
        model_avec: Some(to_grpc_avec(value.model_avec)),
        compression_avec: value.compression_avec.map(to_grpc_avec),
        rho: value.rho,
        kappa: value.kappa,
        psi: value.psi,
    }
}

fn to_grpc_psi_range(value: PsiRange) -> proto::PsiRange {
    proto::PsiRange {
        min: value.min,
        max: value.max,
        average: value.average,
    }
}

fn to_grpc_numeric_range(value: NumericRange) -> proto::NumericRange {
    proto::NumericRange {
        min: value.min,
        max: value.max,
        average: value.average,
    }
}

fn to_grpc_confidence_bands(value: ConfidenceBandSummary) -> proto::ConfidenceBandSummary {
    proto::ConfidenceBandSummary {
        low: clamp_usize_to_i32(value.low),
        medium: clamp_usize_to_i32(value.medium),
        high: clamp_usize_to_i32(value.high),
    }
}

fn to_grpc_scope_rekey_result(value: core_models::ScopeRekeyResult) -> proto::ScopeRekeyResult {
    proto::ScopeRekeyResult {
        source_tenant_id: value.source_tenant_id,
        source_session_id: display_session_id(&value.source_session_id),
        target_tenant_id: value.target_tenant_id,
        target_session_id: display_session_id(&value.target_session_id),
        temporal_nodes: clamp_usize_to_i32(value.temporal_nodes),
        calibrations: clamp_usize_to_i32(value.calibrations),
        target_temporal_nodes: clamp_usize_to_i32(value.target_temporal_nodes),
        target_calibrations: clamp_usize_to_i32(value.target_calibrations),
        applied: value.applied,
        conflict: value.conflict,
        message: value.message,
    }
}

fn to_grpc_batch_rekey_reply(value: core_models::BatchRekeyResult) -> proto::BatchRekeyReply {
    let updated_scopes = value.scopes.iter().filter(|scope| scope.applied).count();
    let conflict_scopes = value.scopes.iter().filter(|scope| scope.conflict).count();

    proto::BatchRekeyReply {
        dry_run: value.dry_run,
        requested_node_ids: clamp_usize_to_i32(value.requested_node_ids),
        resolved_node_ids: clamp_usize_to_i32(value.resolved_node_ids),
        missing_node_ids: value.missing_node_ids,
        scopes: value
            .scopes
            .into_iter()
            .map(to_grpc_scope_rekey_result)
            .collect(),
        temporal_nodes_updated: clamp_usize_to_i32(value.temporal_nodes_updated),
        calibrations_updated: clamp_usize_to_i32(value.calibrations_updated),
        updated_scopes: clamp_usize_to_i32(updated_scopes),
        conflict_scopes: clamp_usize_to_i32(conflict_scopes),
    }
}

fn timestamp_to_proto(value: DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}

fn timestamp_from_proto(value: Option<prost_types::Timestamp>) -> Result<DateTime<Utc>, Status> {
    let value = value.ok_or_else(|| Status::invalid_argument("missing timestamp"))?;
    DateTime::<Utc>::from_timestamp(value.seconds, value.nanos as u32)
        .ok_or_else(|| Status::invalid_argument("invalid timestamp"))
}

fn timestamp_from_proto_optional(
    value: Option<prost_types::Timestamp>,
) -> Result<Option<DateTime<Utc>>, Status> {
    value
        .map(|timestamp| timestamp_from_proto(Some(timestamp)))
        .transpose()
}

fn normalize_request_tiers(tiers: Option<&[String]>) -> Option<Vec<String>> {
    let normalized = tiers
        .unwrap_or(&[])
        .iter()
        .map(|tier| tier.trim().to_ascii_lowercase())
        .filter(|tier| !tier.is_empty())
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn clamp_usize_to_i32(value: usize) -> i32 {
    if value > i32::MAX as usize {
        i32::MAX
    } else {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::proto::sttp_gateway_service_server::SttpGatewayService;
    use crate::gateway_args::{EmbeddingsProviderKind, GatewayBackend};
    use crate::providers::parse_avec_state_from_text;
    use crate::tenant::session_belongs_to_tenant;
    use locus_core::storage::{
        SurrealDbEndpointsSettings, SurrealDbRuntimeOptions, SurrealDbSettings,
    };

    fn sample_node(session_id: &str) -> String {
        format!(
            r#"
⊕⟨ {{ trigger: manual, response_format: temporal_node, origin_session: "{session_id}", compression_depth: 1, parent_node: null, prime: {{ attractor_config: {{ stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }}, context_summary: "gateway test", relevant_tier: raw, retrieval_budget: 3 }} }} ⟩
⦿⟨ {{ timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "{session_id}", user_avec: {{ stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }}, model_avec: {{ stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }} }} ⟩
◈⟨ {{ test(.99): "gateway parser check" }} ⟩
⍉⟨ {{ rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: {{ stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }} }} ⟩
"#
        )
    }

    fn surreal_test_args() -> GatewayArgs {
        GatewayArgs {
            http_port: 8080,
            grpc_port: 8081,
            backend: GatewayBackend::Surreal,
            root_dir_name: ".locus-gateway".to_string(),
            remote: true,
            cors_enabled: true,
            cors_allowed_origins: "*".to_string(),
            surreal_embedded_endpoint: None,
            surreal_remote_endpoint: Some("ws://127.0.0.1:8000/rpc".to_string()),
            surreal_namespace: "entasis".to_string(),
            surreal_database: "locus_gateway".to_string(),
            surreal_user: "root".to_string(),
            surreal_password: "root".to_string(),
            embeddings_enabled: false,
            embeddings_provider: EmbeddingsProviderKind::Ollama,
            embeddings_endpoint: "http://127.0.0.1:11434/api/embeddings".to_string(),
            embeddings_model: "sttp-encoder".to_string(),
            embeddings_repo: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            avec_scoring_enabled: false,
            avec_scoring_endpoint: "http://127.0.0.1:11434/api/chat".to_string(),
            avec_scoring_model: "qwen2.5:0.5b".to_string(),
        }
    }

    #[test]
    fn parse_avec_state_accepts_plain_json() {
        let value = parse_avec_state_from_text(
            r#"{"stability":0.8,"friction":0.2,"logic":0.9,"autonomy":0.7}"#,
        )
        .expect("plain JSON should parse");

        assert!((value.stability - 0.8).abs() < f32::EPSILON);
        assert!((value.friction - 0.2).abs() < f32::EPSILON);
        assert!((value.logic - 0.9).abs() < f32::EPSILON);
        assert!((value.autonomy - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn parse_avec_state_extracts_embedded_json() {
        let value = parse_avec_state_from_text(
            "Here is the answer: {\"stability\":1.1,\"friction\":-0.1,\"logic\":0.5,\"autonomy\":0.6}",
        )
        .expect("embedded JSON should parse");

        assert!((value.stability - 1.0).abs() < f32::EPSILON);
        assert!((value.friction - 0.0).abs() < f32::EPSILON);
        assert!((value.logic - 0.5).abs() < f32::EPSILON);
        assert!((value.autonomy - 0.6).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn score_avec_returns_bad_request_when_disabled() {
        let state = Arc::new(build_in_memory_state().await.expect("state should build"));

        let err = score_avec_handler(
            State(state),
            HeaderMap::new(),
            Json(ScoreAvecHttpRequest {
                text: "hello world".to_string(),
                tenant_id: None,
            }),
        )
        .await
        .expect_err("scoring should fail when disabled");

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn surreal_runtime_options_are_derived_from_gateway_args() {
        let args = surreal_test_args();
        let mut settings = SurrealDbSettings::default();
        settings.endpoints = SurrealDbEndpointsSettings {
            embedded: args.surreal_embedded_endpoint.clone(),
            remote: args.surreal_remote_endpoint.clone(),
        };
        settings.namespace = args.surreal_namespace.clone();
        settings.database = args.surreal_database.clone();

        let runtime_args = vec!["--remote".to_string()];
        let runtime = SurrealDbRuntimeOptions::from_args(
            &runtime_args,
            &settings,
            Some(args.root_dir_name.as_str()),
        )
        .expect("runtime options should be computed");

        assert!(runtime.use_remote);
        assert_eq!(runtime.endpoint, "ws://127.0.0.1:8000/rpc");
        assert_eq!(runtime.namespace, "entasis");
        assert_eq!(runtime.database, "locus_gateway");
    }

    #[test]
    fn tenant_scoping_is_applied_only_for_non_default_tenant() {
        let scoped = scope_session_id("acme", "session-1");
        assert_eq!(scoped, "tenant:acme::session:session-1");
        assert!(session_belongs_to_tenant(&scoped, "acme"));
        assert!(!session_belongs_to_tenant(&scoped, "default"));
        assert_eq!(display_session_id(&scoped), "session-1");

        let legacy = scope_session_id("default", "session-1");
        assert_eq!(legacy, "session-1");
        assert!(session_belongs_to_tenant(&legacy, "default"));
    }

    #[tokio::test]
    async fn http_calibrate_defaults_trigger_to_manual() {
        let state = Arc::new(build_in_memory_state().await.expect("state should build"));

        let request = CalibrateSessionHttpRequest {
            session_id: "http-calibrate-session".to_string(),
            tenant_id: None,
            stability: 0.8,
            friction: 0.2,
            logic: 0.8,
            autonomy: 0.7,
            trigger: None,
        };

        let Json(reply) = calibrate_handler(State(state), HeaderMap::new(), Json(request))
            .await
            .expect("calibrate should succeed");

        assert_eq!(reply.trigger, "manual");
        assert!(reply.is_first_calibration);
    }

    #[tokio::test]
    async fn http_store_then_get_context_roundtrip() {
        let state = Arc::new(build_in_memory_state().await.expect("state should build"));
        let session_id = "http-store-session";

        let Json(store_reply) = store_context_handler(
            State(state.clone()),
            HeaderMap::new(),
            Json(StoreContextHttpRequest {
                node: sample_node(session_id),
                session_id: session_id.to_string(),
                tenant_id: None,
            }),
        )
        .await
        .expect("store should succeed");

        assert!(store_reply.valid);
        assert!(!store_reply.node_id.is_empty());

        let Json(context_reply) = get_context_handler(
            State(state),
            HeaderMap::new(),
            Json(GetContextHttpRequest {
                session_id: session_id.to_string(),
                tenant_id: None,
                stability: 0.85,
                friction: 0.25,
                logic: 0.80,
                autonomy: 0.70,
                limit: Some(5),
                from_utc: None,
                to_utc: None,
                tiers: None,
                query_text: Some("gateway parser check".to_string()),
                query_embedding: None,
                alpha: None,
                beta: None,
            }),
        )
        .await
        .expect("get_context should succeed");

        assert!(context_reply.retrieved >= 1);
        assert!(
            context_reply
                .nodes
                .iter()
                .any(|node| node.session_id == session_id)
        );
    }

    #[tokio::test]
    async fn http_embedding_context_roundtrip_with_direct_embeddings() {
        let state = Arc::new(build_in_memory_state().await.expect("state should build"));
        let session_id = "http-embedding-context-session";

        let Json(store_reply) = store_context_handler(
            State(state.clone()),
            HeaderMap::new(),
            Json(StoreContextHttpRequest {
                node: sample_node(session_id),
                session_id: session_id.to_string(),
                tenant_id: None,
            }),
        )
        .await
        .expect("store should succeed");

        assert!(store_reply.valid);

        let Json(context_reply) = get_embedding_context_handler(
            State(state),
            HeaderMap::new(),
            Json(GetEmbeddingContextHttpRequest {
                session_id: session_id.to_string(),
                tenant_id: None,
                stability: 0.85,
                friction: 0.25,
                logic: 0.80,
                autonomy: 0.70,
                limit: Some(5),
                from_utc: None,
                to_utc: None,
                tiers: None,
                rag_query_text: None,
                rag_embedding: Some(vec![0.1, 0.2, 0.3]),
                avec_query_text: None,
                avec_embedding: Some(vec![0.2, 0.2, 0.2]),
                rag_weight: Some(0.7),
                avec_weight: Some(0.3),
                alpha: Some(0.65),
                beta: Some(0.35),
            }),
        )
        .await
        .expect("embedding context should succeed");

        assert!(context_reply.retrieved >= 1);
    }

    #[tokio::test]
    async fn http_embedding_context_rejects_mismatched_embedding_dimensions() {
        let state = Arc::new(build_in_memory_state().await.expect("state should build"));

        let err = get_embedding_context_handler(
            State(state),
            HeaderMap::new(),
            Json(GetEmbeddingContextHttpRequest {
                session_id: "session".to_string(),
                tenant_id: None,
                stability: 0.8,
                friction: 0.2,
                logic: 0.8,
                autonomy: 0.7,
                limit: Some(5),
                from_utc: None,
                to_utc: None,
                tiers: None,
                rag_query_text: None,
                rag_embedding: Some(vec![0.1, 0.2, 0.3]),
                avec_query_text: None,
                avec_embedding: Some(vec![0.3, 0.2]),
                rag_weight: Some(0.7),
                avec_weight: Some(0.3),
                alpha: None,
                beta: None,
            }),
        )
        .await
        .expect_err("mismatched dimensions should fail");

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn grpc_service_roundtrip_for_calibrate_store_and_list() {
        let state = Arc::new(build_in_memory_state().await.expect("state should build"));
        let service = GrpcGatewayService::new(state);
        let session_id = "grpc-store-session";

        let calibrate_reply = service
            .calibrate_session(Request::new(proto::CalibrateSessionRequest {
                session_id: session_id.to_string(),
                stability: 0.8,
                friction: 0.2,
                logic: 0.8,
                autonomy: 0.7,
                trigger: String::new(),
            }))
            .await
            .expect("gRPC calibrate should succeed")
            .into_inner();

        assert_eq!(calibrate_reply.trigger, "manual");

        let store_reply = service
            .store_context(Request::new(proto::StoreContextRequest {
                node: sample_node(session_id),
                session_id: session_id.to_string(),
            }))
            .await
            .expect("gRPC store should succeed")
            .into_inner();

        assert!(store_reply.valid);

        let rekey_reply = service
            .batch_rekey(Request::new(proto::BatchRekeyRequest {
                node_ids: vec![store_reply.node_id.clone()],
                target_session_id: session_id.to_string(),
                target_tenant_id: Some("default".to_string()),
                dry_run: Some(true),
                allow_merge: Some(false),
            }))
            .await
            .expect("gRPC batch_rekey should succeed")
            .into_inner();

        assert_eq!(rekey_reply.requested_node_ids, 1);
        assert_eq!(rekey_reply.resolved_node_ids, 1);
        assert!(rekey_reply.scopes.iter().any(|scope| !scope.applied));

        let list_reply = service
            .list_nodes(Request::new(proto::ListNodesRequest {
                limit: 50,
                session_id: Some(session_id.to_string()),
            }))
            .await
            .expect("gRPC list_nodes should succeed")
            .into_inner();

        assert!(list_reply.retrieved >= 1);
        assert!(
            list_reply
                .nodes
                .iter()
                .any(|node| node.session_id == session_id)
        );

        let context_reply = service
            .get_context(Request::new(proto::GetContextRequest {
                session_id: session_id.to_string(),
                stability: 0.8,
                friction: 0.2,
                logic: 0.8,
                autonomy: 0.7,
                limit: 5,
                query_text: None,
                query_embedding: vec![0.1, 0.2, 0.3],
                alpha: Some(0.7),
                beta: Some(0.3),
                from_utc: None,
                to_utc: None,
                tiers: Vec::new(),
            }))
            .await
            .expect("gRPC get_context should succeed")
            .into_inner();

        assert!(context_reply.retrieved >= 1);

        let embedding_context_reply = service
            .get_embedding_context(Request::new(proto::GetEmbeddingContextRequest {
                session_id: session_id.to_string(),
                stability: 0.8,
                friction: 0.2,
                logic: 0.8,
                autonomy: 0.7,
                limit: 5,
                rag_query_text: None,
                rag_embedding: vec![0.1, 0.2, 0.3],
                avec_query_text: None,
                avec_embedding: vec![0.2, 0.2, 0.2],
                rag_weight: Some(0.7),
                avec_weight: Some(0.3),
                alpha: Some(0.7),
                beta: Some(0.3),
                from_utc: None,
                to_utc: None,
                tiers: Vec::new(),
            }))
            .await
            .expect("gRPC get_embedding_context should succeed")
            .into_inner();

        assert!(embedding_context_reply.retrieved >= 1);

        let err = service
            .get_embedding_context(Request::new(proto::GetEmbeddingContextRequest {
                session_id: session_id.to_string(),
                stability: 0.8,
                friction: 0.2,
                logic: 0.8,
                autonomy: 0.7,
                limit: 5,
                rag_query_text: None,
                rag_embedding: vec![0.1, 0.2, 0.3],
                avec_query_text: None,
                avec_embedding: vec![0.2, 0.2],
                rag_weight: Some(0.7),
                avec_weight: Some(0.3),
                alpha: None,
                beta: None,
                from_utc: None,
                to_utc: None,
                tiers: Vec::new(),
            }))
            .await
            .expect_err("gRPC get_embedding_context should reject invalid dimensions");

        assert_eq!(err.code(), Status::invalid_argument("x").code());
    }
}
