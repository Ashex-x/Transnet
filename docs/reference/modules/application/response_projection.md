# Response projection

中文：[响应投影](../../../../docs_cn/reference/modules/application/response_projection_cn.md)

The response-projection application module deterministically reduces one validated superset translation aggregate to the requested `brief`, `standard`, or `full` representation.

Status: target design. The current routes do not yet share the target superset or this deterministic projector.

## Projection policy

`brief` keeps unit, detected source language, ordered translations, and meaning labels needed to avoid ambiguity. `standard` adds selected concise definitions or phrase usage, at most one example per meaning, at most two passage tips, and only material relationships. `full` adds all bounded eligible lexical, domain, evidence, taxonomy, and intensity detail.

Projection uses versioned field and item allowlists. Empty fields and groups are omitted. A request-local proposed domain is eligible only at `full` level.

## Invariants

All levels are subsets of the same release-pinned superset except for envelope metadata. A lower level never changes meaning order, canonical IDs, translation text, evidence or verification states, degradation signals, or selected facts. Materially plausible meanings remain visible when omission would mislead.

The projector performs no provider call, retrieval, reranking, persistence, or wire parsing. Serialization remains the API response module's responsibility.

## Verification

Golden and property tests should compare all three projections from the same aggregate, assert strict field-and-item subset behavior, enforce example and tip limits, retain ambiguity, and omit empty groups. Tests should pin the projection-policy identifier so a policy change is explicit and reviewable.

## Related documents

- [Service module reference](../../modules.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [System design](../../../transnet.md)
- [Quality assurance](../../../guides/quality-assurance.md)
