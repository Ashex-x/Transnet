# Offline publication

中文：[离线发布](../../../docs_cn/reference/operations/publication_cn.md)

This module owns the offline path that turns reviewed content into an immutable compatible release.

## Workflow

The publisher stages structured content, validates schema and rights, checks evidence and deterministic invariants, writes authoritative revisions, projects immutable vector node and edge collections, reconciles exact counts and release identifiers, evaluates quality gates, and activates the release atomically. Quarantine, removal, and rollback preserve auditability and compatibility.

The publisher is a separate composition root and the only component allowed mutation-capable structured and vector ports. Generated candidates are not evidence. They become canonical only after the configured evidence, rights, deterministic validation, and review policy succeeds.

## Online boundary

Live translation and lookup requests never invoke publication, create durable proposals, or write their text or output to canonical storage. Request-local gaps and proposed domains disappear with the request unless an independently authorized offline process deliberately imports reviewed material.

Operational commands, manifests, rollback rules, and verification gates belong to the [content-publishing guide](../../guides/content-publishing.md).
