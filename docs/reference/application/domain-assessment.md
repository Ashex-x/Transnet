# Domain assessment

中文：[领域评估](../../../docs_cn/reference/application/domain-assessment_cn.md)

The domain-assessment application module determines whether a resolved meaning benefits from domain-specific expansion and records one closed request-local resolution outcome.

Status: target design. Domain types and storage contracts exist, but the current executable does not retrieve the production domain inventory or compose this assessment path.

## Responsibilities

The module first retrieves a bounded, release-pinned inventory containing stable domain IDs, multilingual names and aliases, definitions, inclusion and exclusion scopes, broader domains, and knowledge coverage. A model may choose only IDs in that allowlist.

The deterministic result is `existing`, `proposed_new`, `general`, or `uncertain`. Selecting no supplied ID with a valid structured explanation may produce a request-local proposal; inventory failure always produces `uncertain`. A proposal has no canonical domain ID and appears only where the full projection permits it.

## Dependencies and invariants

The module uses the structured-data read port and, when useful, a bounded structured model operation. It neither searches an unrestricted model-generated catalog nor calls a live create-domain operation.

Familiar words used technically may select a domain, while weakly supported term-like strings remain general or uncertain. Proposals and model reasoning are discarded with the request and never enter MySQL or Qdrant outside the separate reviewed publication workflow.

## Verification

Tests should cover allowlisted existing selections, general meanings, legitimate proposals, weak evidence, overlapping scopes, multilingual labels, and unavailable or incompatible inventories. They must prove that unknown model IDs are rejected and dependency failure cannot masquerade as `proposed_new`.

## Related documents

- [Service module reference](../modules.md)
- [SQL data endpoints](../../interfaces/mysql.md)
- [System design](../../transnet.md)
- [Quality assurance](../../guides/quality-assurance.md)
