-- Transnet target MySQL 8 schema.
--
-- Status: target design. The current Transnet executable does not create, migrate,
-- or directly access these tables. This hybrid schema keeps stable identity,
-- release membership, and frequently filtered fields relational. Type-specific,
-- bounded canonical content is immutable, schema-versioned JSON.
--
-- Deploy the schemas with separate grants. Transnet may reach transnet_canonical
-- only through island-port's SQL endpoint. island_product is owned exclusively by
-- island-port. Neither schema may receive live request text, lookup context,
-- request history, provider output, bearer tokens, IP addresses, or user agents.
-- principal_ref and idempotency_key_digest are application-keyed digests.

SET NAMES utf8mb4 COLLATE utf8mb4_0900_ai_ci;
SET time_zone = '+00:00';

CREATE DATABASE IF NOT EXISTS transnet_canonical
  CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;
CREATE DATABASE IF NOT EXISTS island_product
  CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;

USE transnet_canonical;

-- One row identifies and activates a complete SQL/Qdrant release trio. The
-- component manifest contains closed, schema-validated component names, versions,
-- counts, hashes, vector dimensions, and embedding-model versions.
CREATE TABLE content_release (
  release_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  lifecycle_state ENUM('staged', 'validated', 'active', 'retired', 'quarantined') NOT NULL,
  active_slot TINYINT GENERATED ALWAYS AS (
    CASE WHEN lifecycle_state = 'active' THEN 1 ELSE NULL END
  ) STORED,
  schema_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  component_manifest JSON NOT NULL,
  manifest_hash BINARY(32) NOT NULL,
  predecessor_release_id VARCHAR(128) CHARACTER SET ascii NULL,
  created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  activated_at TIMESTAMP(6) NULL,
  UNIQUE KEY uq_content_release_active (active_slot),
  UNIQUE KEY uq_content_release_manifest (manifest_hash),
  CONSTRAINT fk_content_release_predecessor FOREIGN KEY (predecessor_release_id)
    REFERENCES content_release (release_id),
  CONSTRAINT ck_content_release_manifest CHECK (JSON_VALID(component_manifest)),
  CONSTRAINT ck_content_release_activation CHECK (
    lifecycle_state <> 'active' OR activated_at IS NOT NULL
  )
) ENGINE = InnoDB;

-- Publication validation has one bounded lifecycle. validation_report is a
-- closed array of {check_name, result, checked_count, diagnostic_code} objects.
CREATE TABLE publication_job (
  job_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  actor_service VARCHAR(128) CHARACTER SET ascii NOT NULL,
  target_release_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  state ENUM('staging', 'validating', 'ready', 'published', 'failed', 'cancelled') NOT NULL,
  policy_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  validation_schema_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  validation_report JSON NOT NULL,
  result_reference VARCHAR(255) CHARACTER SET ascii NULL,
  started_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  completed_at TIMESTAMP(6) NULL,
  KEY ix_publication_job_release (target_release_id, state),
  CONSTRAINT fk_publication_job_release FOREIGN KEY (target_release_id)
    REFERENCES content_release (release_id),
  CONSTRAINT ck_publication_validation_json CHECK (JSON_VALID(validation_report)),
  CONSTRAINT ck_publication_completion CHECK (
    (state IN ('published', 'failed', 'cancelled') AND completed_at IS NOT NULL) OR
    (state NOT IN ('published', 'failed', 'cancelled') AND completed_at IS NULL)
  )
) ENGINE = InnoDB;

-- Idempotency has a shorter retention lifecycle than the publication audit record,
-- so it remains a compact independent table.
CREATE TABLE publication_idempotency (
  publisher_scope VARCHAR(128) CHARACTER SET ascii NOT NULL,
  idempotency_key_digest BINARY(32) NOT NULL,
  request_fingerprint BINARY(32) NOT NULL,
  job_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  expires_at TIMESTAMP(6) NOT NULL,
  PRIMARY KEY (publisher_scope, idempotency_key_digest),
  KEY ix_publication_idempotency_expiry (expires_at),
  CONSTRAINT fk_publication_idempotency_job FOREIGN KEY (job_id)
    REFERENCES publication_job (job_id)
) ENGINE = InnoDB;

-- Source identity is small and independently governed. citation_metadata is
-- versioned because source classes have different citation shapes.
CREATE TABLE canonical_source (
  source_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  source_class VARCHAR(64) CHARACTER SET ascii NOT NULL,
  citation_schema_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  citation_metadata JSON NOT NULL,
  rights_policy VARCHAR(128) CHARACTER SET ascii NOT NULL,
  lifecycle_state ENUM('active', 'restricted', 'withdrawn') NOT NULL,
  content_hash BINARY(32) NOT NULL,
  CONSTRAINT ck_source_citation_json CHECK (JSON_VALID(citation_metadata))
) ENGINE = InnoDB;

-- Evidence revisions remain separate because rights filtering and evidence
-- hydration address them directly. payload holds typed locator and safe provenance
-- metadata; it must not contain credentials, prompts, or live request material.
CREATE TABLE evidence_revision (
  evidence_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  revision INT UNSIGNED NOT NULL,
  source_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  evidence_state ENUM('supported', 'disputed', 'withdrawn') NOT NULL,
  redistribution_policy VARCHAR(128) CHARACTER SET ascii NOT NULL,
  payload_schema_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  payload JSON NOT NULL,
  content_hash BINARY(32) NOT NULL,
  PRIMARY KEY (evidence_id, revision),
  KEY ix_evidence_source_state (source_id, evidence_state),
  CONSTRAINT fk_evidence_source FOREIGN KEY (source_id)
    REFERENCES canonical_source (source_id),
  CONSTRAINT ck_evidence_revision CHECK (revision > 0),
  CONSTRAINT ck_evidence_payload_json CHECK (JSON_VALID(payload))
) ENGINE = InnoDB;

-- Stable identity shared by lexemes, senses, cards, translations, domains, facts,
-- and semantic scales. parent_entity_id represents only identity ownership such as
-- sense -> lexeme; semantic relationships belong in canonical_relationship.
CREATE TABLE canonical_entity (
  entity_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  entity_type ENUM('lexeme', 'sense', 'basic_card', 'translation', 'domain', 'fact', 'semantic_scale') NOT NULL,
  parent_entity_id VARCHAR(128) CHARACTER SET ascii NULL,
  created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  KEY ix_canonical_entity_type (entity_type, entity_id),
  KEY ix_canonical_entity_parent (parent_entity_id, entity_type),
  CONSTRAINT fk_canonical_entity_parent FOREIGN KEY (parent_entity_id)
    REFERENCES canonical_entity (entity_id),
  CONSTRAINT ck_canonical_entity_not_self CHECK (
    parent_entity_id IS NULL OR parent_entity_id <> entity_id
  )
) ENGINE = InnoDB;

-- Immutable content for every entity family. normalized_key, language, and
-- sense_entity_id are promoted because exact card/translation resolution filters
-- them frequently; all other bounded type-specific fields stay in payload.
-- Publisher schemas define required payloads:
--   lexeme: canonical form, forms/aliases, script and normalizer version
--   sense: definition, part of speech and scope
--   basic_card: ordered translations, pronunciations, forms, definitions,
--               examples, usage notes, domain/evidence/root references
--   translation: source/target text, unit, fingerprint, dialect/register/scope,
--                evidence/provenance, rights, selection and review data
--   domain: labels, definition, scopes, hierarchy, fact families, languages/profile
--   fact: subject, predicate, object/literal, statement, scope and support
--   semantic_scale: dimension, direction, conditions, ordered sense-qualified
--                   members, domains and evidence
CREATE TABLE canonical_entity_revision (
  entity_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  revision INT UNSIGNED NOT NULL,
  publication_state ENUM('draft', 'approved', 'withdrawn') NOT NULL,
  verification_state ENUM('verified', 'not_applicable', 'withdrawn') NOT NULL,
  normalized_key VARCHAR(512) NULL,
  language VARCHAR(35) CHARACTER SET ascii NULL,
  sense_entity_id VARCHAR(128) CHARACTER SET ascii NULL,
  payload_schema_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  payload JSON NOT NULL,
  content_hash BINARY(32) NOT NULL,
  PRIMARY KEY (entity_id, revision),
  KEY ix_entity_revision_lookup (normalized_key, language, publication_state),
  KEY ix_entity_revision_sense (sense_entity_id, publication_state),
  KEY ix_entity_revision_state (publication_state, verification_state),
  CONSTRAINT fk_entity_revision_identity FOREIGN KEY (entity_id)
    REFERENCES canonical_entity (entity_id),
  CONSTRAINT fk_entity_revision_sense FOREIGN KEY (sense_entity_id)
    REFERENCES canonical_entity (entity_id),
  CONSTRAINT ck_entity_revision_positive CHECK (revision > 0),
  CONSTRAINT ck_entity_revision_payload_json CHECK (JSON_VALID(payload))
) ENGINE = InnoDB;

-- Stable stored-edge identity. fact_entity_id must name a canonical_entity whose
-- type is fact; the publication validator enforces that cross-row type constraint.
CREATE TABLE canonical_relationship (
  edge_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  fact_entity_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  UNIQUE KEY uq_relationship_edge_fact (edge_id, fact_entity_id),
  CONSTRAINT fk_relationship_fact FOREIGN KEY (fact_entity_id)
    REFERENCES canonical_entity (entity_id)
) ENGINE = InnoDB;

-- Immutable edge content. Endpoint and relation fields remain relational for graph
-- reads and assessment validation. payload holds explanation, conditions,
-- restrictions, applicable senses, domains, evidence and safe provenance.
CREATE TABLE canonical_relationship_revision (
  edge_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  fact_entity_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  fact_revision INT UNSIGNED NOT NULL,
  source_entity_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  target_entity_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  relation_type VARCHAR(128) CHARACTER SET ascii NOT NULL,
  direction ENUM('directed', 'symmetric') NOT NULL,
  publication_state ENUM('draft', 'approved', 'withdrawn') NOT NULL,
  verification_state ENUM('verified', 'withdrawn') NOT NULL,
  assessment_enabled BOOLEAN NOT NULL DEFAULT FALSE,
  payload_schema_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  payload JSON NOT NULL,
  content_hash BINARY(32) NOT NULL,
  PRIMARY KEY (edge_id, relation_version),
  KEY ix_relationship_source (source_entity_id, relation_type, publication_state),
  KEY ix_relationship_target (target_entity_id, relation_type, publication_state),
  KEY ix_relationship_assessment (assessment_enabled, publication_state, edge_id),
  CONSTRAINT fk_relationship_revision_identity FOREIGN KEY (edge_id, fact_entity_id)
    REFERENCES canonical_relationship (edge_id, fact_entity_id),
  CONSTRAINT fk_relationship_revision_fact FOREIGN KEY (fact_entity_id, fact_revision)
    REFERENCES canonical_entity_revision (entity_id, revision),
  CONSTRAINT fk_relationship_revision_source FOREIGN KEY (source_entity_id)
    REFERENCES canonical_entity (entity_id),
  CONSTRAINT fk_relationship_revision_target FOREIGN KEY (target_entity_id)
    REFERENCES canonical_entity (entity_id),
  CONSTRAINT ck_relationship_version CHECK (relation_version > 0),
  CONSTRAINT ck_relationship_fact_revision CHECK (fact_revision > 0),
  CONSTRAINT ck_relationship_distinct_endpoints CHECK (source_entity_id <> target_entity_id),
  CONSTRAINT ck_relationship_payload_json CHECK (JSON_VALID(payload))
) ENGINE = InnoDB;

-- One generic membership table pins both entity revisions and relationship
-- versions. MySQL cannot express a polymorphic foreign key; the publication
-- transaction validates member_kind against the corresponding revision table
-- before a release can enter validated or active state.
CREATE TABLE release_member (
  release_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  member_kind ENUM('entity', 'relationship') NOT NULL,
  member_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  member_revision INT UNSIGNED NOT NULL,
  member_type VARCHAR(64) CHARACTER SET ascii NOT NULL,
  content_hash BINARY(32) NOT NULL,
  PRIMARY KEY (release_id, member_kind, member_id),
  KEY ix_release_member_reverse (member_kind, member_id, member_revision, release_id),
  KEY ix_release_member_type (release_id, member_kind, member_type),
  CONSTRAINT fk_release_member_release FOREIGN KEY (release_id)
    REFERENCES content_release (release_id),
  CONSTRAINT ck_release_member_revision CHECK (member_revision > 0)
) ENGINE = InnoDB;

-- Durable, idempotent commands for rebuilding the Qdrant projection. Payload text
-- is always re-read from canonical revisions and is never copied into the outbox.
CREATE TABLE qdrant_projection_outbox (
  command_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  release_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  point_family ENUM('node', 'edge') NOT NULL,
  operation ENUM('upsert', 'delete') NOT NULL,
  canonical_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  canonical_revision INT UNSIGNED NOT NULL,
  content_hash BINARY(32) NOT NULL,
  state ENUM('pending', 'processing', 'complete', 'failed') NOT NULL DEFAULT 'pending',
  attempt_count SMALLINT UNSIGNED NOT NULL DEFAULT 0,
  available_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  last_attempt_at TIMESTAMP(6) NULL,
  diagnostic_code VARCHAR(128) CHARACTER SET ascii NULL,
  UNIQUE KEY uq_projection_command (
    release_id, point_family, canonical_id, canonical_revision, operation
  ),
  KEY ix_projection_dispatch (state, available_at, command_id),
  CONSTRAINT fk_projection_release FOREIGN KEY (release_id)
    REFERENCES content_release (release_id),
  CONSTRAINT ck_projection_revision CHECK (canonical_revision > 0)
) ENGINE = InnoDB;

USE island_product;

-- Append-only judgment events. No raw user identifier or free-form moderation text
-- is allowed. Idempotent replay returns this row; a fingerprint mismatch conflicts.
CREATE TABLE relationship_judgment_event (
  event_id BINARY(16) PRIMARY KEY,
  principal_ref BINARY(32) NOT NULL,
  route_scope VARCHAR(128) CHARACTER SET ascii NOT NULL,
  idempotency_key_digest BINARY(32) NOT NULL,
  request_fingerprint BINARY(32) NOT NULL,
  content_release VARCHAR(128) CHARACTER SET ascii NOT NULL,
  edge_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  judgment ENUM('confirm', 'challenge') NOT NULL,
  eligibility_decision ENUM('eligible', 'ineligible') NOT NULL,
  recorded_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  UNIQUE KEY uq_judgment_idempotency (
    principal_ref, route_scope, idempotency_key_digest
  ),
  KEY ix_judgment_target_time (
    content_release, edge_id, relation_version, recorded_at
  ),
  CONSTRAINT ck_judgment_relation_version CHECK (relation_version > 0)
) ENGINE = InnoDB;

-- Moderation has an independent lifecycle and cannot rewrite the immutable event.
-- Free-form notes belong in a separately protected system.
CREATE TABLE relationship_judgment_moderation (
  event_id BINARY(16) PRIMARY KEY,
  moderation_state ENUM('pending', 'eligible', 'ineligible') NOT NULL,
  policy_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  reason_code VARCHAR(128) CHARACTER SET ascii NULL,
  decided_at TIMESTAMP(6) NULL,
  KEY ix_judgment_moderation (moderation_state, decided_at),
  CONSTRAINT fk_moderation_event FOREIGN KEY (event_id)
    REFERENCES relationship_judgment_event (event_id),
  CONSTRAINT ck_moderation_decision CHECK (
    (moderation_state = 'pending' AND decided_at IS NULL) OR
    (moderation_state <> 'pending' AND decided_at IS NOT NULL)
  )
) ENGINE = InnoDB;

-- Read-optimized current value for one protected principal and exact target. It is
-- updated atomically with the append-only event and makes aggregation bounded.
CREATE TABLE relationship_judgment_current (
  principal_ref BINARY(32) NOT NULL,
  content_release VARCHAR(128) CHARACTER SET ascii NOT NULL,
  edge_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  judgment ENUM('confirm', 'challenge') NOT NULL,
  moderation_state ENUM('pending', 'eligible', 'ineligible') NOT NULL,
  latest_event_id BINARY(16) NOT NULL,
  projection_version BIGINT UNSIGNED NOT NULL,
  updated_at TIMESTAMP(6) NOT NULL,
  PRIMARY KEY (principal_ref, content_release, edge_id, relation_version),
  KEY ix_current_aggregate (
    content_release, edge_id, relation_version, moderation_state, judgment
  ),
  CONSTRAINT fk_current_latest_event FOREIGN KEY (latest_event_id)
    REFERENCES relationship_judgment_event (event_id),
  CONSTRAINT ck_current_projection_version CHECK (projection_version > 0)
) ENGINE = InnoDB;

-- One row identifies an immutable anonymous aggregate batch. active_slot enforces
-- that graph reads can resolve only one complete active snapshot.
CREATE TABLE relationship_aggregate_release (
  aggregate_version VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  lifecycle_state ENUM('staged', 'active', 'retired') NOT NULL,
  active_slot TINYINT GENERATED ALWAYS AS (
    CASE WHEN lifecycle_state = 'active' THEN 1 ELSE NULL END
  ) STORED,
  policy_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  algorithm_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  source_window_end TIMESTAMP(6) NOT NULL,
  row_count BIGINT UNSIGNED NOT NULL,
  content_hash BINARY(32) NOT NULL,
  predecessor_version VARCHAR(128) CHARACTER SET ascii NULL,
  created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  activated_at TIMESTAMP(6) NULL,
  UNIQUE KEY uq_aggregate_release_active (active_slot),
  UNIQUE KEY uq_aggregate_release_hash (content_hash),
  CONSTRAINT fk_aggregate_predecessor FOREIGN KEY (predecessor_version)
    REFERENCES relationship_aggregate_release (aggregate_version),
  CONSTRAINT ck_aggregate_algorithm CHECK (
    algorithm_version = 'relationship-distance-v1'
  ),
  CONSTRAINT ck_aggregate_activation CHECK (
    lifecycle_state <> 'active' OR activated_at IS NOT NULL
  )
) ENGINE = InnoDB;

-- Counts and their derived distance are one immutable projection because they have
-- identical keys and lifecycle. Rows below the privacy threshold are retained only
-- inside island-port and are never returned to Transnet.
CREATE TABLE relationship_assessment_projection (
  aggregate_version VARCHAR(128) CHARACTER SET ascii NOT NULL,
  content_release VARCHAR(128) CHARACTER SET ascii NOT NULL,
  edge_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  eligible_confirm_count INT UNSIGNED NOT NULL,
  eligible_challenge_count INT UNSIGNED NOT NULL,
  threshold_state ENUM('below_threshold', 'eligible') NOT NULL,
  base_distance_basis_points SMALLINT UNSIGNED NOT NULL,
  adjustment_basis_points SMALLINT NOT NULL,
  effective_distance_basis_points SMALLINT UNSIGNED NOT NULL,
  calculated_at TIMESTAMP(6) NOT NULL,
  PRIMARY KEY (aggregate_version, content_release, edge_id, relation_version),
  KEY ix_assessment_target (content_release, edge_id, relation_version),
  KEY ix_assessment_rank (aggregate_version, effective_distance_basis_points),
  CONSTRAINT fk_assessment_release FOREIGN KEY (aggregate_version)
    REFERENCES relationship_aggregate_release (aggregate_version),
  CONSTRAINT ck_assessment_relation_version CHECK (relation_version > 0),
  CONSTRAINT ck_assessment_threshold CHECK (
    (threshold_state = 'eligible' AND
      eligible_confirm_count + eligible_challenge_count >= 5) OR
    (threshold_state = 'below_threshold' AND
      eligible_confirm_count + eligible_challenge_count < 5)
  ),
  CONSTRAINT ck_assessment_base CHECK (
    base_distance_basis_points BETWEEN 0 AND 10000
  ),
  CONSTRAINT ck_assessment_adjustment CHECK (
    adjustment_basis_points BETWEEN -1500 AND 1500
  ),
  CONSTRAINT ck_assessment_effective CHECK (
    effective_distance_basis_points BETWEEN 0 AND 10000
  ),
  CONSTRAINT ck_assessment_formula CHECK (
    effective_distance_basis_points = LEAST(
      10000,
      GREATEST(0, base_distance_basis_points - adjustment_basis_points)
    )
  )
) ENGINE = InnoDB;
