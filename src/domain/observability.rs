//! Closed, content-free events and metrics for backend observability.
//!
//! This catalog names the operational signals required by the backend foundation without defining
//! an exporter, trace backend, retention policy, or alert thresholds. Metric events carry only
//! fixed enum dimensions. They cannot carry raw queries, contexts, answers, credentials or other
//! tokens, identities, or free-form attribute keys and values.

use std::fmt;

use time::OffsetDateTime;

/// Version of the structured observability event envelope.
pub const EVENT_SCHEMA_VERSION: &str = "transnet-observability-event-v1";

/// Service name emitted by every Transnet event.
pub const SERVICE_NAME: &str = "transnet";

/// A deployment environment with bounded cardinality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeploymentEnvironment {
  /// Developer workstation or local integration environment.
  Development,
  /// Automated test environment.
  Test,
  /// Pre-production environment.
  Staging,
  /// Production environment.
  Production,
}

/// A closed event severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventSeverity {
  /// Informational lifecycle observation.
  Info,
  /// Recoverable degradation or rejected operation.
  Warning,
  /// Operation failure requiring diagnosis.
  Error,
}

/// Static event names accepted by the structured envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventName {
  /// An admitted HTTP operation completed.
  RequestCompleted,
  /// One logical dependency call completed.
  DependencyCompleted,
  /// Telemetry delivery was dropped.
  TelemetryDropped,
  /// The process began graceful shutdown.
  ShutdownStarted,
}

/// Static route templates safe for events and metric labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StaticRoute {
  /// A route not represented by the current closed catalog.
  Unmatched,
  /// Transitional health probe.
  Health,
  /// Transitional liveness probe.
  Livez,
  /// Transitional readiness probe.
  Readyz,
  /// Target translation operation.
  Translations,
  /// Target BasicCard lookup operation.
  BasicCardLookup,
  /// Target canonical sense read operation.
  SenseRead,
  /// Transitional graph node read.
  GraphNodeRead,
  /// Transitional graph-neighbor read.
  GraphNeighbors,
}

/// A closed dependency dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyKind {
  /// No dependency participated.
  None,
  /// Generation model runtime.
  Model,
  /// Canonical structured-data authority.
  CanonicalData,
  /// Retrieval projection authority.
  RetrievalData,
  /// Ephemeral embedding runtime.
  Embedding,
  /// Bounded live-retrieval boundary.
  LiveRetrieval,
}

/// A bounded reason why best-effort telemetry was dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TelemetryDropReason {
  /// The bounded in-flight capacity was exhausted.
  Capacity,
  /// No asynchronous runtime was available for non-blocking delivery.
  RuntimeUnavailable,
}

impl TelemetryDropReason {
  const fn as_label(self) -> &'static str {
    match self {
      Self::Capacity => "capacity",
      Self::RuntimeUnavailable => "runtime_unavailable",
    }
  }
}

/// Validated W3C trace context admitted from an internal HTTP boundary.
///
/// Debug output intentionally identifies only that valid context is present. Callers can obtain
/// the normalized header value solely for propagation to another trusted internal hop.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceParent {
  normalized: String,
}

impl TraceParent {
  /// Parses the strict W3C `traceparent` version-00 wire shape.
  ///
  /// # Errors
  ///
  /// Returns [`TraceParentError`] for unsupported versions, malformed hexadecimal fields, all-zero
  /// trace or parent identifiers, or noncanonical lengths.
  pub fn parse(value: &str) -> Result<Self, TraceParentError> {
    let bytes = value.as_bytes();
    if bytes.len() != 55
      || !bytes.iter().all(u8::is_ascii)
      || bytes[2] != b'-'
      || bytes[35] != b'-'
      || bytes[52] != b'-'
    {
      return Err(TraceParentError::Malformed);
    }
    if &value[0..2] != "00" {
      return Err(TraceParentError::UnsupportedVersion);
    }
    let trace_id = &value[3..35];
    let parent_id = &value[36..52];
    let flags = &value[53..55];
    if ![trace_id, parent_id, flags]
      .into_iter()
      .all(|field| field.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
      return Err(TraceParentError::Malformed);
    }
    if trace_id.bytes().all(|byte| byte == b'0') || parent_id.bytes().all(|byte| byte == b'0') {
      return Err(TraceParentError::ZeroIdentifier);
    }
    Ok(Self {
      normalized: value.to_ascii_lowercase(),
    })
  }

  /// Returns the normalized header value for trusted internal propagation.
  pub fn as_header_value(&self) -> &str {
    &self.normalized
  }
}

impl fmt::Debug for TraceParent {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("TraceParent(VALIDATED)")
  }
}

/// Failure to validate an inbound W3C trace parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TraceParentError {
  /// The header does not have the canonical four-field version-00 shape.
  #[error("traceparent is malformed")]
  Malformed,
  /// Only W3C trace-context version 00 is currently admitted.
  #[error("traceparent version is unsupported")]
  UnsupportedVersion,
  /// W3C forbids all-zero trace and parent identifiers.
  #[error("traceparent contains an all-zero identifier")]
  ZeroIdentifier,
}

/// A versioned structured event containing only closed, content-free dimensions.
///
/// The type has no free-form message, attribute map, request body, URL, credential, or identifier
/// field. This makes ordinary construction content-free by design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservabilityEvent {
  timestamp: OffsetDateTime,
  severity: EventSeverity,
  environment: DeploymentEnvironment,
  event_name: EventName,
  route: StaticRoute,
  outcome: MetricOutcome,
  dependency: DependencyKind,
}

impl ObservabilityEvent {
  /// Creates one content-free event at a caller-supplied UTC timestamp.
  pub const fn new(
    timestamp: OffsetDateTime,
    severity: EventSeverity,
    environment: DeploymentEnvironment,
    event_name: EventName,
    route: StaticRoute,
    outcome: MetricOutcome,
    dependency: DependencyKind,
  ) -> Self {
    Self {
      timestamp,
      severity,
      environment,
      event_name,
      route,
      outcome,
      dependency,
    }
  }

  /// Returns the fixed envelope schema version.
  pub const fn event_schema(&self) -> &'static str {
    EVENT_SCHEMA_VERSION
  }

  /// Returns the fixed emitting service name.
  pub const fn service(&self) -> &'static str {
    SERVICE_NAME
  }

  /// Returns the package version of the emitting service.
  pub const fn service_version(&self) -> &'static str {
    env!("CARGO_PKG_VERSION")
  }

  /// Returns the event timestamp.
  pub const fn timestamp(&self) -> OffsetDateTime {
    self.timestamp
  }

  /// Returns the bounded severity.
  pub const fn severity(&self) -> EventSeverity {
    self.severity
  }

  /// Returns the bounded deployment environment.
  pub const fn environment(&self) -> DeploymentEnvironment {
    self.environment
  }

  /// Returns the static event name.
  pub const fn event_name(&self) -> EventName {
    self.event_name
  }

  /// Returns the static matched route.
  pub const fn route(&self) -> StaticRoute {
    self.route
  }

  /// Returns the closed operation outcome.
  pub const fn outcome(&self) -> MetricOutcome {
    self.outcome
  }

  /// Returns the closed dependency dimension.
  pub const fn dependency(&self) -> DependencyKind {
    self.dependency
  }
}

/// Stable metric names for backend operational signals.
///
/// Names are intentionally exporter-neutral and use Prometheus-compatible identifiers. Counter
/// and state names are separated so an adapter cannot mistake a categorical state for a numeric
/// measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MetricName {
  /// Count of bounded lookup-stage outcomes.
  LookupStageTotal,
  /// Count of learning-model contract-validation outcomes.
  ModelValidationTotal,
  /// Count of categorical vector-reconciliation lag observations.
  VectorLagStateTotal,
  /// Count of graph-read operation outcomes.
  GraphOperationTotal,
  /// Count of best-effort telemetry records dropped before delivery.
  TelemetryDroppedTotal,
}

impl MetricName {
  /// Returns the fixed exporter-facing metric identifier.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::LookupStageTotal => "transnet_lookup_stage_total",
      Self::ModelValidationTotal => "transnet_model_validation_total",
      Self::VectorLagStateTotal => "transnet_vector_lag_state_total",
      Self::GraphOperationTotal => "transnet_graph_operation_total",
      Self::TelemetryDroppedTotal => "transnet_telemetry_dropped_total",
    }
  }
}

/// A bounded stage in a lookup attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LookupStage {
  /// Request parsing and bounded input validation.
  RequestValidation,
  /// Selection of the immutable canonical content version.
  ContentResolution,
  /// Canonical lexical and vector candidate retrieval.
  CandidateRetrieval,
  /// Public-card or response assembly from retrieved candidates.
  ResponseAssembly,
}

impl LookupStage {
  const fn as_label(self) -> &'static str {
    match self {
      Self::RequestValidation => "request_validation",
      Self::ContentResolution => "content_resolution",
      Self::CandidateRetrieval => "candidate_retrieval",
      Self::ResponseAssembly => "response_assembly",
    }
  }
}

/// A safe categorical outcome shared by bounded backend operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MetricOutcome {
  /// The operation completed with its normal contract.
  Succeeded,
  /// The operation returned a documented reduced-capability result.
  Degraded,
  /// Validation or policy rejected the operation before it completed.
  Rejected,
  /// A dependency or internal failure prevented completion.
  Failed,
}

impl MetricOutcome {
  const fn as_label(self) -> &'static str {
    match self {
      Self::Succeeded => "succeeded",
      Self::Degraded => "degraded",
      Self::Rejected => "rejected",
      Self::Failed => "failed",
    }
  }
}

/// A bounded result of validating a structured lexical-model response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModelValidationOutcome {
  /// The first model response satisfied the typed contract.
  Accepted,
  /// A bounded repair path produced a valid typed response.
  Repaired,
  /// The response remained invalid after permitted validation and repair work.
  Rejected,
}

impl ModelValidationOutcome {
  const fn as_label(self) -> &'static str {
    match self {
      Self::Accepted => "accepted",
      Self::Repaired => "repaired",
      Self::Rejected => "rejected",
    }
  }
}

/// A bounded categorical bucket for vector-index reconciliation lag.
///
/// The catalog deliberately uses a bucket rather than a raw release identifier, collection
/// identifier, query, or embedding payload. An exporter may map these categories to its own safe
/// numeric monitoring model later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VectorLagBucket {
  /// The active vector collection is reconciled with the active content release.
  InSync,
  /// Reconciliation is behind but within a short operational window.
  UnderFiveMinutes,
  /// Reconciliation is behind beyond the short window but below an hour.
  UnderOneHour,
  /// Reconciliation is behind by at least an hour.
  OneHourOrMore,
  /// Reconciliation could not be assessed or failed safely.
  Unavailable,
}

impl VectorLagBucket {
  const fn as_label(self) -> &'static str {
    match self {
      Self::InSync => "in_sync",
      Self::UnderFiveMinutes => "under_five_minutes",
      Self::UnderOneHour => "under_one_hour",
      Self::OneHourOrMore => "one_hour_or_more",
      Self::Unavailable => "unavailable",
    }
  }
}

/// A read-only graph operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphOperation {
  /// Read one graph node by its public identifier.
  NodeRead,
  /// Read bounded neighboring edges and nodes.
  NeighborExpansion,
  /// Perform a bounded breadth-first traversal.
  Traversal,
}

impl GraphOperation {
  const fn as_label(self) -> &'static str {
    match self {
      Self::NodeRead => "node_read",
      Self::NeighborExpansion => "neighbor_expansion",
      Self::Traversal => "traversal",
    }
  }
}

/// One closed metric event accepted by the metrics port.
///
/// Every variant contains only typed categorical values. There is deliberately no generic
/// constructor, label map, `String`, byte payload, number, owner, request, answer, or token field.
/// This prevents callers from attaching private text or identity data to ordinary telemetry.
///
/// ```compile_fail
/// use transnet::domain::observability::MetricEvent;
///
/// let _ = MetricEvent::LookupStage {
///   stage: "private query".to_owned(),
///   outcome: "private answer".to_owned(),
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MetricEvent {
  /// One bounded lookup-stage outcome.
  LookupStage {
    /// Stage completed or attempted.
    stage: LookupStage,
    /// Safe categorical result.
    outcome: MetricOutcome,
  },
  /// One structured lexical-model validation outcome.
  ModelValidation {
    /// Safe categorical validation result.
    outcome: ModelValidationOutcome,
  },
  /// One vector-release reconciliation lag bucket.
  VectorLag {
    /// Safe lag category rather than a raw release or collection identifier.
    bucket: VectorLagBucket,
  },
  /// One bounded graph-read outcome.
  GraphOperation {
    /// Read-only graph operation.
    operation: GraphOperation,
    /// Safe categorical result.
    outcome: MetricOutcome,
  },
  /// One best-effort telemetry record was dropped.
  TelemetryDropped {
    /// Bounded reason for the drop.
    reason: TelemetryDropReason,
  },
}

impl MetricEvent {
  /// Returns the metric series to which this event belongs.
  pub const fn metric_name(self) -> MetricName {
    match self {
      Self::LookupStage { .. } => MetricName::LookupStageTotal,
      Self::ModelValidation { .. } => MetricName::ModelValidationTotal,
      Self::VectorLag { .. } => MetricName::VectorLagStateTotal,
      Self::GraphOperation { .. } => MetricName::GraphOperationTotal,
      Self::TelemetryDropped { .. } => MetricName::TelemetryDroppedTotal,
    }
  }

  /// Returns only fixed metric labels for this event.
  ///
  /// The returned value has no public constructor and its labels have private fields, so callers
  /// cannot add free-form query, context, answer, token, identity, or credential values.
  pub fn attributes(self) -> MetricAttributes {
    let labels = match self {
      Self::LookupStage { stage, outcome } => vec![
        MetricLabel::new("stage", stage.as_label()),
        MetricLabel::new("outcome", outcome.as_label()),
      ],
      Self::ModelValidation { outcome } => {
        vec![MetricLabel::new("outcome", outcome.as_label())]
      }
      Self::VectorLag { bucket } => {
        vec![MetricLabel::new("lag_bucket", bucket.as_label())]
      }
      Self::GraphOperation { operation, outcome } => vec![
        MetricLabel::new("operation", operation.as_label()),
        MetricLabel::new("outcome", outcome.as_label()),
      ],
      Self::TelemetryDropped { reason } => {
        vec![MetricLabel::new("reason", reason.as_label())]
      }
    };
    MetricAttributes { labels }
  }
}

/// Fixed labels emitted with one metric event.
///
/// The label list can only be created by [`MetricEvent::attributes`]. Its values are static enum
/// labels, never caller-provided data.
///
/// ```compile_fail
/// use transnet::domain::observability::MetricAttributes;
///
/// let _ = MetricAttributes {
///   labels: vec![],
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricAttributes {
  labels: Vec<MetricLabel>,
}

impl MetricAttributes {
  /// Returns the fixed label pairs for exporter adaptation or deterministic inspection.
  pub fn labels(&self) -> &[MetricLabel] {
    &self.labels
  }
}

/// One fixed metric label pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MetricLabel {
  key: &'static str,
  value: &'static str,
}

impl MetricLabel {
  const fn new(key: &'static str, value: &'static str) -> Self {
    Self { key, value }
  }

  /// Returns the fixed label key.
  pub const fn key(self) -> &'static str {
    self.key
  }

  /// Returns the fixed categorical label value.
  pub const fn value(self) -> &'static str {
    self.value
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn all_metric_events() -> Vec<MetricEvent> {
    vec![
      MetricEvent::LookupStage {
        stage: LookupStage::RequestValidation,
        outcome: MetricOutcome::Succeeded,
      },
      MetricEvent::LookupStage {
        stage: LookupStage::ContentResolution,
        outcome: MetricOutcome::Degraded,
      },
      MetricEvent::LookupStage {
        stage: LookupStage::CandidateRetrieval,
        outcome: MetricOutcome::Rejected,
      },
      MetricEvent::LookupStage {
        stage: LookupStage::ResponseAssembly,
        outcome: MetricOutcome::Failed,
      },
      MetricEvent::ModelValidation {
        outcome: ModelValidationOutcome::Accepted,
      },
      MetricEvent::ModelValidation {
        outcome: ModelValidationOutcome::Repaired,
      },
      MetricEvent::ModelValidation {
        outcome: ModelValidationOutcome::Rejected,
      },
      MetricEvent::VectorLag {
        bucket: VectorLagBucket::InSync,
      },
      MetricEvent::VectorLag {
        bucket: VectorLagBucket::UnderFiveMinutes,
      },
      MetricEvent::VectorLag {
        bucket: VectorLagBucket::UnderOneHour,
      },
      MetricEvent::VectorLag {
        bucket: VectorLagBucket::OneHourOrMore,
      },
      MetricEvent::VectorLag {
        bucket: VectorLagBucket::Unavailable,
      },
      MetricEvent::GraphOperation {
        operation: GraphOperation::NodeRead,
        outcome: MetricOutcome::Succeeded,
      },
      MetricEvent::GraphOperation {
        operation: GraphOperation::NeighborExpansion,
        outcome: MetricOutcome::Degraded,
      },
      MetricEvent::GraphOperation {
        operation: GraphOperation::Traversal,
        outcome: MetricOutcome::Failed,
      },
      MetricEvent::TelemetryDropped {
        reason: TelemetryDropReason::Capacity,
      },
      MetricEvent::TelemetryDropped {
        reason: TelemetryDropReason::RuntimeUnavailable,
      },
    ]
  }

  #[test]
  fn covers_every_required_metric_series_with_stable_names() {
    let names = all_metric_events()
      .into_iter()
      .map(|event| event.metric_name().as_str())
      .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(
      names,
      std::collections::BTreeSet::from([
        "transnet_graph_operation_total",
        "transnet_lookup_stage_total",
        "transnet_model_validation_total",
        "transnet_telemetry_dropped_total",
        "transnet_vector_lag_state_total",
      ])
    );
  }

  #[test]
  fn emits_only_closed_static_labels_without_private_content() {
    let protected_values = [
      "private lookup query",
      "private surrounding context",
      "private learner answer",
      "private bearer token",
      "private learner identity",
    ];
    let protected_label_terms = ["query", "context", "answer", "token", "identity", "user"];

    let mut emitted = Vec::new();
    for event in all_metric_events() {
      emitted.push(("metric", event.metric_name().as_str()));
      let attributes = event.attributes();
      emitted.extend(
        attributes
          .labels()
          .iter()
          .map(|label| (label.key(), label.value())),
      );
    }

    assert!(emitted.iter().all(|(key, value)| {
      protected_values
        .iter()
        .all(|protected| !key.contains(protected) && !value.contains(protected))
    }));
    assert!(emitted.iter().all(|(key, _)| {
      protected_label_terms
        .iter()
        .all(|protected| !key.contains(protected))
    }));
    assert!(emitted.iter().all(|(_, value)| {
      protected_label_terms
        .iter()
        .all(|protected| !value.contains(protected))
    }));
  }

  #[test]
  fn debug_output_cannot_include_values_that_events_do_not_accept() {
    let protected_values = [
      "query=water",
      "context=private sentence",
      "answer=private response",
      "token=credential",
      "identity=learner-123",
    ];
    let output = all_metric_events()
      .into_iter()
      .map(|event| format!("{event:?}{:?}", event.attributes()))
      .collect::<String>();

    assert!(protected_values
      .iter()
      .all(|protected| !output.contains(protected)));
  }

  #[test]
  fn trace_parent_is_strict_normalized_and_debug_redacted() {
    let value = "00-4BF92F3577B34DA6A3CE929D0E0E4736-00F067AA0BA902B7-01";
    let parent = TraceParent::parse(value).unwrap();

    assert_eq!(
      parent.as_header_value(),
      "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
    );
    assert_eq!(format!("{parent:?}"), "TraceParent(VALIDATED)");
    assert!(TraceParent::parse("00-00000000000000000000000000000000-00f067aa0ba902b7-01").is_err());
    assert!(TraceParent::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01").is_err());
    assert!(TraceParent::parse("credential-secret").is_err());
    assert!(TraceParent::parse(&format!("{}é", "a".repeat(53))).is_err());
  }

  #[test]
  fn event_envelope_has_only_versioned_closed_dimensions() {
    let event = ObservabilityEvent::new(
      OffsetDateTime::UNIX_EPOCH,
      EventSeverity::Warning,
      DeploymentEnvironment::Test,
      EventName::DependencyCompleted,
      StaticRoute::Translations,
      MetricOutcome::Degraded,
      DependencyKind::Model,
    );
    let rendered = format!("{event:?}");

    assert_eq!(event.event_schema(), EVENT_SCHEMA_VERSION);
    assert_eq!(event.service(), SERVICE_NAME);
    assert_eq!(event.service_version(), env!("CARGO_PKG_VERSION"));
    for forbidden in [
      "request-body-secret",
      "bearer-credential-secret",
      "provider-response-secret",
      "https://citation-secret.invalid",
    ] {
      assert!(!rendered.contains(forbidden));
    }
  }
}
