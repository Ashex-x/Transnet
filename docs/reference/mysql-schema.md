# MySQL schema proposal

## Status

This document defines the proposed MySQL 8 persistence contract for Transnet basic core. It is not implemented at the branch point. Migrations will become authoritative when implementation begins; this reference then tracks the implemented schema.

[System design](../transnet.md) owns architecture and storage boundaries. This document owns table purposes, keys, relationships, encryption fields, retention, and transactional invariants.

## Storage conventions

- Use `utf8mb4` for user-visible strings and explicit ASCII binary collations for identifiers, enums, hashes, versions, and BCP-47 tags.
- Use UTC `DATETIME(6)` timestamps.
- Use compact `BIGINT UNSIGNED` internal keys and application-generated 26-character ULIDs as opaque external IDs.
- Preserve original Unicode and calculate explicit NFC, language-aware search keys instead of relying on one database collation.
- Add a foreign key and supporting index for every ownership or graph traversal path.
- Use JSON only for versioned opaque snapshots or type-specific payloads, not frequently filtered ontology.
- Keep event ledgers append-only and maintain current-state projections transactionally.
- Start with indexed retention and archival. Partitioned event tables require a separate design because the FK-heavy schema cannot be assumed compatible with MySQL partitioning.
- Encrypt database volumes, replicas, and backups. Encrypt identity display fields and learner content again at the application boundary.
- Generate a unique authenticated-encryption nonce under each key version and store the authentication tag with ciphertext.
- Use versioned HMACs with secrets outside MySQL for email, provider-subject, idempotency, and low-entropy query equality tokens.

## Logical ownership

| Boundary | Tables | Writer |
| --- | --- | --- |
| Canonical lexical | Releases, languages, sources, lexemes, forms, senses, assertions, relations, scales, patterns | Validated import and editorial pipeline |
| Generated | Card snapshots, generated examples, candidate relations, frozen exercises | Versioned generation and validation pipeline |
| User-owned | Preferences, history, saved senses, goals, layouts, attempts | Authenticated learner action |
| Community | Feedback events, current projections, aggregates, moderation | Feedback service and workers |
| Operations | Lookup jobs, generation runs, idempotency, outbox, privacy requests | API and workers |

## Canonical lexical catalog

| Table | Purpose and required constraints |
| --- | --- |
| `lexicon_releases` | Immutable release, source-manifest hash, status, publication time, and rollback predecessor |
| `active_content_version` | Singleton pointer to one validated lexicon release, vector collection, schema, and ranker combination |
| `languages` | Unique BCP-47 tag, display name, writing system support, morphology version, and enabled state |
| `lexical_sources` | Provider, version, license, attribution, display/embed/redistribution permissions, and import time |
| `evidence_fragments` | Source-local reference, content hash, language, assertion type, display policy, review state, and source FK |
| `lexemes` | Public ID, language, lemma, normalized key, part of speech, release interval, and status |
| `word_forms` | Lexeme, original form, normalized key, morphology features, evidence, and release interval |
| `form_aliases` | Form, alias text, romanization/misspelling type, scheme, confidence, and evidence |
| `pronunciations` | Lexeme, dialect, IPA, stress, licensed audio reference, evidence, and status |
| `sense_pronunciations` | Sense-pronunciation join for meaning-specific pronunciation |
| `senses` | Public ID, lexeme, sense key, English definition, qualified CEFR/frequency, release interval, and status |
| `sense_glosses` | Sense, explanation language, gloss, evidence, confidence, and review state |
| `usage_labels` | Register, dialect, domain, connotation, politeness, datedness, or sensitivity type |
| `sense_usage_labels` | Versioned sense-label assertion with scope, evidence, and confidence |
| `grammar_patterns` | Sense, structured construction, learner note, examples, evidence, and release |
| `learner_pitfalls` | Source language and sense pair, error category, correction, level, and evidence |
| `etymology_assertions` | Lexeme, origin/derivation assertion, date range, language, confidence, and evidence |
| `sense_history_events` | Sense, dated semantic-development event, uncertainty, display text, and evidence |
| `examples` | Sense, text, optional localized explanation, origin, source, license, level, and review status |
| `collocations` | Shared edge version when feedback-enabled, head sense, construction, corpus strength, dialect/register, and evidence |
| `collocation_lexeme_dependents` | Collocation and dependent lexeme with explicit grammatical role |
| `collocation_sense_dependents` | Collocation and dependent sense when meaning is constrained |
| `graph_edges` | Stable public identity and entity family for feedback-enabled graph assertions |
| `graph_edge_versions` | Composite `(graph_edge_id, version)` authority, feedback capability, scope, lifecycle, and publication time |
| `sense_relation_types` | Authoritative and inverse type, direction/symmetry rule, endpoint kinds, and display behavior |
| `sense_relations` | Shared edge/version key, sense pair, authoritative direction, type, scope, evidence confidence, and state |
| `lexeme_relation_types` | Inflection, derivation, and etymological relation types with inverse rules |
| `lexeme_relations` | Shared edge/version key, lexeme pair, type, scope, evidence confidence, and state |
| `semantic_scales` | Dimension, context, dialect/domain scope, evidence, and release |
| `semantic_scale_members` | Scale, sense, ordinal or calibrated value, confidence, and evidence |
| `relation_suggestions` | Proposed endpoints/type, generator, evidence bundle, similarity features, moderation state, and resolution |
| `entity_successors` | Retired entity/version, split/merge successor, confidence, release, and learner-state migration policy |
| `embedding_records` | Entity, purpose/content language, content hash, model/dimensions, collection, release, and synchronization state |

`graph_edges` and `graph_edge_versions` are a supertype registry. Sense relations, lexeme relations, and selected collocations use shared primary keys while retaining subtype-specific endpoint foreign keys. Derived scale adjacency has no registry row and cannot receive feedback directly.

Assertions supported by several sources use join tables such as `sense_relation_evidence`, `example_evidence`, and `detail_evidence`. Generated material remains non-authoritative until explicit review promotes it.

When a sense is split or merged, a high-confidence one-to-one successor may migrate saved state. An ambiguous split pauses mastery and asks the learner to choose. Historical attempts retain their original sense and release.

## Identity and preferences

```sql
CREATE TABLE users (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  email_ciphertext VARBINARY(1024) NULL,
  email_nonce BINARY(12) NULL,
  email_key_version SMALLINT UNSIGNED NULL,
  email_lookup_hmac BINARY(32) NULL,
  email_verified BOOLEAN NOT NULL DEFAULT FALSE,
  display_name_ciphertext VARBINARY(512) NULL,
  display_name_nonce BINARY(12) NULL,
  display_name_key_version SMALLINT UNSIGNED NULL,
  status ENUM('active', 'suspended', 'deleting') NOT NULL DEFAULT 'active',
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
    ON UPDATE CURRENT_TIMESTAMP(6),
  deleted_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_users_public_id (public_id),
  UNIQUE KEY uq_users_email_hmac (email_lookup_hmac)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE user_identities (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  user_id BIGINT UNSIGNED NOT NULL,
  provider VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  provider_issuer VARCHAR(255) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  provider_subject_hmac BINARY(32) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  last_used_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_identity_subject (provider, provider_issuer, provider_subject_hmac),
  KEY ix_identities_user (user_id),
  CONSTRAINT fk_identities_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE user_sessions (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  user_id BIGINT UNSIGNED NOT NULL,
  token_family_id BINARY(16) NOT NULL,
  refresh_token_hash BINARY(32) NOT NULL,
  device_label_ciphertext VARBINARY(512) NULL,
  device_label_nonce BINARY(12) NULL,
  device_label_key_version SMALLINT UNSIGNED NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  expires_at DATETIME(6) NOT NULL,
  last_used_at DATETIME(6) NULL,
  rotated_at DATETIME(6) NULL,
  replaced_by_session_id BIGINT UNSIGNED NULL,
  reuse_detected_at DATETIME(6) NULL,
  revoked_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_sessions_token_hash (refresh_token_hash),
  KEY ix_sessions_user_expiry (user_id, expires_at),
  KEY ix_sessions_token_family (user_id, token_family_id),
  CONSTRAINT fk_sessions_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT fk_sessions_replacement FOREIGN KEY (replaced_by_session_id)
    REFERENCES user_sessions (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE user_preferences (
  user_id BIGINT UNSIGNED NOT NULL,
  interface_locale VARCHAR(35) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  explanation_language VARCHAR(35) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  english_dialect VARCHAR(35) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'en-US',
  english_level ENUM('A1', 'A2', 'B1', 'B2', 'C1', 'C2', 'unknown')
    NOT NULL DEFAULT 'unknown',
  time_zone VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'UTC',
  daily_goal SMALLINT UNSIGNED NOT NULL DEFAULT 10,
  history_enabled BOOLEAN NOT NULL DEFAULT FALSE,
  history_retention_days SMALLINT UNSIGNED NOT NULL DEFAULT 90,
  personalization_enabled BOOLEAN NOT NULL DEFAULT TRUE,
  mature_content_mode ENUM('hide', 'warn', 'show') NOT NULL DEFAULT 'warn',
  accessibility_json JSON NOT NULL,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
    ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (user_id),
  CONSTRAINT chk_preferences_daily_goal CHECK (daily_goal BETWEEN 1 AND 500),
  CONSTRAINT chk_preferences_history_retention
    CHECK (history_retention_days BETWEEN 1 AND 3650),
  CONSTRAINT fk_preferences_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE user_languages (
  user_id BIGINT UNSIGNED NOT NULL,
  language_tag VARCHAR(35) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  role ENUM('primary', 'known', 'learning') NOT NULL,
  proficiency ENUM('A1', 'A2', 'B1', 'B2', 'C1', 'C2', 'native', 'unknown')
    NOT NULL,
  preference_order TINYINT UNSIGNED NOT NULL DEFAULT 0,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
    ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (user_id, language_tag, role),
  CONSTRAINT fk_user_languages_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE learning_goals (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  user_id BIGINT UNSIGNED NOT NULL,
  goal_type ENUM('general', 'travel', 'academic', 'business', 'exam', 'custom')
    NOT NULL,
  title_ciphertext VARBINARY(1024) NOT NULL,
  title_nonce BINARY(12) NOT NULL,
  title_key_version SMALLINT UNSIGNED NOT NULL,
  active BOOLEAN NOT NULL DEFAULT TRUE,
  target_date DATE NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
    ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY uq_learning_goals_public_id (public_id),
  KEY ix_learning_goals_user_active (user_id, active, updated_at),
  CONSTRAINT fk_learning_goals_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE user_consents (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  user_id BIGINT UNSIGNED NOT NULL,
  consent_type VARCHAR(40) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  policy_version VARCHAR(40) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  granted BOOLEAN NOT NULL,
  occurred_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  withdrawn_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  KEY ix_consents_user_type (user_id, consent_type, occurred_at DESC),
  CONSTRAINT fk_consents_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;
```

OIDC is the default authentication source. Local password credentials, if required later, use a dedicated table and a memory-hard password hash. Provider tokens, raw session tokens, passwords, and hidden model reasoning are never stored.

## History and saved vocabulary

```sql
CREATE TABLE learning_card_snapshots (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  cache_key BINARY(32) NOT NULL,
  canonical_card_json JSON NOT NULL,
  lexicon_release_id BIGINT UNSIGNED NOT NULL,
  ranking_version VARCHAR(40) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  model_version VARCHAR(160) CHARACTER SET ascii COLLATE ascii_bin NULL,
  prompt_version VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  expires_at DATETIME(6) NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_card_snapshots_public_id (public_id),
  UNIQUE KEY uq_card_snapshots_cache_key (cache_key),
  KEY ix_card_snapshots_expiry (expires_at),
  CONSTRAINT fk_card_snapshots_release FOREIGN KEY (lexicon_release_id)
    REFERENCES lexicon_releases (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE lookup_events (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  user_id BIGINT UNSIGNED NOT NULL,
  query_ciphertext VARBINARY(2048) NOT NULL,
  query_nonce BINARY(12) NOT NULL,
  query_key_version SMALLINT UNSIGNED NOT NULL,
  normalized_query_hmac BINARY(32) NOT NULL,
  query_hmac_key_version SMALLINT UNSIGNED NOT NULL,
  resolved_language_tag VARCHAR(35) CHARACTER SET ascii COLLATE ascii_bin NULL,
  selected_sense_id BIGINT UNSIGNED NULL,
  lexicon_release_id BIGINT UNSIGNED NOT NULL,
  card_snapshot_id BIGINT UNSIGNED NULL,
  occurred_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  expires_at DATETIME(6) NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_lookup_events_public_id (public_id),
  KEY ix_lookup_history (user_id, occurred_at DESC),
  KEY ix_lookup_repeat (user_id, normalized_query_hmac, occurred_at DESC),
  KEY ix_lookup_expiry (expires_at),
  CONSTRAINT fk_lookup_events_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT fk_lookup_events_sense FOREIGN KEY (selected_sense_id)
    REFERENCES senses (id),
  CONSTRAINT fk_lookup_events_release FOREIGN KEY (lexicon_release_id)
    REFERENCES lexicon_releases (id),
  CONSTRAINT fk_lookup_events_snapshot FOREIGN KEY (card_snapshot_id)
    REFERENCES learning_card_snapshots (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE saved_senses (
  user_id BIGINT UNSIGNED NOT NULL,
  sense_id BIGINT UNSIGNED NOT NULL,
  state ENUM('learning', 'known', 'paused', 'archived') NOT NULL DEFAULT 'learning',
  source ENUM('saved', 'lookup', 'practice', 'import') NOT NULL,
  note_ciphertext VARBINARY(4096) NULL,
  note_nonce BINARY(12) NULL,
  note_key_version SMALLINT UNSIGNED NULL,
  first_seen_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  last_seen_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
    ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (user_id, sense_id),
  KEY ix_saved_senses_state (user_id, state, updated_at),
  CONSTRAINT fk_saved_senses_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT fk_saved_senses_sense FOREIGN KEY (sense_id)
    REFERENCES senses (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;
```

A shared snapshot contains no original query, lookup ID, context ordering, policy filter, or private field. Each saved event calculates non-null `expires_at` from the current retention preference. Reducing a retention period may enqueue deletion; the system never silently extends a promised deletion date.

## Relationship feedback and graph layouts

```sql
CREATE TABLE relation_feedback_events (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  user_id BIGINT UNSIGNED NULL,
  relation_id BIGINT UNSIGNED NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  dimension ENUM('usefulness', 'accuracy') NOT NULL,
  judgment VARCHAR(40) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  context_hash BINARY(32) NULL,
  context_json JSON NULL,
  supersedes_event_id BIGINT UNSIGNED NULL,
  idempotency_key_hash BINARY(32) NOT NULL,
  comment_ciphertext VARBINARY(2048) NULL,
  comment_nonce BINARY(12) NULL,
  comment_key_version SMALLINT UNSIGNED NULL,
  occurred_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY uq_relation_feedback_public_id (public_id),
  UNIQUE KEY uq_relation_feedback_idempotency (user_id, idempotency_key_hash),
  KEY ix_relation_feedback_relation (relation_id, relation_version, occurred_at),
  CONSTRAINT fk_relation_feedback_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE SET NULL,
  CONSTRAINT fk_relation_feedback_relation
    FOREIGN KEY (relation_id, relation_version)
    REFERENCES graph_edge_versions (graph_edge_id, version),
  CONSTRAINT fk_relation_feedback_supersedes FOREIGN KEY (supersedes_event_id)
    REFERENCES relation_feedback_events (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE relation_feedback_current (
  user_id BIGINT UNSIGNED NOT NULL,
  relation_id BIGINT UNSIGNED NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  dimension ENUM('usefulness', 'accuracy') NOT NULL,
  judgment VARCHAR(40) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  source_event_id BIGINT UNSIGNED NOT NULL,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (user_id, relation_id, relation_version, dimension),
  KEY ix_feedback_current_relation (
    relation_id,
    relation_version,
    dimension,
    judgment
  ),
  CONSTRAINT fk_feedback_current_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT fk_feedback_current_relation
    FOREIGN KEY (relation_id, relation_version)
    REFERENCES graph_edge_versions (graph_edge_id, version),
  CONSTRAINT fk_feedback_current_event FOREIGN KEY (source_event_id)
    REFERENCES relation_feedback_events (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE relation_feedback_aggregates (
  relation_id BIGINT UNSIGNED NOT NULL,
  relation_version INT UNSIGNED NOT NULL,
  eligible_voter_count INT UNSIGNED NOT NULL DEFAULT 0,
  positive_weight DECIMAL(12,4) NOT NULL DEFAULT 0,
  problem_weight DECIMAL(12,4) NOT NULL DEFAULT 0,
  effective_sample_size DECIMAL(12,4) NOT NULL DEFAULT 0,
  community_posterior DECIMAL(5,4) NOT NULL DEFAULT 0.5000,
  sample_confidence DECIMAL(5,4) NOT NULL DEFAULT 0.0000,
  algorithm_version VARCHAR(40) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (relation_id, relation_version),
  CONSTRAINT chk_feedback_posterior CHECK (community_posterior BETWEEN 0 AND 1),
  CONSTRAINT chk_feedback_sample CHECK (sample_confidence BETWEEN 0 AND 1),
  CONSTRAINT fk_feedback_aggregate_relation
    FOREIGN KEY (relation_id, relation_version)
    REFERENCES graph_edge_versions (graph_edge_id, version)
    ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE saved_graph_views (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  user_id BIGINT UNSIGNED NOT NULL,
  root_entity_kind ENUM('sense', 'lexeme') NOT NULL,
  root_entity_public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  filter_hash BINARY(32) NOT NULL,
  content_release_public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  layout_algorithm VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  layout_version INT UNSIGNED NOT NULL DEFAULT 1,
  camera_json JSON NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
    ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY uq_graph_views_public_id (public_id),
  UNIQUE KEY uq_graph_view_context (
    user_id,
    root_entity_kind,
    root_entity_public_id,
    filter_hash,
    content_release_public_id,
    layout_algorithm
  ),
  CONSTRAINT fk_graph_views_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE saved_graph_node_positions (
  graph_view_id BIGINT UNSIGNED NOT NULL,
  node_kind ENUM('sense', 'lexeme', 'construction', 'scale') NOT NULL,
  node_public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  x DOUBLE NOT NULL,
  y DOUBLE NOT NULL,
  z DOUBLE NOT NULL,
  pinned BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
    ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (graph_view_id, node_kind, node_public_id),
  CONSTRAINT fk_graph_positions_view FOREIGN KEY (graph_view_id)
    REFERENCES saved_graph_views (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;
```

`context_json` contains only versioned non-sensitive graph context such as root ID, dialect, domain, and filters. Query text is forbidden. Saved coordinates must be finite, bounded, and refer to nodes in the saved view.

Publishing a new edge version creates an empty current projection. An old judgment is never applied to a semantically changed edge without an explicit audited migration.

## Practice and mastery

```sql
CREATE TABLE practice_sessions (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  user_id BIGINT UNSIGNED NOT NULL,
  status ENUM('active', 'completed', 'abandoned') NOT NULL DEFAULT 'active',
  requested_item_count SMALLINT UNSIGNED NOT NULL,
  served_item_count SMALLINT UNSIGNED NOT NULL DEFAULT 0,
  completed_item_count SMALLINT UNSIGNED NOT NULL DEFAULT 0,
  scheduler_version VARCHAR(40) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  completed_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_practice_sessions_public_id (public_id),
  KEY ix_practice_sessions_user_time (user_id, created_at DESC),
  CONSTRAINT chk_practice_session_count CHECK (requested_item_count BETWEEN 1 AND 100),
  CONSTRAINT fk_practice_sessions_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE exercise_instances (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  focus_sense_id BIGINT UNSIGNED NOT NULL,
  skill_code VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  item_type VARCHAR(40) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  prompt_language VARCHAR(35) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  english_dialect VARCHAR(35) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  level_band ENUM('A1', 'A2', 'B1', 'B2', 'C1', 'C2', 'mixed') NOT NULL,
  prompt_json JSON NOT NULL,
  accepted_answer_spec_json JSON NOT NULL,
  explanation_json JSON NOT NULL,
  origin ENUM('curated', 'template', 'generated') NOT NULL,
  lexicon_release_id BIGINT UNSIGNED NOT NULL,
  generator_version VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  model_version VARCHAR(160) CHARACTER SET ascii COLLATE ascii_bin NULL,
  prompt_version VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NULL,
  rubric_version VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  answer_normalization_version VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  review_status ENUM('candidate', 'active', 'quarantined', 'retired')
    NOT NULL DEFAULT 'candidate',
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY uq_exercise_instances_public_id (public_id),
  KEY ix_exercise_selection (
    focus_sense_id,
    skill_code,
    prompt_language,
    english_dialect,
    level_band,
    review_status
  ),
  CONSTRAINT fk_exercise_instances_sense FOREIGN KEY (focus_sense_id)
    REFERENCES senses (id),
  CONSTRAINT fk_exercise_instances_release FOREIGN KEY (lexicon_release_id)
    REFERENCES lexicon_releases (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE exercise_sense_targets (
  exercise_instance_id BIGINT UNSIGNED NOT NULL,
  sense_id BIGINT UNSIGNED NOT NULL,
  target_role VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  PRIMARY KEY (exercise_instance_id, sense_id, target_role),
  CONSTRAINT fk_exercise_sense_target FOREIGN KEY (exercise_instance_id)
    REFERENCES exercise_instances (id) ON DELETE CASCADE,
  CONSTRAINT fk_exercise_sense FOREIGN KEY (sense_id) REFERENCES senses (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE exercise_pattern_targets (
  exercise_instance_id BIGINT UNSIGNED NOT NULL,
  grammar_pattern_id BIGINT UNSIGNED NOT NULL,
  target_role VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  PRIMARY KEY (exercise_instance_id, grammar_pattern_id, target_role),
  CONSTRAINT fk_exercise_pattern_target FOREIGN KEY (exercise_instance_id)
    REFERENCES exercise_instances (id) ON DELETE CASCADE,
  CONSTRAINT fk_exercise_pattern FOREIGN KEY (grammar_pattern_id)
    REFERENCES grammar_patterns (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE exercise_collocation_targets (
  exercise_instance_id BIGINT UNSIGNED NOT NULL,
  collocation_id BIGINT UNSIGNED NOT NULL,
  target_role VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  PRIMARY KEY (exercise_instance_id, collocation_id, target_role),
  CONSTRAINT fk_exercise_collocation_target FOREIGN KEY (exercise_instance_id)
    REFERENCES exercise_instances (id) ON DELETE CASCADE,
  CONSTRAINT fk_exercise_collocation FOREIGN KEY (collocation_id)
    REFERENCES collocations (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE exercise_scale_targets (
  exercise_instance_id BIGINT UNSIGNED NOT NULL,
  semantic_scale_id BIGINT UNSIGNED NOT NULL,
  target_role VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  PRIMARY KEY (exercise_instance_id, semantic_scale_id, target_role),
  CONSTRAINT fk_exercise_scale_target FOREIGN KEY (exercise_instance_id)
    REFERENCES exercise_instances (id) ON DELETE CASCADE,
  CONSTRAINT fk_exercise_scale FOREIGN KEY (semantic_scale_id)
    REFERENCES semantic_scales (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE practice_session_items (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  practice_session_id BIGINT UNSIGNED NOT NULL,
  exercise_instance_id BIGINT UNSIGNED NOT NULL,
  position SMALLINT UNSIGNED NOT NULL,
  status ENUM('queued', 'served', 'answered', 'skipped') NOT NULL DEFAULT 'queued',
  served_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_session_items_public_id (public_id),
  UNIQUE KEY uq_session_item_position (practice_session_id, position),
  UNIQUE KEY uq_session_item_exercise (practice_session_id, exercise_instance_id),
  CONSTRAINT fk_session_items_session FOREIGN KEY (practice_session_id)
    REFERENCES practice_sessions (id) ON DELETE CASCADE,
  CONSTRAINT fk_session_items_exercise FOREIGN KEY (exercise_instance_id)
    REFERENCES exercise_instances (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE user_sense_skill_state (
  user_id BIGINT UNSIGNED NOT NULL,
  sense_id BIGINT UNSIGNED NOT NULL,
  skill_code VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  learning_state ENUM('new', 'learning', 'review', 'mastered', 'paused')
    NOT NULL DEFAULT 'new',
  scheduler_version VARCHAR(40) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  stability DECIMAL(10,4) NOT NULL DEFAULT 0,
  difficulty DECIMAL(10,4) NOT NULL DEFAULT 0,
  mastery_score DECIMAL(5,4) NOT NULL DEFAULT 0,
  success_streak INT UNSIGNED NOT NULL DEFAULT 0,
  lapse_count INT UNSIGNED NOT NULL DEFAULT 0,
  last_reviewed_at DATETIME(6) NULL,
  due_at DATETIME(6) NOT NULL,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
    ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (user_id, sense_id, skill_code),
  KEY ix_skill_state_due (user_id, learning_state, due_at),
  CONSTRAINT chk_skill_state_mastery CHECK (mastery_score BETWEEN 0 AND 1),
  CONSTRAINT fk_skill_state_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT fk_skill_state_sense FOREIGN KEY (sense_id)
    REFERENCES senses (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE practice_attempts (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  practice_session_item_id BIGINT UNSIGNED NOT NULL,
  answer_ciphertext VARBINARY(8192) NULL,
  answer_nonce BINARY(12) NULL,
  answer_key_version SMALLINT UNSIGNED NULL,
  answer_expires_at DATETIME(6) NULL,
  answer_redacted_at DATETIME(6) NULL,
  result ENUM('correct', 'partial', 'incorrect', 'skipped', 'needs_review') NOT NULL,
  scheduler_rating ENUM('again', 'hard', 'good', 'easy', 'unchanged') NOT NULL,
  score DECIMAL(5,4) NULL,
  evaluator_confidence DECIMAL(5,4) NOT NULL,
  response_ms INT UNSIGNED NULL,
  hints_used TINYINT UNSIGNED NOT NULL DEFAULT 0,
  evaluator_version VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  scheduler_version VARCHAR(40) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  idempotency_key_hash BINARY(32) NOT NULL,
  occurred_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY uq_practice_attempts_public_id (public_id),
  UNIQUE KEY uq_practice_attempts_idempotency (idempotency_key_hash),
  UNIQUE KEY uq_practice_attempts_session_item (practice_session_item_id),
  KEY ix_practice_attempts_time (occurred_at DESC),
  KEY ix_practice_attempts_answer_expiry (answer_expires_at),
  CONSTRAINT chk_attempt_score CHECK (score IS NULL OR score BETWEEN 0 AND 1),
  CONSTRAINT chk_evaluator_confidence CHECK (evaluator_confidence BETWEEN 0 AND 1),
  CONSTRAINT fk_practice_attempts_session_item FOREIGN KEY (practice_session_item_id)
    REFERENCES practice_session_items (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;
```

Exercise instances are immutable after activation. Secondary target tables allow discrimination, contrast, collocation, grammar, and scale exercises without unsafe polymorphic references.

Attempt submission locks the owned session item, resolves idempotency, inserts the event, updates session counters, and advances the focus-sense mastery row in one transaction. User, sense, and skill are derived through the session and frozen exercise rather than duplicated on the attempt.

Skipped attempts have no answer ciphertext. Other answers are redacted after a short review window while results and scheduler effects remain.

## Jobs, idempotency, and privacy requests

```sql
CREATE TABLE lookup_jobs (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  user_id BIGINT UNSIGNED NULL,
  capability_token_hash BINARY(32) NULL,
  request_fingerprint BINARY(32) NOT NULL,
  request_ciphertext VARBINARY(16384) NULL,
  request_nonce BINARY(12) NULL,
  request_key_version SMALLINT UNSIGNED NULL,
  status ENUM('queued', 'running', 'completed', 'failed', 'expired')
    NOT NULL DEFAULT 'queued',
  card_snapshot_id BIGINT UNSIGNED NULL,
  result_ciphertext MEDIUMBLOB NULL,
  result_nonce BINARY(12) NULL,
  result_key_version SMALLINT UNSIGNED NULL,
  error_code VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NULL,
  error_retryable BOOLEAN NULL,
  attempt_count INT UNSIGNED NOT NULL DEFAULT 0,
  available_at DATETIME(6) NOT NULL,
  started_at DATETIME(6) NULL,
  completed_at DATETIME(6) NULL,
  expires_at DATETIME(6) NOT NULL,
  request_erased_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_lookup_jobs_public_id (public_id),
  UNIQUE KEY uq_lookup_jobs_capability (capability_token_hash),
  KEY ix_lookup_jobs_owner_time (user_id, id DESC),
  KEY ix_lookup_jobs_claim (status, available_at, id),
  KEY ix_lookup_jobs_expiry (expires_at),
  CONSTRAINT chk_lookup_jobs_requester CHECK (
    (user_id IS NOT NULL AND capability_token_hash IS NULL)
    OR (user_id IS NULL AND capability_token_hash IS NOT NULL)
  ),
  CONSTRAINT fk_lookup_jobs_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT fk_lookup_jobs_snapshot FOREIGN KEY (card_snapshot_id)
    REFERENCES learning_card_snapshots (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE api_idempotency_records (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  user_id BIGINT UNSIGNED NOT NULL,
  method VARCHAR(8) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  route_template VARCHAR(160) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  idempotency_key_hash BINARY(32) NOT NULL,
  request_hash BINARY(32) NOT NULL,
  status ENUM('in_progress', 'completed') NOT NULL DEFAULT 'in_progress',
  response_status SMALLINT UNSIGNED NULL,
  response_ciphertext MEDIUMBLOB NULL,
  response_nonce BINARY(12) NULL,
  response_key_version SMALLINT UNSIGNED NULL,
  resource_public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  expires_at DATETIME(6) NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_api_idempotency_scope (
    user_id,
    method,
    route_template,
    idempotency_key_hash
  ),
  KEY ix_api_idempotency_expiry (expires_at),
  CONSTRAINT fk_api_idempotency_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE generation_runs (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  purpose VARCHAR(40) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  provider VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  model VARCHAR(160) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  prompt_version VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  schema_version VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  evidence_manifest_json JSON NOT NULL,
  input_token_count INT UNSIGNED NULL,
  output_token_count INT UNSIGNED NULL,
  latency_ms INT UNSIGNED NULL,
  status ENUM('started', 'valid', 'rejected', 'failed') NOT NULL,
  failure_code VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  completed_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_generation_runs_public_id (public_id),
  KEY ix_generation_runs_purpose_time (purpose, created_at)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE outbox_events (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  event_type VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  aggregate_type VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  aggregate_public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  operation_version VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  payload_json JSON NOT NULL,
  status ENUM('pending', 'running', 'done', 'dead') NOT NULL DEFAULT 'pending',
  attempt_count INT UNSIGNED NOT NULL DEFAULT 0,
  available_at DATETIME(6) NOT NULL,
  locked_at DATETIME(6) NULL,
  locked_by VARCHAR(120) CHARACTER SET ascii COLLATE ascii_bin NULL,
  last_error_code VARCHAR(80) CHARACTER SET ascii COLLATE ascii_bin NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  completed_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_outbox_operation (
    event_type,
    aggregate_type,
    aggregate_public_id,
    operation_version
  ),
  KEY ix_outbox_claim (status, available_at, id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;

CREATE TABLE privacy_requests (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  public_id CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  user_id BIGINT UNSIGNED NULL,
  request_type ENUM('export', 'clear_history', 'delete_account') NOT NULL,
  status ENUM('pending', 'running', 'completed', 'failed') NOT NULL DEFAULT 'pending',
  idempotency_key_hash BINARY(32) NOT NULL,
  capability_token_hash BINARY(32) NOT NULL,
  result_object_key_ciphertext VARBINARY(2048) NULL,
  result_object_key_nonce BINARY(12) NULL,
  result_object_key_version SMALLINT UNSIGNED NULL,
  requested_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  completed_at DATETIME(6) NULL,
  result_expires_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY uq_privacy_requests_public_id (public_id),
  UNIQUE KEY uq_privacy_requests_idempotency (user_id, idempotency_key_hash),
  UNIQUE KEY uq_privacy_requests_capability (capability_token_hash),
  KEY ix_privacy_requests_status (status, requested_at),
  CONSTRAINT fk_privacy_requests_user FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_ai_ci;
```

Lookup jobs bind to an authenticated owner or an anonymous capability. Query and context are encrypted for the minimum job lifetime, are absent from the outbox, and are erased after completion or expiry.

Workers claim jobs with leases and `SELECT ... FOR UPDATE SKIP LOCKED`, heartbeat long work, retry transient failures with bounded jitter, and move exhausted jobs to a dead state. Replays are operator controlled and idempotent by event type, entity, and operation version.

Account deletion returns a one-time privacy capability before sessions are revoked. The capability permits status polling after the user record is removed. Export object references and capabilities expire.

## Transactional invariants

### Feedback submission

1. Lock or create the scoped idempotency record.
2. Verify edge public ID, edge version, feedback capability, and ownership context.
3. Insert the immutable feedback event.
4. Replace the current projection for the same `(user, edge, edge version, dimension)`.
5. Insert an aggregate-recompute outbox event.
6. Store the response in the idempotency record and commit.

### Practice submission

1. Lock the owned session and served session item.
2. Resolve the idempotency record and reject a mismatched request hash.
3. Insert one immutable attempt.
4. Mark the item answered and update session counters.
5. Lock and update `(user, focus sense, skill)` mastery.
6. Store the response and commit.

### Content publication

1. Build and validate a staged immutable lexicon release.
2. Build and reconcile the matching vector collection.
3. Record that the pair passed quality and license gates.
4. Update the singleton `active_content_version` pointer in one MySQL transaction.
5. Resolve every request against the exact published pair until a later release.

MySQL and a vector engine do not share a transaction. The active pointer prevents requests from mixing a new database release with an old vector collection during a deployment transition.

### Account deletion

1. Create the privacy request and return its one-time capability.
2. Revoke every session and prevent new personalization writes.
3. Capture affected aggregate relation IDs for recomputation.
4. Delete or anonymize owned history, saved state, layouts, attempts, notes, comments, identifiers, and queued jobs.
5. Remove derived private caches and exports.
6. Recompute required aggregates and complete the privacy request.

## Migration requirements

- Forward migrations and the immediately previous application version must coexist during rolling deployment.
- Destructive column removal follows expand, backfill, switch reads, stop writes, and contract phases.
- Every release records migration compatibility and rollback limits.
- Foreign-key and uniqueness behavior is covered by MySQL integration tests.
- Test migrations include concurrent attempt submission, feedback replacement, outbox claiming, session rotation, history expiry, and deletion.
- Production migrations never derive public IDs or encrypted content inside ad hoc SQL when application-controlled key material is required.

## Related documents

- [System design](../transnet.md)
- [Learning experience](../product/learning-experience.md)
- [Learning API](learning-api.md)
- [Content publishing](../guides/content-publishing.md)
- [Overall plan](../todo.md)
