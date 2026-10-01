# MySQL migration policy

中文：[MySQL 迁移策略](../../../docs_cn/interfaces/tables/migrations_cn.md)

This policy governs future changes to the target MySQL implementation. [`mysql.sql`](mysql.sql) is the reviewed target snapshot, not a production upgrade script. When implementation begins, its exact logical shape is frozen as `000001_baseline.sql`; fresh production installs and upgrades both execute the immutable, monotonically numbered island-port migration chain through the runner.

## Migration identity and ownership

Canonical and product schemas have independent migration sequences and separate `schema_migration` ledgers. A migration filename uses `NNNNNN_description.sql`; its numeric version, name, SHA-256 checksum, and runner version must match the ledger. An applied migration is never edited, reordered, or reused. Corrections use a new migration.

One deployment holds a database advisory lock for the schema, rejects an unknown or changed checksum, and refuses to continue when any ledger row remains `applying`. Because MySQL DDL can commit implicitly, the runner records `applying` before execution and changes it to `applied` only after postconditions pass. Recovery inspects the declared preconditions and postconditions; it never blindly reruns a partially applied file.

## Compatibility window

Every rollout follows expand, migrate, switch, and contract:

1. **Expand:** add nullable columns, new tables, indexes, registry rows, or dual-readable payload versions. Old readers and writers must continue to work.
2. **Migrate:** backfill in bounded, restartable primary-key ranges. Record progress outside request content, throttle load, and verify counts and hashes.
3. **Switch:** deploy writers before readers depend on new data, dual-write only for a bounded documented window, then activate a content release whose manifest declares the new schema capability.
4. **Contract:** remove old reads, fields, indexes, or payload versions only after every supported binary and retained release no longer needs them and the rollback window has expired.

A single migration must not rename and drop a live column, change a column to a narrower type, reinterpret an existing code, rewrite a large table, or add a required field without a backfill/default compatibility phase. Destructive contract migrations require a backup/restore drill and explicit operator approval.

## Extensible registries and immutable data

Entity families and relation semantics are data-driven through `entity_type_revision` and `relation_type_revision`; adding a type does not alter `canonical_entity`. A release pins exact source, evidence, type-registry, entity, and relationship revisions. Published revisions are never updated in place. A correction inserts a new revision and later release membership.

Database `ENUM` values are reserved for small infrastructure state machines whose new states require coordinated code and migration. Extensible product and knowledge taxonomies use registry rows or versioned payload schemas instead of `ENUM` or one column per type.

JSON payload evolution is additive within a schema version. A breaking payload change uses a new `payload_schema_version`; readers support an explicit finite set during the compatibility window. Publishers validate and canonicalize JSON before hashing, so key order or serializer changes cannot silently change identity.

Assessment algorithms and privacy thresholds are versioned row data, not DDL constants. The publisher validates a projection against its aggregate release, including `algorithm_version`, `policy_version`, and `minimum_group_size`, before activation. A new algorithm builds a new immutable aggregate release instead of rewriting an active one.

## Online DDL and backfills

Before execution, capture table size, replica lag, free space, expected lock behavior, and the exact MySQL version. Production `ALTER TABLE` statements declare an acceptable `ALGORITHM` and `LOCK` so they fail instead of silently falling back to a table copy or stronger lock. Prefer instant or in-place additive DDL only after verifying support on that server version. If MySQL would copy a large table or take a long metadata lock, use a reviewed online-schema-change procedure rather than forcing the statement. MySQL atomic DDL protects one supported statement; it is not transactional DDL and does not make a multi-statement migration atomic.

Backfills must be idempotent, resumable, rate-limited, and observable with aggregate counters only. They use stable primary-key pagination, never `OFFSET`, and do not place canonical bodies or private product values in logs. Foreign keys and indexes are added only after orphan and duplicate checks pass.

## Activation, rollback, and removal

Schema deployment does not activate content. Publication first verifies that every serving binary understands the release manifest and pinned payload versions, then reconciles canonical SQL and Qdrant projections before the atomic content-release switch.

Application rollback selects the previous immutable release. Binary rollback is permitted only while the database remains within that binary's declared compatibility range. DDL rollback is normally forward repair; down migrations are allowed only when proven lossless. Dropping a column, table, registry revision, or retained release requires evidence that no supported binary, release, foreign key, outbox command, or audit obligation references it.

## Required verification

Each migration has tests for fresh install, upgrade from the oldest supported schema, interrupted execution and recovery, mixed old/new binaries, bounded backfill, activation, application rollback, and checksum drift. CI also compares a database created by all migrations with the reviewed snapshot, ignoring only documented physical differences such as generated constraint names.

Related: [storage catalog](README.md), [persistence boundaries](../../reference/persistence.md), [content publishing](../../guides/content-publishing.md), [MySQL atomic DDL](https://dev.mysql.com/doc/refman/8.4/en/atomic-ddl.html), and [InnoDB online DDL](https://dev.mysql.com/doc/refman/8.4/en/innodb-online-ddl.html).
