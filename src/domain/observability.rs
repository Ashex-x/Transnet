//! Closed, content-free events and metrics for backend observability.
//!
//! This catalog names the operational signals required by the backend foundation without defining
//! an exporter, trace backend, retention policy, or alert thresholds. Metric events carry only
//! fixed enum dimensions. They cannot carry raw queries, contexts, answers, credentials or other
//! tokens, identities, or free-form attribute keys and values.

use std::{fmt, str::FromStr, time::Duration};

use time::OffsetDateTime;

/// Version of the structured observability event envelope.
pub const EVENT_SCHEMA_VERSION: &str = "transnet-observability-event-v1";

/// Service name emitted by every Transnet event.
pub const SERVICE_NAME: &str = "transnet";

/// Minimum bounded telemetry queue capacity accepted by the export contract.
pub const MIN_TELEMETRY_QUEUE_CAPACITY: usize = 1;

/// Maximum bounded telemetry queue capacity accepted by the export contract.
pub const MAX_TELEMETRY_QUEUE_CAPACITY: usize = 4_096;

/// Minimum exporter operation timeout accepted by the export contract.
pub const MIN_TELEMETRY_EXPORT_TIMEOUT: Duration = Duration::from_millis(1);

/// Maximum exporter operation timeout accepted by the export contract.
pub const MAX_TELEMETRY_EXPORT_TIMEOUT: Duration = Duration::from_secs(5);

/// Denominator used by deterministic telemetry sampling rates.
pub const TELEMETRY_SAMPLING_BASIS_POINTS: u16 = 10_000;

/// Closed selection of whether a production export boundary is installed.
///
/// `Configured` deliberately does not name a transport. Collector transport and framing remain a
/// deployment-owned contract that must be frozen before an adapter is implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryExportMode {
  /// No exporter is installed and no export queue is created.
  Disabled,
  /// A separately composed production exporter must consume this repository-owned contract.
  Configured,
}

impl FromStr for TelemetryExportMode {
  type Err = TelemetryExportModeParseError;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    match value {
      "disabled" => Ok(Self::Disabled),
      "configured" => Ok(Self::Configured),
      _ => Err(TelemetryExportModeParseError),
    }
  }
}

/// Closed failure to parse a telemetry export mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("telemetry export mode is invalid")]
pub struct TelemetryExportModeParseError;

/// A validated deterministic sampling rate in basis points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelemetrySamplingRate(u16);

impl TelemetrySamplingRate {
  /// Creates a rate from zero through 10,000 basis points inclusive.
  ///
  /// # Errors
  ///
  /// Returns a closed validation error when `basis_points` exceeds 10,000.
  pub const fn new(basis_points: u16) -> Result<Self, TelemetryExportValidationError> {
    if basis_points > TELEMETRY_SAMPLING_BASIS_POINTS {
      Err(TelemetryExportValidationError::InvalidSamplingRate)
    } else {
      Ok(Self(basis_points))
    }
  }

  /// Returns the validated rate in basis points.
  pub const fn basis_points(self) -> u16 {
    self.0
  }
}

/// Sampling policy for content-free structured events.
///
/// A future exporter must make a content-independent decision before bounded queue admission.
/// Metric records and telemetry-drop records are never sampled so counters and failure accounting
/// remain complete. The selection algorithm remains an exporter implementation contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelemetrySamplingPolicy {
  success: TelemetrySamplingRate,
  failure: TelemetrySamplingRate,
}

impl TelemetrySamplingPolicy {
  /// Creates separate deterministic rates for successful and non-successful structured events.
  pub const fn new(success: TelemetrySamplingRate, failure: TelemetrySamplingRate) -> Self {
    Self { success, failure }
  }

  /// Returns the successful-event sampling rate.
  pub const fn success(self) -> TelemetrySamplingRate {
    self.success
  }

  /// Returns the failure-event sampling rate.
  pub const fn failure(self) -> TelemetrySamplingRate {
    self.failure
  }

  /// Returns the sampling rate for a structured record.
  ///
  /// `None` means the record is required and must bypass sampling. The future exporter owns the
  /// deterministic, content-independent selection algorithm and must apply it before queue
  /// admission.
  pub const fn rate_for(self, record: &TelemetryExportRecord) -> Option<TelemetrySamplingRate> {
    match record.sampling_class() {
      TelemetrySamplingClass::Success => Some(self.success),
      TelemetrySamplingClass::Failure => Some(self.failure),
      TelemetrySamplingClass::Required => None,
    }
  }
}

/// Validated repository-owned settings for a configured exporter boundary.
///
/// This policy is not runtime configuration by itself. A later adapter PR must freeze collector
/// transport and then map strict configuration into this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelemetryExportPolicy {
  queue_capacity: usize,
  export_timeout: Duration,
  sampling: TelemetrySamplingPolicy,
}

impl TelemetryExportPolicy {
  /// Validates queue, timeout, and sampling bounds without selecting a collector transport.
  ///
  /// # Errors
  ///
  /// Returns a closed error when queue capacity is outside 1 through 4,096 or the timeout is
  /// outside 1 millisecond through 5 seconds.
  pub const fn new(
    queue_capacity: usize,
    export_timeout: Duration,
    sampling: TelemetrySamplingPolicy,
  ) -> Result<Self, TelemetryExportValidationError> {
    if queue_capacity < MIN_TELEMETRY_QUEUE_CAPACITY
      || queue_capacity > MAX_TELEMETRY_QUEUE_CAPACITY
    {
      return Err(TelemetryExportValidationError::InvalidQueueCapacity);
    }
    if export_timeout.as_nanos() < MIN_TELEMETRY_EXPORT_TIMEOUT.as_nanos()
      || export_timeout.as_nanos() > MAX_TELEMETRY_EXPORT_TIMEOUT.as_nanos()
    {
      return Err(TelemetryExportValidationError::InvalidExportTimeout);
    }
    Ok(Self {
      queue_capacity,
      export_timeout,
      sampling,
    })
  }

  /// Returns the maximum records admitted to the future bounded exporter queue.
  pub const fn queue_capacity(self) -> usize {
    self.queue_capacity
  }

  /// Returns the maximum duration of one future exporter operation.
  pub const fn export_timeout(self) -> Duration {
    self.export_timeout
  }

  /// Returns the structured-event sampling policy.
  pub const fn sampling(self) -> TelemetrySamplingPolicy {
    self.sampling
  }
}

/// Closed telemetry export policy validation failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TelemetryExportValidationError {
  /// Queue capacity was zero or exceeded the fixed in-memory bound.
  #[error("telemetry queue capacity is invalid")]
  InvalidQueueCapacity,
  /// Export timeout was zero or exceeded the best-effort isolation bound.
  #[error("telemetry export timeout is invalid")]
  InvalidExportTimeout,
  /// A sampling rate exceeded 10,000 basis points.
  #[error("telemetry sampling rate is invalid")]
  InvalidSamplingRate,
}

/// Closed exporter failures that must never change a business result or readiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TelemetryExportFailure {
  /// The configured exporter or collector could not accept work.
  #[error("telemetry exporter is unavailable")]
  Unavailable,
  /// One bounded export operation exceeded its timeout.
  #[error("telemetry export timed out")]
  Timeout,
  /// A logical content-free record could not be serialized by the future adapter.
  #[error("telemetry export serialization failed")]
  Serialization,
  /// The exporter was already shutting down and accepted no new work.
  #[error("telemetry exporter is shutting down")]
  ShuttingDown,
}

impl TelemetryExportFailure {
  /// Returns the closed drop reason recorded by an exporter after this failure.
  pub const fn drop_reason(self) -> TelemetryDropReason {
    match self {
      Self::Unavailable => TelemetryDropReason::ExporterUnavailable,
      Self::Timeout => TelemetryDropReason::ExportTimeout,
      Self::Serialization => TelemetryDropReason::Serialization,
      Self::ShuttingDown => TelemetryDropReason::ShuttingDown,
    }
  }
}

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
  /// Service capability discovery.
  Capabilities,
  /// Health probe.
  Health,
  /// Liveness probe.
  Livez,
  /// Readiness probe.
  Readyz,
  /// Target translation operation.
  Translations,
  /// Target streaming translation operation.
  TranslationsStream,
  /// Target BasicCard lookup operation.
  BasicCardLookup,
  /// Target canonical sense read operation.
  SenseRead,
  /// Guided knowledge-view read.
  KnowledgeViews,
  /// Evidence-backed knowledge-path read.
  KnowledgePaths,
  /// Compatibility identity for the removed transitional graph-node read.
  ///
  /// The current HTTP classifier never produces this value.
  GraphNodeRead,
  /// Compatibility identity for the removed transitional graph-neighbor read.
  ///
  /// The current HTTP classifier never produces this value.
  GraphNeighbors,
}

impl StaticRoute {
  /// Classifies one Axum matched-path template into the closed route catalog.
  ///
  /// Unknown templates fail closed to [`Self::Unmatched`]. Callers must never substitute a raw
  /// URI, path, or query when classification fails.
  pub fn from_matched_path(path: &str) -> Self {
    match path {
      "/api/v1/capabilities" => Self::Capabilities,
      "/api/v1/health" => Self::Health,
      "/api/v1/livez" => Self::Livez,
      "/api/v1/readyz" => Self::Readyz,
      "/api/v1/translations" => Self::Translations,
      "/api/v1/translations/stream" => Self::TranslationsStream,
      "/api/v1/basic-cards/lookup" => Self::BasicCardLookup,
      "/api/v1/senses/get" => Self::SenseRead,
      "/api/v1/knowledge/views" => Self::KnowledgeViews,
      "/api/v1/knowledge/paths" => Self::KnowledgePaths,
      _ => Self::Unmatched,
    }
  }

  /// Returns the stable low-cardinality route identity used by tracing.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Unmatched => "unmatched",
      Self::Capabilities => "/api/v1/capabilities",
      Self::Health => "/api/v1/health",
      Self::Livez => "/api/v1/livez",
      Self::Readyz => "/api/v1/readyz",
      Self::Translations => "/api/v1/translations",
      Self::TranslationsStream => "/api/v1/translations/stream",
      Self::BasicCardLookup => "/api/v1/basic-cards/lookup",
      Self::SenseRead => "/api/v1/senses/get",
      Self::KnowledgeViews => "/api/v1/knowledge/views",
      Self::KnowledgePaths => "/api/v1/knowledge/paths",
      Self::GraphNodeRead => "legacy.graph_node_read",
      Self::GraphNeighbors => "legacy.graph_neighbors",
    }
  }
}

impl fmt::Display for StaticRoute {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(self.as_str())
  }
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
  /// A structured event was excluded by the deterministic pre-queue sampling policy.
  SampledOut,
  /// The bounded in-flight capacity was exhausted.
  Capacity,
  /// No asynchronous runtime was available for non-blocking delivery.
  RuntimeUnavailable,
  /// The configured exporter or collector was unavailable.
  ExporterUnavailable,
  /// A bounded export operation exceeded its timeout.
  ExportTimeout,
  /// The future adapter could not serialize a logical content-free record.
  Serialization,
  /// Exporter shutdown rejected a new best-effort record.
  ShuttingDown,
}

impl TelemetryDropReason {
  const fn as_label(self) -> &'static str {
    match self {
      Self::SampledOut => "sampled_out",
      Self::Capacity => "capacity",
      Self::RuntimeUnavailable => "runtime_unavailable",
      Self::ExporterUnavailable => "exporter_unavailable",
      Self::ExportTimeout => "export_timeout",
      Self::Serialization => "serialization",
      Self::ShuttingDown => "shutting_down",
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

/// One content-free metric observation prepared for a future exporter adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricExportRecord {
  timestamp: OffsetDateTime,
  environment: DeploymentEnvironment,
  event: MetricEvent,
}

impl MetricExportRecord {
  /// Creates one timestamped metric record without accepting arbitrary labels or payloads.
  pub const fn new(
    timestamp: OffsetDateTime,
    environment: DeploymentEnvironment,
    event: MetricEvent,
  ) -> Self {
    Self {
      timestamp,
      environment,
      event,
    }
  }

  /// Returns the fixed logical envelope schema version.
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

  /// Returns the closed deployment environment.
  pub const fn environment(&self) -> DeploymentEnvironment {
    self.environment
  }

  /// Returns the closed metric observation.
  pub const fn event(&self) -> MetricEvent {
    self.event
  }
}

/// Logical content-free record accepted by a future telemetry exporter.
///
/// This enum freezes repository-owned data semantics without selecting collector transport,
/// network framing, authentication, batching, or retention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelemetryExportRecord {
  /// One closed structured operational event.
  Event(ObservabilityEvent),
  /// One closed metric observation.
  Metric(MetricExportRecord),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TelemetrySamplingClass {
  Success,
  Failure,
  Required,
}

impl TelemetryExportRecord {
  const fn sampling_class(&self) -> TelemetrySamplingClass {
    match self {
      Self::Metric(_) => TelemetrySamplingClass::Required,
      Self::Event(event) if matches!(event.event_name, EventName::TelemetryDropped) => {
        TelemetrySamplingClass::Required
      }
      Self::Event(event) if matches!(event.outcome, MetricOutcome::Succeeded) => {
        TelemetrySamplingClass::Success
      }
      Self::Event(_) => TelemetrySamplingClass::Failure,
    }
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

  fn sampling() -> TelemetrySamplingPolicy {
    TelemetrySamplingPolicy::new(
      TelemetrySamplingRate::new(2_500).unwrap(),
      TelemetrySamplingRate::new(10_000).unwrap(),
    )
  }

  fn structured_event(outcome: MetricOutcome, name: EventName) -> TelemetryExportRecord {
    TelemetryExportRecord::Event(ObservabilityEvent::new(
      OffsetDateTime::UNIX_EPOCH,
      EventSeverity::Info,
      DeploymentEnvironment::Test,
      name,
      StaticRoute::Translations,
      outcome,
      DependencyKind::Model,
    ))
  }

  #[test]
  fn export_contract_is_closed_and_strictly_bounded() {
    assert_eq!("disabled".parse(), Ok(TelemetryExportMode::Disabled));
    assert_eq!("configured".parse(), Ok(TelemetryExportMode::Configured));
    assert_eq!(
      "stdout".parse::<TelemetryExportMode>(),
      Err(TelemetryExportModeParseError)
    );

    let policy = TelemetryExportPolicy::new(
      MIN_TELEMETRY_QUEUE_CAPACITY,
      MIN_TELEMETRY_EXPORT_TIMEOUT,
      sampling(),
    )
    .unwrap();
    assert_eq!(policy.queue_capacity(), 1);
    assert_eq!(policy.export_timeout(), Duration::from_millis(1));
    assert_eq!(policy.sampling(), sampling());
    assert!(TelemetryExportPolicy::new(
      MAX_TELEMETRY_QUEUE_CAPACITY,
      MAX_TELEMETRY_EXPORT_TIMEOUT,
      sampling(),
    )
    .is_ok());
    assert_eq!(
      TelemetryExportPolicy::new(0, Duration::from_millis(1), sampling()),
      Err(TelemetryExportValidationError::InvalidQueueCapacity)
    );
    assert_eq!(
      TelemetryExportPolicy::new(4_097, Duration::from_millis(1), sampling()),
      Err(TelemetryExportValidationError::InvalidQueueCapacity)
    );
    assert_eq!(
      TelemetryExportPolicy::new(1, Duration::ZERO, sampling()),
      Err(TelemetryExportValidationError::InvalidExportTimeout)
    );
    assert_eq!(
      TelemetryExportPolicy::new(1, Duration::from_millis(5_001), sampling()),
      Err(TelemetryExportValidationError::InvalidExportTimeout)
    );
    assert_eq!(
      TelemetrySamplingRate::new(10_001),
      Err(TelemetryExportValidationError::InvalidSamplingRate)
    );
  }

  #[test]
  fn sampling_classification_never_samples_metrics_or_drops() {
    let success = structured_event(MetricOutcome::Succeeded, EventName::RequestCompleted);
    let failure = structured_event(MetricOutcome::Failed, EventName::DependencyCompleted);
    let dropped = structured_event(MetricOutcome::Failed, EventName::TelemetryDropped);
    let metric = TelemetryExportRecord::Metric(MetricExportRecord::new(
      OffsetDateTime::UNIX_EPOCH,
      DeploymentEnvironment::Test,
      MetricEvent::LookupStage {
        stage: LookupStage::RequestValidation,
        outcome: MetricOutcome::Succeeded,
      },
    ));

    assert_eq!(
      sampling().rate_for(&success),
      Some(TelemetrySamplingRate::new(2_500).unwrap())
    );
    assert_eq!(
      sampling().rate_for(&failure),
      Some(TelemetrySamplingRate::new(10_000).unwrap())
    );
    assert_eq!(sampling().rate_for(&dropped), None);
    assert_eq!(sampling().rate_for(&metric), None);

    let none = TelemetrySamplingPolicy::new(
      TelemetrySamplingRate::new(0).unwrap(),
      TelemetrySamplingRate::new(0).unwrap(),
    );
    assert_eq!(
      none.rate_for(&success),
      Some(TelemetrySamplingRate::new(0).unwrap())
    );
    assert_eq!(
      none.rate_for(&failure),
      Some(TelemetrySamplingRate::new(0).unwrap())
    );
    assert_eq!(none.rate_for(&dropped), None);
    assert_eq!(none.rate_for(&metric), None);
  }

  #[test]
  fn logical_export_records_expose_only_closed_fields() {
    let record = MetricExportRecord::new(
      OffsetDateTime::UNIX_EPOCH,
      DeploymentEnvironment::Production,
      MetricEvent::TelemetryDropped {
        reason: TelemetryDropReason::Serialization,
      },
    );
    assert_eq!(record.event_schema(), EVENT_SCHEMA_VERSION);
    assert_eq!(record.service(), SERVICE_NAME);
    assert_eq!(record.service_version(), env!("CARGO_PKG_VERSION"));
    assert_eq!(record.timestamp(), OffsetDateTime::UNIX_EPOCH);
    assert_eq!(record.environment(), DeploymentEnvironment::Production);
    assert_eq!(
      record.event(),
      MetricEvent::TelemetryDropped {
        reason: TelemetryDropReason::Serialization
      }
    );
  }

  #[test]
  fn exporter_failures_map_to_distinct_closed_drop_reasons() {
    assert_eq!(
      TelemetryExportFailure::Unavailable.drop_reason(),
      TelemetryDropReason::ExporterUnavailable
    );
    assert_eq!(
      TelemetryExportFailure::Timeout.drop_reason(),
      TelemetryDropReason::ExportTimeout
    );
    assert_eq!(
      TelemetryExportFailure::Serialization.drop_reason(),
      TelemetryDropReason::Serialization
    );
    assert_eq!(
      TelemetryExportFailure::ShuttingDown.drop_reason(),
      TelemetryDropReason::ShuttingDown
    );
  }

  #[test]
  fn current_http_routes_have_distinct_closed_identities() {
    let routes = [
      ("/api/v1/capabilities", StaticRoute::Capabilities),
      ("/api/v1/health", StaticRoute::Health),
      ("/api/v1/livez", StaticRoute::Livez),
      ("/api/v1/readyz", StaticRoute::Readyz),
      ("/api/v1/translations", StaticRoute::Translations),
      (
        "/api/v1/translations/stream",
        StaticRoute::TranslationsStream,
      ),
      ("/api/v1/basic-cards/lookup", StaticRoute::BasicCardLookup),
      ("/api/v1/senses/get", StaticRoute::SenseRead),
      ("/api/v1/knowledge/views", StaticRoute::KnowledgeViews),
      ("/api/v1/knowledge/paths", StaticRoute::KnowledgePaths),
    ];

    let identities = routes
      .iter()
      .map(|(path, expected)| {
        let route = StaticRoute::from_matched_path(path);
        assert_eq!(route, *expected);
        assert_eq!(route.as_str(), *path);
        route.as_str()
      })
      .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(identities.len(), routes.len());
    assert_eq!(
      StaticRoute::from_matched_path("/api/v1/translations"),
      StaticRoute::Translations
    );
    assert_eq!(
      StaticRoute::from_matched_path("/api/v1/translations/stream"),
      StaticRoute::TranslationsStream
    );
    assert_eq!(
      StaticRoute::from_matched_path("/api/v1/knowledge/views"),
      StaticRoute::KnowledgeViews
    );
    assert_eq!(
      StaticRoute::from_matched_path("/api/v1/knowledge/paths"),
      StaticRoute::KnowledgePaths
    );
  }

  #[test]
  fn unknown_and_removed_routes_fail_closed() {
    for path in [
      "/api/v1/private/request-content",
      "/api/v1/translations?query=private",
      "/v1/graph/nodes/:node_kind/:node_id",
      "/v1/graph/nodes/:node_kind/:node_id/neighbors",
    ] {
      assert_eq!(StaticRoute::from_matched_path(path), StaticRoute::Unmatched);
    }
    assert_eq!(StaticRoute::Unmatched.as_str(), "unmatched");
  }

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
        reason: TelemetryDropReason::SampledOut,
      },
      MetricEvent::TelemetryDropped {
        reason: TelemetryDropReason::Capacity,
      },
      MetricEvent::TelemetryDropped {
        reason: TelemetryDropReason::RuntimeUnavailable,
      },
      MetricEvent::TelemetryDropped {
        reason: TelemetryDropReason::ExporterUnavailable,
      },
      MetricEvent::TelemetryDropped {
        reason: TelemetryDropReason::ExportTimeout,
      },
      MetricEvent::TelemetryDropped {
        reason: TelemetryDropReason::Serialization,
      },
      MetricEvent::TelemetryDropped {
        reason: TelemetryDropReason::ShuttingDown,
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
