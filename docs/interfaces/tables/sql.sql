-- Transnet target MySQL 8 schema.
--
-- Status: target design. The current Transnet executable does not create, migrate,
-- or directly access these tables. Deploy the two schemas with separate grants:
-- Transnet may reach transnet_canonical only through island-port's SQL endpoint;
-- island_product is owned exclusively by island-port.
--
-- Published revisions and activated snapshots are immutable by application policy.
-- Live translation text, lookup context, request history, provider output, raw user
-- identifiers, bearer tokens, IP addresses, and user-agent strings must never be
-- inserted. principal_ref and idempotency_key_digest are application-keyed digests.

SET NAMES utf8mb4 COLLATE utf8mb4_0900_ai_ci;
SET time_zone = '+00:00';

CREATE DATABASE IF NOT EXISTS transnet_canonical
  CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;
CREATE DATABASE IF NOT EXISTS island_product
  CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;

USE transnet_canonical;

CREATE TABLE content_release (
  release_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  lifecycle_state ENUM('staged', 'validated', 'active', 'retired', 'quarantined') NOT NULL,
  sql_schema_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  payload_schema_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  activated_at TIMESTAMP(6) NULL,
  manifest_hash BINARY(32) NOT NULL,
  predecessor_release_id VARCHAR(128) CHARACTER SET ascii NULL,
  UNIQUE KEY uq_content_release_manifest (manifest_hash),
  CONSTRAINT fk_content_release_predecessor FOREIGN KEY (predecessor_release_id)
    REFERENCES content_release (release_id),
  CONSTRAINT ck_content_release_activation CHECK (
    (lifecycle_state = 'active' AND activated_at IS NOT NULL) OR lifecycle_state <> 'active'
  )
) ENGINE = InnoDB;

CREATE TABLE release_component (
  release_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  component_kind ENUM('cards', 'translations', 'domains', 'facts', 'relationships', 'scales', 'qdrant_nodes', 'qdrant_edges') NOT NULL,
  component_version VARCHAR(128) CHARACTER SET ascii NOT NULL,
  row_count BIGINT UNSIGNED NOT NULL,
  content_hash BINARY(32) NOT NULL,
  compatibility_state ENUM('pending', 'compatible', 'incompatible') NOT NULL,
  PRIMARY KEY (release_id, component_kind),
  CONSTRAINT fk_release_component_release FOREIGN KEY (release_id)
    REFERENCES content_release (release_id),
  CONSTRAINT ck_release_component_count CHECK (row_count >= 0)
) ENGINE = InnoDB;

CREATE TABLE publication_job (
  job_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  actor_service VARCHAR(128) CHARACTER SET ascii NOT NULL,
  state ENUM('staging', 'validating', 'ready', 'published', 'failed', 'cancelled') NOT NULL,
  policy_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  started_at TIMESTAMP(6) NOT NULL,
  completed_at TIMESTAMP(6) NULL
) ENGINE = InnoDB;

CREATE TABLE publication_validation (
  job_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  check_name VARCHAR(128) CHARACTER SET ascii NOT NULL,
  result ENUM('passed', 'failed', 'skipped') NOT NULL,
  checked_count BIGINT UNSIGNED NOT NULL,
  diagnostic_code VARCHAR(128) CHARACTER SET ascii NULL,
  PRIMARY KEY (job_id, check_name),
  CONSTRAINT fk_publication_validation_job FOREIGN KEY (job_id)
    REFERENCES publication_job (job_id)
) ENGINE = InnoDB;

CREATE TABLE publication_idempotency (
  publisher_scope VARCHAR(128) CHARACTER SET ascii NOT NULL,
  idempotency_key_digest BINARY(32) NOT NULL,
  request_fingerprint BINARY(32) NOT NULL,
  result_reference VARCHAR(255) CHARACTER SET ascii NOT NULL,
  expires_at TIMESTAMP(6) NOT NULL,
  PRIMARY KEY (publisher_scope, idempotency_key_digest),
  KEY ix_publication_idempotency_expiry (expires_at)
) ENGINE = InnoDB;

CREATE TABLE source (
  source_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  source_class VARCHAR(64) CHARACTER SET ascii NOT NULL,
  citation_metadata JSON NOT NULL,
  citation_schema_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  rights_policy VARCHAR(128) CHARACTER SET ascii NOT NULL,
  lifecycle_state ENUM('active', 'restricted', 'withdrawn') NOT NULL,
  CONSTRAINT ck_source_citation_json CHECK (JSON_VALID(citation_metadata))
) ENGINE = InnoDB;

CREATE TABLE evidence (
  evidence_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  revision INT UNSIGNED NOT NULL,
  source_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  locator VARCHAR(1024) NOT NULL,
  evidence_state ENUM('supported', 'disputed', 'withdrawn') NOT NULL,
  redistribution_policy VARCHAR(128) CHARACTER SET ascii NOT NULL,
  content_hash BINARY(32) NOT NULL,
  PRIMARY KEY (evidence_id, revision),
  KEY ix_evidence_source (source_id),
  CONSTRAINT fk_evidence_source FOREIGN KEY (source_id) REFERENCES source (source_id),
  CONSTRAINT ck_evidence_revision CHECK (revision > 0)
) ENGINE = InnoDB;

CREATE TABLE provenance_record (
  provenance_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  producer_class VARCHAR(64) CHARACTER SET ascii NOT NULL,
  policy_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  tool_version VARCHAR(128) CHARACTER SET ascii NULL,
  recorded_at TIMESTAMP(6) NOT NULL,
  metadata_schema_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  safe_metadata JSON NOT NULL,
  CONSTRAINT ck_provenance_metadata_json CHECK (JSON_VALID(safe_metadata))
) ENGINE = InnoDB;

CREATE TABLE lexeme (
  lexeme_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  language VARCHAR(35) CHARACTER SET ascii NOT NULL,
  created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  KEY ix_lexeme_language (language)
) ENGINE = InnoDB;

CREATE TABLE lexeme_form (
  lexeme_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  normalized_form VARCHAR(512) NOT NULL,
  normalizer_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  match_class ENUM('canonical', 'alias', 'inflection', 'spelling', 'transliteration', 'abbreviation') NOT NULL,
  display_form VARCHAR(512) NOT NULL,
  script VARCHAR(32) CHARACTER SET ascii NULL,
  PRIMARY KEY (lexeme_id, normalized_form, normalizer_version, match_class),
  KEY ix_lexeme_form_lookup (normalized_form, normalizer_version, match_class),
  CONSTRAINT fk_lexeme_form_lexeme FOREIGN KEY (lexeme_id) REFERENCES lexeme (lexeme_id)
) ENGINE = InnoDB;

CREATE TABLE sense (
  sense_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  lexeme_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  KEY ix_sense_lexeme (lexeme_id),
  CONSTRAINT fk_sense_lexeme FOREIGN KEY (lexeme_id) REFERENCES lexeme (lexeme_id)
) ENGINE = InnoDB;

CREATE TABLE sense_revision (
  sense_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  revision INT UNSIGNED NOT NULL,
  definition TEXT NOT NULL,
  part_of_speech VARCHAR(64) CHARACTER SET ascii NULL,
  scope_json JSON NOT NULL,
  lifecycle_state ENUM('draft', 'approved', 'withdrawn') NOT NULL,
  content_hash BINARY(32) NOT NULL,
  PRIMARY KEY (sense_id, revision),
  CONSTRAINT fk_sense_revision_sense FOREIGN KEY (sense_id) REFERENCES sense (sense_id),
  CONSTRAINT ck_sense_revision_positive CHECK (revision > 0),
  CONSTRAINT ck_sense_revision_scope CHECK (JSON_VALID(scope_json))
) ENGINE = InnoDB;

CREATE TABLE basic_card_revision (
  card_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  revision INT UNSIGNED NOT NULL,
  sense_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  canonical_form VARCHAR(512) NOT NULL,
  language VARCHAR(35) CHARACTER SET ascii NOT NULL,
  concise_summary TEXT NOT NULL,
  content_hash BINARY(32) NOT NULL,
  PRIMARY KEY (card_id, revision),
  KEY ix_basic_card_sense (sense_id),
  CONSTRAINT fk_basic_card_sense FOREIGN KEY (sense_id) REFERENCES sense (sense_id),
  CONSTRAINT ck_basic_card_revision CHECK (revision > 0)
) ENGINE = InnoDB;

CREATE TABLE card_item (
  card_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  card_revision INT UNSIGNED NOT NULL,
  item_kind ENUM('translation', 'pronunciation', 'form', 'definition', 'example', 'usage_note') NOT NULL,
  ordinal SMALLINT UNSIGNED NOT NULL,
  language VARCHAR(35) CHARACTER SET ascii NULL,
  value_json JSON NOT NULL,
  value_schema_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  PRIMARY KEY (card_id, card_revision, item_kind, ordinal),
  CONSTRAINT fk_card_item_card FOREIGN KEY (card_id, card_revision)
    REFERENCES basic_card_revision (card_id, revision),
  CONSTRAINT ck_card_item_json CHECK (JSON_VALID(value_json))
) ENGINE = InnoDB;

CREATE TABLE domain (
  domain_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ENGINE = InnoDB;

CREATE TABLE domain_revision (
  domain_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  revision INT UNSIGNED NOT NULL,
  canonical_label VARCHAR(512) NOT NULL,
  definition TEXT NOT NULL,
  inclusion_scope JSON NOT NULL,
  exclusion_scope JSON NOT NULL,
  verification_state ENUM('verified', 'withdrawn') NOT NULL,
  coverage_state ENUM('seed', 'partial', 'curated') NOT NULL,
  content_hash BINARY(32) NOT NULL,
  PRIMARY KEY (domain_id, revision),
  CONSTRAINT fk_domain_revision_domain FOREIGN KEY (domain_id) REFERENCES domain (domain_id),
  CONSTRAINT ck_domain_revision_positive CHECK (revision > 0),
  CONSTRAINT ck_domain_inclusion_json CHECK (JSON_VALID(inclusion_scope)),
  CONSTRAINT ck_domain_exclusion_json CHECK (JSON_VALID(exclusion_scope))
) ENGINE = InnoDB;

CREATE TABLE domain_attribute (
  domain_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  domain_revision INT UNSIGNED NOT NULL,
  attribute_kind ENUM('label', 'broader', 'fact_family', 'language') NOT NULL,
  ordinal SMALLINT UNSIGNED NOT NULL,
  value VARCHAR(512) NOT NULL,
  language VARCHAR(35) CHARACTER SET ascii NULL,
  PRIMARY KEY (domain_id, domain_revision, attribute_kind, ordinal),
  KEY ix_domain_attribute_value (attribute_kind, value),
  CONSTRAINT fk_domain_attribute_revision FOREIGN KEY (domain_id, domain_revision)
    REFERENCES domain_revision (domain_id, revision)
) ENGINE = InnoDB;

CREATE TABLE canonical_translation (
  translation_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  unit ENUM('word', 'phrase', 'passage') NOT NULL,
  source_language VARCHAR(35) CHARACTER SET ascii NOT NULL,
  target_language VARCHAR(35) CHARACTER SET ascii NOT NULL,
  sense_id VARCHAR(128) CHARACTER SET ascii NULL,
  KEY ix_translation_languages (source_language, target_language),
  KEY ix_translation_sense (sense_id),
  CONSTRAINT fk_translation_sense FOREIGN KEY (sense_id) REFERENCES sense (sense_id)
) ENGINE = InnoDB;

CREATE TABLE canonical_translation_revision (
  translation_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  revision INT UNSIGNED NOT NULL,
  source_text TEXT NOT NULL,
  target_text TEXT NOT NULL,
  source_fingerprint BINARY(32) NOT NULL,
  normalizer_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  dialect VARCHAR(35) CHARACTER SET ascii NULL,
  register_name VARCHAR(64) CHARACTER SET ascii NULL,
  scope_json JSON NOT NULL,
  rights_assertion VARCHAR(128) CHARACTER SET ascii NOT NULL,
  selection_reason VARCHAR(255) NOT NULL,
  review_decision ENUM('approved', 'rejected', 'withdrawn') NOT NULL,
  content_hash BINARY(32) NOT NULL,
  PRIMARY KEY (translation_id, revision),
  KEY ix_translation_source_fingerprint (source_fingerprint),
  CONSTRAINT fk_translation_revision_identity FOREIGN KEY (translation_id)
    REFERENCES canonical_translation (translation_id),
  CONSTRAINT ck_translation_revision_positive CHECK (revision > 0),
  CONSTRAINT ck_translation_scope_json CHECK (JSON_VALID(scope_json))
) ENGINE = InnoDB;

CREATE TABLE knowledge_fact (
  fact_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ENGINE = InnoDB;

CREATE TABLE knowledge_fact_revision (
  fact_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  revision INT UNSIGNED NOT NULL,
  subject_node_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  predicate VARCHAR(128) CHARACTER SET ascii NOT NULL,
  object_node_id VARCHAR(128) CHARACTER SET ascii NULL,
  object_literal JSON NULL,
  statement TEXT NOT NULL,
  verification_state ENUM('verified', 'withdrawn') NOT NULL,
  content_hash BINARY(32) NOT NULL,
  PRIMARY KEY (fact_id, revision),
  KEY ix_fact_subject_predicate (subject_node_id, predicate),
  KEY ix_fact_object_node (object_node_id),
  CONSTRAINT fk_fact_revision_identity FOREIGN KEY (fact_id) REFERENCES knowledge_fact (fact_id),
  CONSTRAINT ck_fact_revision_positive CHECK (revision > 0),
  CONSTRAINT ck_fact_one_object CHECK ((object_node_id IS NULL) <> (object_literal IS NULL)),
  CONSTRAINT ck_fact_literal_json CHECK (object_literal IS NULL OR JSON_VALID(object_literal))
) ENGINE = InnoDB;

CREATE TABLE revision_reference (
  owner_kind ENUM('card', 'translation', 'fact', 'scale') NOT NULL,
  owner_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  owner_revision INT UNSIGNED NOT NULL,
  reference_kind ENUM('domain', 'sense', 'condition', 'evidence', 'provenance', 'knowledge_root') NOT NULL,
  ordinal SMALLINT UNSIGNED NOT NULL,
  referenced_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  referenced_revision INT UNSIGNED NULL,
  PRIMARY KEY (owner_kind, owner_id, owner_revision, reference_kind, ordinal),
  KEY ix_revision_reference_target (reference_kind, referenced_id, referenced_revision),
  CONSTRAINT ck_revision_reference_owner_revision CHECK (owner_revision > 0)
) ENGINE = InnoDB;

CREATE TABLE knowledge_relationship (
  edge_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  fact_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  CONSTRAINT fk_relationship_fact FOREIGN KEY (fact_id) REFERENCES knowledge_fact (fact_id)
) ENGINE = InnoDB;

CREATE TABLE knowledge_relationship_revision (
  edge_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  fact_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  fact_revision INT UNSIGNED NOT NULL,
  source_node_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  target_node_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  relation_type VARCHAR(128) CHARACTER SET ascii NOT NULL,
  direction ENUM('directed', 'symmetric') NOT NULL,
  explanation TEXT NOT NULL,
  restrictions_json JSON NOT NULL,
  assessment_enabled BOOLEAN NOT NULL DEFAULT FALSE,
  review_state ENUM('approved', 'withdrawn') NOT NULL,
  content_hash BINARY(32) NOT NULL,
  PRIMARY KEY (edge_id, relation_version),
  KEY ix_relationship_endpoints (source_node_id, target_node_id),
  KEY ix_relationship_type (relation_type),
  CONSTRAINT fk_relationship_revision_edge FOREIGN KEY (edge_id)
    REFERENCES knowledge_relationship (edge_id),
  CONSTRAINT fk_relationship_revision_fact FOREIGN KEY (fact_id, fact_revision)
    REFERENCES knowledge_fact_revision (fact_id, revision),
  CONSTRAINT ck_relationship_version CHECK (relation_version > 0),
  CONSTRAINT ck_relationship_restrictions_json CHECK (JSON_VALID(restrictions_json)),
  CONSTRAINT ck_relationship_distinct_endpoints CHECK (source_node_id <> target_node_id)
) ENGINE = InnoDB;

CREATE TABLE semantic_scale (
  scale_id VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ENGINE = InnoDB;

CREATE TABLE semantic_scale_revision (
  scale_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  revision INT UNSIGNED NOT NULL,
  dimension_name VARCHAR(128) CHARACTER SET ascii NOT NULL,
  direction ENUM('increasing', 'decreasing') NOT NULL,
  conditions_json JSON NOT NULL,
  verification_state ENUM('verified', 'withdrawn') NOT NULL,
  content_hash BINARY(32) NOT NULL,
  PRIMARY KEY (scale_id, revision),
  CONSTRAINT fk_scale_revision_identity FOREIGN KEY (scale_id) REFERENCES semantic_scale (scale_id),
  CONSTRAINT ck_scale_revision_positive CHECK (revision > 0),
  CONSTRAINT ck_scale_conditions_json CHECK (JSON_VALID(conditions_json))
) ENGINE = InnoDB;

CREATE TABLE semantic_scale_member (
  scale_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  scale_revision INT UNSIGNED NOT NULL,
  position SMALLINT UNSIGNED NOT NULL,
  node_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  sense_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  PRIMARY KEY (scale_id, scale_revision, position),
  UNIQUE KEY uq_scale_member_node_sense (scale_id, scale_revision, node_id, sense_id),
  CONSTRAINT fk_scale_member_revision FOREIGN KEY (scale_id, scale_revision)
    REFERENCES semantic_scale_revision (scale_id, revision),
  CONSTRAINT fk_scale_member_sense FOREIGN KEY (sense_id) REFERENCES sense (sense_id)
) ENGINE = InnoDB;

CREATE TABLE release_sense (
  release_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  sense_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  sense_revision INT UNSIGNED NOT NULL,
  PRIMARY KEY (release_id, sense_id),
  CONSTRAINT fk_release_sense_release FOREIGN KEY (release_id) REFERENCES content_release (release_id),
  CONSTRAINT fk_release_sense_revision FOREIGN KEY (sense_id, sense_revision)
    REFERENCES sense_revision (sense_id, revision)
) ENGINE = InnoDB;

CREATE TABLE release_basic_card (
  release_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  card_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  card_revision INT UNSIGNED NOT NULL,
  PRIMARY KEY (release_id, card_id),
  CONSTRAINT fk_release_card_release FOREIGN KEY (release_id) REFERENCES content_release (release_id),
  CONSTRAINT fk_release_card_revision FOREIGN KEY (card_id, card_revision)
    REFERENCES basic_card_revision (card_id, revision)
) ENGINE = InnoDB;

CREATE TABLE release_translation (
  release_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  translation_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  translation_revision INT UNSIGNED NOT NULL,
  PRIMARY KEY (release_id, translation_id),
  CONSTRAINT fk_release_translation_release FOREIGN KEY (release_id) REFERENCES content_release (release_id),
  CONSTRAINT fk_release_translation_revision FOREIGN KEY (translation_id, translation_revision)
    REFERENCES canonical_translation_revision (translation_id, revision)
) ENGINE = InnoDB;

CREATE TABLE release_domain (
  release_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  domain_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  domain_revision INT UNSIGNED NOT NULL,
  PRIMARY KEY (release_id, domain_id),
  CONSTRAINT fk_release_domain_release FOREIGN KEY (release_id) REFERENCES content_release (release_id),
  CONSTRAINT fk_release_domain_revision FOREIGN KEY (domain_id, domain_revision)
    REFERENCES domain_revision (domain_id, revision)
) ENGINE = InnoDB;

CREATE TABLE release_fact (
  release_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  fact_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  fact_revision INT UNSIGNED NOT NULL,
  PRIMARY KEY (release_id, fact_id),
  CONSTRAINT fk_release_fact_release FOREIGN KEY (release_id) REFERENCES content_release (release_id),
  CONSTRAINT fk_release_fact_revision FOREIGN KEY (fact_id, fact_revision)
    REFERENCES knowledge_fact_revision (fact_id, revision)
) ENGINE = InnoDB;

CREATE TABLE release_relationship (
  release_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  edge_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  PRIMARY KEY (release_id, edge_id),
  UNIQUE KEY uq_release_relationship_version (release_id, edge_id, relation_version),
  CONSTRAINT fk_release_relationship_release FOREIGN KEY (release_id)
    REFERENCES content_release (release_id),
  CONSTRAINT fk_release_relationship_revision FOREIGN KEY (edge_id, relation_version)
    REFERENCES knowledge_relationship_revision (edge_id, relation_version)
) ENGINE = InnoDB;

CREATE TABLE release_scale (
  release_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  scale_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  scale_revision INT UNSIGNED NOT NULL,
  PRIMARY KEY (release_id, scale_id),
  CONSTRAINT fk_release_scale_release FOREIGN KEY (release_id) REFERENCES content_release (release_id),
  CONSTRAINT fk_release_scale_revision FOREIGN KEY (scale_id, scale_revision)
    REFERENCES semantic_scale_revision (scale_id, revision)
) ENGINE = InnoDB;

CREATE TABLE qdrant_projection_outbox (
  command_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  release_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  point_family ENUM('node', 'edge') NOT NULL,
  operation ENUM('upsert', 'delete') NOT NULL,
  canonical_record_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  canonical_revision INT UNSIGNED NOT NULL,
  content_hash BINARY(32) NOT NULL,
  state ENUM('pending', 'processing', 'complete', 'failed') NOT NULL DEFAULT 'pending',
  attempt_count SMALLINT UNSIGNED NOT NULL DEFAULT 0,
  available_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  last_attempt_at TIMESTAMP(6) NULL,
  diagnostic_code VARCHAR(128) CHARACTER SET ascii NULL,
  UNIQUE KEY uq_projection_command (release_id, point_family, canonical_record_id, canonical_revision, operation),
  KEY ix_projection_dispatch (state, available_at, command_id),
  CONSTRAINT fk_projection_release FOREIGN KEY (release_id) REFERENCES content_release (release_id),
  CONSTRAINT ck_projection_revision CHECK (canonical_revision > 0)
) ENGINE = InnoDB;

USE island_product;

CREATE TABLE relationship_judgment_event (
  event_id BINARY(16) PRIMARY KEY,
  principal_ref BINARY(32) NOT NULL,
  route_scope VARCHAR(128) CHARACTER SET ascii NOT NULL,
  edge_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  content_release VARCHAR(128) CHARACTER SET ascii NOT NULL,
  judgment ENUM('confirm', 'challenge') NOT NULL,
  request_fingerprint BINARY(32) NOT NULL,
  idempotency_key_digest BINARY(32) NOT NULL,
  eligibility_decision ENUM('eligible', 'ineligible') NOT NULL,
  recorded_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  UNIQUE KEY uq_judgment_idempotency (principal_ref, route_scope, idempotency_key_digest),
  KEY ix_judgment_principal_target (principal_ref, content_release, edge_id, relation_version),
  KEY ix_judgment_target_time (content_release, edge_id, relation_version, recorded_at),
  CONSTRAINT ck_judgment_relation_version CHECK (relation_version > 0)
) ENGINE = InnoDB;

CREATE TABLE relationship_judgment_current (
  principal_ref BINARY(32) NOT NULL,
  content_release VARCHAR(128) CHARACTER SET ascii NOT NULL,
  edge_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  judgment ENUM('confirm', 'challenge') NOT NULL,
  latest_event_id BINARY(16) NOT NULL,
  projection_version BIGINT UNSIGNED NOT NULL,
  updated_at TIMESTAMP(6) NOT NULL,
  PRIMARY KEY (principal_ref, content_release, edge_id, relation_version),
  KEY ix_current_target_judgment (content_release, edge_id, relation_version, judgment),
  CONSTRAINT fk_current_latest_event FOREIGN KEY (latest_event_id)
    REFERENCES relationship_judgment_event (event_id),
  CONSTRAINT ck_current_projection_version CHECK (projection_version > 0)
) ENGINE = InnoDB;

CREATE TABLE relationship_judgment_moderation (
  event_id BINARY(16) PRIMARY KEY,
  eligibility_outcome ENUM('eligible', 'ineligible', 'pending') NOT NULL,
  policy_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  reason_code VARCHAR(128) CHARACTER SET ascii NULL,
  decided_at TIMESTAMP(6) NULL,
  CONSTRAINT fk_moderation_event FOREIGN KEY (event_id)
    REFERENCES relationship_judgment_event (event_id),
  CONSTRAINT ck_moderation_decision_time CHECK (
    (eligibility_outcome = 'pending' AND decided_at IS NULL) OR
    (eligibility_outcome <> 'pending' AND decided_at IS NOT NULL)
  )
) ENGINE = InnoDB;

CREATE TABLE relationship_aggregate_release (
  aggregate_version VARCHAR(128) CHARACTER SET ascii PRIMARY KEY,
  lifecycle_state ENUM('staged', 'active', 'retired') NOT NULL,
  activated_at TIMESTAMP(6) NULL,
  predecessor_version VARCHAR(128) CHARACTER SET ascii NULL,
  row_count BIGINT UNSIGNED NOT NULL,
  content_hash BINARY(32) NOT NULL,
  UNIQUE KEY uq_aggregate_content_hash (content_hash),
  CONSTRAINT fk_aggregate_predecessor FOREIGN KEY (predecessor_version)
    REFERENCES relationship_aggregate_release (aggregate_version),
  CONSTRAINT ck_aggregate_activation CHECK (
    (lifecycle_state = 'active' AND activated_at IS NOT NULL) OR lifecycle_state <> 'active'
  )
) ENGINE = InnoDB;

CREATE TABLE relationship_judgment_aggregate (
  aggregate_version VARCHAR(128) CHARACTER SET ascii NOT NULL,
  content_release VARCHAR(128) CHARACTER SET ascii NOT NULL,
  edge_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  eligible_confirm_count INT UNSIGNED NOT NULL,
  eligible_challenge_count INT UNSIGNED NOT NULL,
  threshold_state ENUM('below_threshold', 'eligible') NOT NULL,
  policy_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  source_window_end TIMESTAMP(6) NOT NULL,
  calculated_at TIMESTAMP(6) NOT NULL,
  PRIMARY KEY (aggregate_version, content_release, edge_id, relation_version),
  KEY ix_judgment_aggregate_target (content_release, edge_id, relation_version),
  CONSTRAINT fk_judgment_aggregate_release FOREIGN KEY (aggregate_version)
    REFERENCES relationship_aggregate_release (aggregate_version),
  CONSTRAINT ck_judgment_aggregate_threshold CHECK (
    (threshold_state = 'eligible' AND eligible_confirm_count + eligible_challenge_count >= 5) OR
    (threshold_state = 'below_threshold' AND eligible_confirm_count + eligible_challenge_count < 5)
  )
) ENGINE = InnoDB;

CREATE TABLE relationship_distance_projection (
  aggregate_version VARCHAR(128) CHARACTER SET ascii NOT NULL,
  content_release VARCHAR(128) CHARACTER SET ascii NOT NULL,
  edge_id VARCHAR(128) CHARACTER SET ascii NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  base_distance_basis_points SMALLINT UNSIGNED NOT NULL,
  adjustment_basis_points SMALLINT NOT NULL,
  effective_distance_basis_points SMALLINT UNSIGNED NOT NULL,
  algorithm_version VARCHAR(64) CHARACTER SET ascii NOT NULL,
  calculated_at TIMESTAMP(6) NOT NULL,
  PRIMARY KEY (aggregate_version, content_release, edge_id, relation_version),
  KEY ix_distance_target (content_release, edge_id, relation_version),
  KEY ix_distance_effective (aggregate_version, effective_distance_basis_points),
  CONSTRAINT fk_distance_aggregate FOREIGN KEY (
    aggregate_version, content_release, edge_id, relation_version
  ) REFERENCES relationship_judgment_aggregate (
    aggregate_version, content_release, edge_id, relation_version
  ),
  CONSTRAINT ck_distance_base CHECK (base_distance_basis_points BETWEEN 0 AND 10000),
  CONSTRAINT ck_distance_adjustment CHECK (adjustment_basis_points BETWEEN -1500 AND 1500),
  CONSTRAINT ck_distance_effective CHECK (effective_distance_basis_points BETWEEN 0 AND 10000),
  CONSTRAINT ck_distance_algorithm CHECK (algorithm_version = 'relationship-distance-v1'),
  CONSTRAINT ck_distance_formula CHECK (
    effective_distance_basis_points = LEAST(
      10000,
      GREATEST(0, base_distance_basis_points - adjustment_basis_points)
    )
  )
) ENGINE = InnoDB;
