//! HTTP contract coverage for bounded public graph reads and opaque neighbor cursors.

use std::{collections::BTreeSet, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
  body::{to_bytes, Body},
  http::{header, Request, StatusCode},
  Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::Value;
use tower::ServiceExt;
use transnet::{
  adapters::in_memory::{InMemoryGraphRepository, InMemoryMetricsRecorder},
  app_router,
  application::graph::GraphService,
  domain::{
    canonical::{CanonicalId, EvidenceConfidence, LanguageTag, LexicalPartOfSpeech},
    graph::{
      GraphContentVersion, GraphEdgeId, GraphEvidence, GraphFeedbackCapability, GraphNode,
      GraphNodeKey, GraphNodeKind, GraphRanking, GraphRelationType, GraphScope, GraphScore,
      GraphScoreComponents, RelationVersion, StoredGraphRelation,
    },
    observability::{GraphOperation, MetricEvent, MetricOutcome},
  },
  ports::graph_repository::{
    GraphAdjacency, GraphAdjacencyRequest, GraphRepository, GraphRepositoryError,
  },
  AppState, GraphCursorProtectionKey, ProviderConfig, TranslationConfig, TranslationService,
};

fn service() -> TranslationService {
  let provider = ProviderConfig {
    base_url: "http://127.0.0.1:1/v1".to_string(),
    model: "unused".to_string(),
    api_key: "unused".to_string().into(),
  };
  TranslationService::new(
    TranslationConfig {
      long_text_chars: 4_000,
      timeout_seconds: 1,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider.clone(),
    provider,
  )
  .unwrap()
}

fn id(value: &str) -> CanonicalId {
  CanonicalId::new(value).unwrap()
}

fn key(value: &str) -> GraphNodeKey {
  GraphNodeKey::new(GraphNodeKind::Sense, id(value))
}

fn content() -> GraphContentVersion {
  GraphContentVersion {
    release_id: id("release-2026-09"),
    ranking_version: "graph-rank-v1".to_string(),
    community_aggregate_version: "community-v1".to_string(),
  }
}

fn node(value: &str, label: &str) -> GraphNode {
  GraphNode::new(
    key(value),
    label,
    Some(LanguageTag::parse("en").unwrap()),
    Some(LexicalPartOfSpeech::Adjective),
    Some(format!("{label} definition")),
    true,
  )
  .unwrap()
}

fn ranking(score: u16) -> GraphRanking {
  GraphRanking {
    display_rank: GraphScore::new(score).unwrap(),
    components: GraphScoreComponents {
      evidence: GraphScore::new(9_000).unwrap(),
      community: Some(GraphScore::new(750).unwrap()),
      pedagogical: Some(GraphScore::new(300).unwrap()),
    },
    ranking_version: "graph-rank-v1".to_string(),
  }
}

fn relation(edge_id: &str, target: &str, score: u16) -> StoredGraphRelation {
  StoredGraphRelation {
    edge_id: GraphEdgeId::stored(id(edge_id)),
    relation_version: RelationVersion::new(3).unwrap(),
    source: key("hot"),
    target: key(target),
    relation_type: GraphRelationType::Hypernym,
    evidence: GraphEvidence::new(vec![id("evidence-heat")], EvidenceConfidence::High).unwrap(),
    scope: GraphScope {
      dialect: Some(LanguageTag::parse("en-US").unwrap()),
      domain: Some("weather".to_string()),
      register: Some("neutral".to_string()),
      note: Some("temperature only".to_string()),
    },
    feedback_capabilities: BTreeSet::from([
      GraphFeedbackCapability::Accuracy,
      GraphFeedbackCapability::Usefulness,
    ]),
    ranking: ranking(score),
  }
}

fn graph_service() -> Arc<GraphService> {
  let repository = InMemoryGraphRepository::new(content())
    .with_node(node("hot", "hot"))
    .with_node(node("warm", "warm"))
    .with_node(node("mild", "mild"))
    .with_node(node("cool", "cool"))
    .with_relation(relation("edge-high", "warm", 9_900))
    .with_relation(relation("edge-mid", "mild", 8_000))
    .with_relation(relation("edge-low", "cool", 7_000));
  Arc::new(GraphService::new(Arc::new(repository)))
}

fn graph_app() -> Router {
  app_router(AppState::new(service()).with_graph_service(graph_service()))
}

fn graph_app_with_metrics(recorder: &InMemoryMetricsRecorder) -> Router {
  app_router(
    AppState::new(service())
      .with_metrics_recorder(Arc::new(recorder.clone()))
      .with_graph_service(graph_service()),
  )
}

fn graph_app_with_cursor_key(key: GraphCursorProtectionKey) -> Router {
  app_router(
    AppState::new(service())
      .with_graph_service(graph_service())
      .with_graph_cursor_protection_key(key),
  )
}

fn missing_endpoint_graph_service() -> Arc<GraphService> {
  let repository = InMemoryGraphRepository::new(content())
    .with_node(node("hot", "hot"))
    .with_relation(relation("edge-withheld", "withheld-target", 9_900));
  Arc::new(GraphService::new(Arc::new(repository)))
}

fn graph_app_with_service_and_cursor_key(
  graph: Arc<GraphService>,
  key: GraphCursorProtectionKey,
) -> Router {
  app_router(
    AppState::new(service())
      .with_graph_service(graph)
      .with_graph_cursor_protection_key(key),
  )
}

#[derive(Debug)]
struct UnavailableGraphRepository;

#[async_trait]
impl GraphRepository for UnavailableGraphRepository {
  async fn active_graph_content(&self) -> Result<GraphContentVersion, GraphRepositoryError> {
    Err(GraphRepositoryError::Unavailable)
  }

  async fn load_nodes(
    &self,
    _content: &GraphContentVersion,
    _keys: &[GraphNodeKey],
  ) -> Result<Vec<GraphNode>, GraphRepositoryError> {
    Err(GraphRepositoryError::Unavailable)
  }

  async fn adjacency(
    &self,
    _request: &GraphAdjacencyRequest,
  ) -> Result<GraphAdjacency, GraphRepositoryError> {
    Err(GraphRepositoryError::Unavailable)
  }
}

async fn json(response: axum::response::Response) -> Value {
  serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

async fn recorded_events(
  recorder: &InMemoryMetricsRecorder,
  expected_count: usize,
) -> Vec<MetricEvent> {
  tokio::time::timeout(Duration::from_secs(1), async {
    loop {
      let events = recorder.events().await;
      if events.len() >= expected_count {
        return events;
      }
      tokio::task::yield_now().await;
    }
  })
  .await
  .expect("graph metrics recorder receives the expected closed events")
}

async fn settled_events(recorder: &InMemoryMetricsRecorder) -> Vec<MetricEvent> {
  for _ in 0..4 {
    tokio::task::yield_now().await;
  }
  recorder.events().await
}

#[tokio::test]
async fn graph_routes_are_absent_without_an_injected_graph_service() {
  let response = app_router(AppState::new(service()))
    .oneshot(
      Request::get("/v1/graph?root_kind=sense&root_id=hot")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::NOT_FOUND);
  assert_eq!(json(response).await["code"], "not_found");
}

#[tokio::test]
async fn graph_read_returns_typed_evidence_backed_topology_and_accessible_relation_list() {
  let response = graph_app()
    .oneshot(
      Request::get(
        "/v1/graph?root_kind=sense&root_id=hot&depth=1&node_limit=4&edge_limit=3&relation_types=hypernym",
      )
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  let body = json(response).await;
  assert_eq!(body["schema_version"], "1.0");
  assert_eq!(
    body["root"],
    serde_json::json!({"kind": "sense", "id": "hot"})
  );
  assert_eq!(
    body["content_version"],
    serde_json::json!({
      "release_id": "release-2026-09",
      "ranking_version": "graph-rank-v1",
      "community_aggregate_version": "community-v1"
    })
  );
  assert_eq!(body["edges"].as_array().unwrap().len(), 3);
  assert_eq!(body["edges"][0]["id"], "edge-high");
  assert_eq!(body["edges"][0]["source"], body["root"]);
  assert_eq!(
    body["edges"][0]["target"],
    serde_json::json!({"kind": "sense", "id": "warm"})
  );
  assert_eq!(body["edges"][0]["relation_type"], "hypernym");
  assert_eq!(
    body["edges"][0]["direction"],
    serde_json::json!({
      "directed": true,
      "canonical_projection": {"kind": "stored"}
    })
  );
  assert_eq!(body["edges"][0]["relation_version"], 3);
  assert_eq!(
    body["edges"][0]["evidence"],
    serde_json::json!({"evidence_ids": ["evidence-heat"], "confidence": "high"})
  );
  assert_eq!(body["edges"][0]["scope"]["dialect"], "en-US");
  assert_eq!(body["edges"][0]["scope"]["domain"], "weather");
  assert_eq!(
    body["edges"][0]["ranking"]["display_rank_basis_points"],
    9_900
  );
  assert_eq!(
    body["edges"][0]["ranking"]["components"]["evidence_basis_points"],
    9_000
  );
  assert_eq!(body["edges"][0]["ranking"]["version"], "graph-rank-v1");
  assert_eq!(
    body["edges"][0]["feedback_capabilities"],
    serde_json::json!(["usefulness", "accuracy"])
  );
  assert_eq!(body["relation_list"].as_array().unwrap().len(), 3);
  assert_eq!(body["relation_list"][0]["edge_id"], "edge-high");
  assert_eq!(body["relation_list"][0]["source"]["label"], "hot");
  assert_eq!(body["relation_list"][0]["target"]["label"], "warm");
  assert_eq!(body["relation_list"][0]["relation_type"], "hypernym");
  assert_eq!(body["truncated"], false);
  assert!(body["next_cursor"].is_null());
}

#[tokio::test]
async fn graph_reads_record_closed_full_and_neighbor_outcomes_without_identifier_labels() {
  let recorder = InMemoryMetricsRecorder::new();
  let router = graph_app_with_metrics(&recorder);
  let request_id = "graph-request-id-secret-8172";

  let full = router
    .clone()
    .oneshot(
      Request::get(
        "/v1/graph?root_kind=sense&root_id=hot&depth=1&node_limit=4&edge_limit=3&relation_types=hypernym",
      )
      .header("x-request-id", request_id)
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(full.status(), StatusCode::OK);
  assert_eq!(
    recorded_events(&recorder, 1).await,
    vec![MetricEvent::GraphOperation {
      operation: GraphOperation::Traversal,
      outcome: MetricOutcome::Succeeded,
    }]
  );

  let neighbors = router
    .oneshot(
      Request::get("/v1/graph/nodes/sense/hot/neighbors?node_limit=2&edge_limit=1")
        .header("x-request-id", request_id)
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(neighbors.status(), StatusCode::OK);
  let events = recorded_events(&recorder, 2).await;
  assert_eq!(
    events,
    vec![
      MetricEvent::GraphOperation {
        operation: GraphOperation::Traversal,
        outcome: MetricOutcome::Succeeded,
      },
      MetricEvent::GraphOperation {
        operation: GraphOperation::NeighborExpansion,
        outcome: MetricOutcome::Degraded,
      },
    ]
  );
  let rendered = format!("{events:?}");
  assert!(!rendered.contains(request_id));
  assert!(!rendered.contains("hot"));
  for event in events {
    for label in event.attributes().labels() {
      assert!(matches!(label.key(), "operation" | "outcome"));
      assert!(matches!(
        label.value(),
        "traversal" | "neighbor_expansion" | "succeeded" | "degraded"
      ));
    }
  }
}

#[tokio::test]
async fn graph_handler_validation_rejections_record_static_operation_metrics() {
  let recorder = InMemoryMetricsRecorder::new();
  let router = graph_app_with_metrics(&recorder);

  for (uri, status, operation) in [
    (
      "/v1/graph?root_kind=sense&root_id=hot&unexpected=value",
      StatusCode::BAD_REQUEST,
      GraphOperation::Traversal,
    ),
    (
      "/v1/graph?root_kind=sense&root_id=hot&node_limit=76",
      StatusCode::UNPROCESSABLE_ENTITY,
      GraphOperation::Traversal,
    ),
    (
      "/v1/graph/nodes/sense/hot/neighbors?unexpected=value",
      StatusCode::BAD_REQUEST,
      GraphOperation::NeighborExpansion,
    ),
    (
      "/v1/graph/nodes/sense/hot/neighbors?node_limit=1",
      StatusCode::UNPROCESSABLE_ENTITY,
      GraphOperation::NeighborExpansion,
    ),
    (
      "/v1/graph/nodes/%FF/hot/neighbors",
      StatusCode::BAD_REQUEST,
      GraphOperation::NeighborExpansion,
    ),
  ] {
    recorder.clear().await;

    let response = router
      .clone()
      .oneshot(Request::get(uri).body(Body::empty()).unwrap())
      .await
      .unwrap();

    assert_eq!(response.status(), status, "uri: {uri}");
    let _ = recorded_events(&recorder, 1).await;
    assert_eq!(
      settled_events(&recorder).await,
      vec![MetricEvent::GraphOperation {
        operation,
        outcome: MetricOutcome::Rejected,
      }],
      "uri: {uri}"
    );
  }
}

#[tokio::test]
async fn graph_service_validation_records_one_rejection_without_handler_duplication() {
  let recorder = InMemoryMetricsRecorder::new();
  let router = graph_app_with_metrics(&recorder);
  let first = router
    .clone()
    .oneshot(
      Request::get(
        "/v1/graph/nodes/sense/hot/neighbors?node_limit=2&edge_limit=1&relation_types=hypernym",
      )
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(first.status(), StatusCode::OK);
  let cursor = json(first).await["next_cursor"]
    .as_str()
    .unwrap()
    .to_string();
  let _ = recorded_events(&recorder, 1).await;
  assert_eq!(settled_events(&recorder).await.len(), 1);

  recorder.clear().await;

  let response = router
    .oneshot(
      Request::get(format!(
        "/v1/graph/nodes/sense/hot/neighbors?node_limit=2&edge_limit=1&relation_types=hyponym&cursor={cursor}"
      ))
      .body(Body::empty())
      .unwrap(),
    )
  .await
  .unwrap();

  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  let _ = recorded_events(&recorder, 1).await;
  assert_eq!(
    settled_events(&recorder).await,
    vec![MetricEvent::GraphOperation {
      operation: GraphOperation::NeighborExpansion,
      outcome: MetricOutcome::Rejected,
    }]
  );
}

#[tokio::test]
async fn graph_read_exposes_inverse_projection_without_rewriting_canonical_identity() {
  let response = graph_app()
    .oneshot(
      Request::get(
        "/v1/graph?root_kind=sense&root_id=warm&depth=1&node_limit=2&edge_limit=1&relation_types=hyponym",
      )
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  let body = json(response).await;
  assert_eq!(body["edges"][0]["id"], "edge-high");
  assert_eq!(
    body["edges"][0]["source"],
    serde_json::json!({"kind": "sense", "id": "warm"})
  );
  assert_eq!(
    body["edges"][0]["target"],
    serde_json::json!({"kind": "sense", "id": "hot"})
  );
  assert_eq!(body["edges"][0]["relation_type"], "hyponym");
  assert_eq!(
    body["edges"][0]["direction"]["canonical_projection"],
    serde_json::json!({"kind": "inverse_projection"})
  );
}

#[tokio::test]
async fn neighbor_pages_use_confidential_integrity_protected_cursors_and_reject_tampering() {
  let router = graph_app();
  let first = router
    .clone()
    .oneshot(
      Request::get("/v1/graph/nodes/sense/hot/neighbors?node_limit=2")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(first.status(), StatusCode::OK);
  let first = json(first).await;
  assert_eq!(first["edges"][0]["id"], "edge-high");
  assert_eq!(first["relation_list"][0]["edge_id"], "edge-high");
  assert_eq!(first["truncated"], true);
  let cursor = first["next_cursor"].as_str().unwrap().to_string();
  assert!(cursor.starts_with("g2."));
  assert!(!cursor.contains("edge-high"));
  assert!(!cursor.contains("release-2026-09"));

  let repeated_first = router
    .clone()
    .oneshot(
      Request::get("/v1/graph/nodes/sense/hot/neighbors?node_limit=2")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(repeated_first.status(), StatusCode::OK);
  let repeated_cursor = json(repeated_first).await["next_cursor"]
    .as_str()
    .unwrap()
    .to_string();
  assert_ne!(cursor, repeated_cursor);

  let second = router
    .clone()
    .oneshot(
      Request::get(format!(
        "/v1/graph/nodes/sense/hot/neighbors?node_limit=2&cursor={cursor}"
      ))
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(second.status(), StatusCode::OK);
  let second = json(second).await;
  assert_eq!(second["edges"][0]["id"], "edge-mid");

  let tampered = format!("{cursor}x");
  let response = router
    .oneshot(
      Request::get(format!(
        "/v1/graph/nodes/sense/hot/neighbors?node_limit=2&cursor={tampered}"
      ))
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  let body = json(response).await;
  assert_eq!(body["code"], "invalid_graph_request");
  assert_eq!(body["errors"][0]["field"], "cursor");
  assert!(!body.to_string().contains(&tampered));
}

#[tokio::test]
async fn neighbor_cursor_survives_a_graph_replica_when_the_protection_key_is_shared() {
  let shared_key = GraphCursorProtectionKey::new([7_u8; 32]).unwrap();
  let first_replica = graph_app_with_cursor_key(shared_key.clone());
  let second_replica = graph_app_with_cursor_key(shared_key);
  let different_replica =
    graph_app_with_cursor_key(GraphCursorProtectionKey::new([8_u8; 32]).unwrap());
  let first = first_replica
    .oneshot(
      Request::get("/v1/graph/nodes/sense/hot/neighbors?node_limit=2&edge_limit=1")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(first.status(), StatusCode::OK);
  let cursor = json(first).await["next_cursor"]
    .as_str()
    .unwrap()
    .to_string();

  let resumed = second_replica
    .oneshot(
      Request::get(format!(
        "/v1/graph/nodes/sense/hot/neighbors?node_limit=2&edge_limit=1&cursor={cursor}"
      ))
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(resumed.status(), StatusCode::OK);
  assert_eq!(json(resumed).await["edges"][0]["id"], "edge-mid");

  let rejected = different_replica
    .oneshot(
      Request::get(format!(
        "/v1/graph/nodes/sense/hot/neighbors?node_limit=2&edge_limit=1&cursor={cursor}"
      ))
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);
  let rejected = json(rejected).await;
  assert_eq!(rejected["errors"][0]["field"], "cursor");
  assert!(!rejected.to_string().contains(&cursor));
}

#[tokio::test]
async fn neighbor_cursor_hides_withheld_endpoint_identifiers_and_resumes_across_replicas() {
  let graph = missing_endpoint_graph_service();
  let shared_key = GraphCursorProtectionKey::new([9_u8; 32]).unwrap();
  let first_replica = graph_app_with_service_and_cursor_key(graph.clone(), shared_key.clone());
  let second_replica = graph_app_with_service_and_cursor_key(graph, shared_key);
  let first = first_replica
    .oneshot(
      Request::get("/v1/graph/nodes/sense/hot/neighbors?node_limit=2&edge_limit=1")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(first.status(), StatusCode::OK);
  let first = json(first).await;
  assert!(first["edges"].as_array().unwrap().is_empty());
  assert!(first["truncated"].as_bool().unwrap());
  assert!(!first.to_string().contains("withheld-target"));
  assert!(!first.to_string().contains("edge-withheld"));
  let cursor = first["next_cursor"].as_str().unwrap().to_string();
  let mut parts = cursor.split('.');
  assert_eq!(parts.next(), Some("g2"));
  let nonce = URL_SAFE_NO_PAD.decode(parts.next().unwrap()).unwrap();
  let ciphertext = URL_SAFE_NO_PAD.decode(parts.next().unwrap()).unwrap();
  assert!(parts.next().is_none());
  for encoded_part in [&nonce[..], &ciphertext[..]] {
    let exposed = String::from_utf8_lossy(encoded_part);
    assert!(!exposed.contains("withheld-target"));
    assert!(!exposed.contains("edge-withheld"));
  }
  assert!(serde_json::from_slice::<Value>(&ciphertext).is_err());

  let terminal = second_replica
    .oneshot(
      Request::get(format!(
        "/v1/graph/nodes/sense/hot/neighbors?node_limit=2&edge_limit=1&cursor={cursor}"
      ))
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(terminal.status(), StatusCode::OK);
  let terminal = json(terminal).await;
  assert!(terminal["edges"].as_array().unwrap().is_empty());
  assert!(terminal["next_cursor"].is_null());
}

#[tokio::test]
async fn neighbor_cursor_rejects_a_changed_normalized_relation_filter() {
  let router = graph_app();
  let first = router
    .clone()
    .oneshot(
      Request::get(
        "/v1/graph/nodes/sense/hot/neighbors?node_limit=2&edge_limit=1&relation_types=hypernym",
      )
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(first.status(), StatusCode::OK);
  let cursor = json(first).await["next_cursor"]
    .as_str()
    .unwrap()
    .to_string();

  let response = router
    .oneshot(
      Request::get(format!(
        "/v1/graph/nodes/sense/hot/neighbors?node_limit=2&edge_limit=1&relation_types=hyponym&cursor={cursor}"
      ))
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  let body = json(response).await;
  assert_eq!(body["code"], "invalid_graph_request");
  assert_eq!(body["errors"][0]["field"], "cursor");
  assert!(!body.to_string().contains("hyponym"));
}

#[tokio::test]
async fn graph_validation_uses_redacted_v1_problem_details() {
  let raw_id = "unsafe-input-id";
  let response = graph_app()
    .oneshot(
      Request::get(format!(
        "/v1/graph?root_kind=sense&root_id={raw_id}&node_limit=76"
      ))
      .header("x-request-id", "graph-validation-1")
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  assert_eq!(
    response.headers()[header::CONTENT_TYPE],
    "application/problem+json"
  );
  let body = json(response).await;
  assert_eq!(body["code"], "invalid_graph_request");
  assert_eq!(body["errors"][0]["field"], "node_limit");
  assert_eq!(body["request_id"], "graph-validation-1");
  assert!(!body.to_string().contains(raw_id));

  let response = graph_app()
    .oneshot(
      Request::get("/v1/graph/nodes/sense/hot/neighbors?node_limit=1")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  let body = json(response).await;
  assert_eq!(body["errors"][0]["field"], "node_limit");
}

#[tokio::test]
async fn graph_repository_failure_uses_a_redacted_retryable_problem() {
  let graph = Arc::new(GraphService::new(Arc::new(UnavailableGraphRepository)));
  let raw_id = "unavailable-root";
  let recorder = InMemoryMetricsRecorder::new();
  let response = app_router(
    AppState::new(service())
      .with_graph_service(graph)
      .with_metrics_recorder(Arc::new(recorder.clone())),
  )
  .oneshot(
    Request::get(format!("/v1/graph?root_kind=sense&root_id={raw_id}"))
      .body(Body::empty())
      .unwrap(),
  )
  .await
  .unwrap();

  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  let body = json(response).await;
  assert_eq!(body["code"], "graph_unavailable");
  assert_eq!(body["retryable"], true);
  assert!(!body.to_string().contains(raw_id));
  let events = recorded_events(&recorder, 1).await;
  assert_eq!(
    events,
    vec![MetricEvent::GraphOperation {
      operation: GraphOperation::Traversal,
      outcome: MetricOutcome::Failed,
    }]
  );
  assert!(!format!("{events:?}").contains(raw_id));
}
