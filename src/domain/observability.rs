//! Closed, redacted metric events for backend observability.
//!
//! This catalog names the operational signals required by the backend foundation without defining
//! an exporter, trace backend, retention policy, or alert thresholds. Metric events carry only
//! fixed enum dimensions. They cannot carry raw queries, contexts, answers, credentials or other
//! tokens, identities, or free-form attribute keys and values.

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
  /// Count of durable-job lifecycle transitions.
  JobLifecycleTotal,
  /// Count of categorical vector-reconciliation lag observations.
  VectorLagStateTotal,
  /// Count of graph-read operation outcomes.
  GraphOperationTotal,
  /// Count of private feedback operation outcomes without owner labels.
  FeedbackOperationTotal,
  /// Count of practice operation outcomes without learner labels.
  PracticeOperationTotal,
}

impl MetricName {
  /// Returns the fixed exporter-facing metric identifier.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::LookupStageTotal => "transnet_lookup_stage_total",
      Self::ModelValidationTotal => "transnet_model_validation_total",
      Self::JobLifecycleTotal => "transnet_job_lifecycle_total",
      Self::VectorLagStateTotal => "transnet_vector_lag_state_total",
      Self::GraphOperationTotal => "transnet_graph_operation_total",
      Self::FeedbackOperationTotal => "transnet_feedback_operation_total",
      Self::PracticeOperationTotal => "transnet_practice_operation_total",
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

/// A bounded result of validating a structured learning-model response.
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

/// A durable job family with operationally safe names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JobKind {
  /// A private asynchronous lookup job.
  Lookup,
  /// An immutable content-release workflow job.
  ContentRelease,
  /// A derived vector reconciliation job.
  VectorReconciliation,
}

impl JobKind {
  const fn as_label(self) -> &'static str {
    match self {
      Self::Lookup => "lookup",
      Self::ContentRelease => "content_release",
      Self::VectorReconciliation => "vector_reconciliation",
    }
  }
}

/// A durable job lifecycle outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JobLifecycleOutcome {
  /// The job was accepted for later processing.
  Enqueued,
  /// A worker obtained a bounded lease for the job.
  Leased,
  /// The job completed successfully.
  Completed,
  /// The job ended with a recoverable or terminal failure.
  Failed,
  /// The job became unavailable because its deadline or lease expired.
  Expired,
}

impl JobLifecycleOutcome {
  const fn as_label(self) -> &'static str {
    match self {
      Self::Enqueued => "enqueued",
      Self::Leased => "leased",
      Self::Completed => "completed",
      Self::Failed => "failed",
      Self::Expired => "expired",
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

/// A private graph-feedback operation without an owner or edge identifier label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeedbackOperation {
  /// Validate and append one private feedback event.
  Submission,
  /// Read one private current projection.
  ProjectionRead,
}

impl FeedbackOperation {
  const fn as_label(self) -> &'static str {
    match self {
      Self::Submission => "submission",
      Self::ProjectionRead => "projection_read",
    }
  }
}

/// A future private practice operation without learner or answer labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PracticeOperation {
  /// Claim one outstanding or scheduled practice item.
  ItemClaim,
  /// Submit one already-owned practice attempt.
  AttemptSubmission,
  /// Schedule or reschedule a private practice item.
  Scheduling,
}

impl PracticeOperation {
  const fn as_label(self) -> &'static str {
    match self {
      Self::ItemClaim => "item_claim",
      Self::AttemptSubmission => "attempt_submission",
      Self::Scheduling => "scheduling",
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
  /// One structured learning-model validation outcome.
  ModelValidation {
    /// Safe categorical validation result.
    outcome: ModelValidationOutcome,
  },
  /// One durable job lifecycle transition.
  JobLifecycle {
    /// Operational job family.
    job: JobKind,
    /// Safe lifecycle transition.
    outcome: JobLifecycleOutcome,
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
  /// One private graph-feedback operation outcome.
  FeedbackOperation {
    /// Private operation family without owner or target labels.
    operation: FeedbackOperation,
    /// Safe categorical result.
    outcome: MetricOutcome,
  },
  /// One private practice operation outcome.
  PracticeOperation {
    /// Private operation family without learner or answer labels.
    operation: PracticeOperation,
    /// Safe categorical result.
    outcome: MetricOutcome,
  },
}

impl MetricEvent {
  /// Returns the metric series to which this event belongs.
  pub const fn metric_name(self) -> MetricName {
    match self {
      Self::LookupStage { .. } => MetricName::LookupStageTotal,
      Self::ModelValidation { .. } => MetricName::ModelValidationTotal,
      Self::JobLifecycle { .. } => MetricName::JobLifecycleTotal,
      Self::VectorLag { .. } => MetricName::VectorLagStateTotal,
      Self::GraphOperation { .. } => MetricName::GraphOperationTotal,
      Self::FeedbackOperation { .. } => MetricName::FeedbackOperationTotal,
      Self::PracticeOperation { .. } => MetricName::PracticeOperationTotal,
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
      Self::JobLifecycle { job, outcome } => vec![
        MetricLabel::new("job", job.as_label()),
        MetricLabel::new("outcome", outcome.as_label()),
      ],
      Self::VectorLag { bucket } => {
        vec![MetricLabel::new("lag_bucket", bucket.as_label())]
      }
      Self::GraphOperation { operation, outcome } => vec![
        MetricLabel::new("operation", operation.as_label()),
        MetricLabel::new("outcome", outcome.as_label()),
      ],
      Self::FeedbackOperation { operation, outcome } => vec![
        MetricLabel::new("operation", operation.as_label()),
        MetricLabel::new("outcome", outcome.as_label()),
      ],
      Self::PracticeOperation { operation, outcome } => vec![
        MetricLabel::new("operation", operation.as_label()),
        MetricLabel::new("outcome", outcome.as_label()),
      ],
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
      MetricEvent::JobLifecycle {
        job: JobKind::Lookup,
        outcome: JobLifecycleOutcome::Enqueued,
      },
      MetricEvent::JobLifecycle {
        job: JobKind::ContentRelease,
        outcome: JobLifecycleOutcome::Leased,
      },
      MetricEvent::JobLifecycle {
        job: JobKind::VectorReconciliation,
        outcome: JobLifecycleOutcome::Completed,
      },
      MetricEvent::JobLifecycle {
        job: JobKind::Lookup,
        outcome: JobLifecycleOutcome::Failed,
      },
      MetricEvent::JobLifecycle {
        job: JobKind::Lookup,
        outcome: JobLifecycleOutcome::Expired,
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
      MetricEvent::FeedbackOperation {
        operation: FeedbackOperation::Submission,
        outcome: MetricOutcome::Succeeded,
      },
      MetricEvent::FeedbackOperation {
        operation: FeedbackOperation::ProjectionRead,
        outcome: MetricOutcome::Rejected,
      },
      MetricEvent::PracticeOperation {
        operation: PracticeOperation::ItemClaim,
        outcome: MetricOutcome::Succeeded,
      },
      MetricEvent::PracticeOperation {
        operation: PracticeOperation::AttemptSubmission,
        outcome: MetricOutcome::Rejected,
      },
      MetricEvent::PracticeOperation {
        operation: PracticeOperation::Scheduling,
        outcome: MetricOutcome::Failed,
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
        "transnet_feedback_operation_total",
        "transnet_graph_operation_total",
        "transnet_job_lifecycle_total",
        "transnet_lookup_stage_total",
        "transnet_model_validation_total",
        "transnet_practice_operation_total",
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
}
