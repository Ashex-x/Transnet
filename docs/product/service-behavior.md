# Transnet service behavior

中文：[Transnet 服务行为](../../docs_cn/product/service-behavior_cn.md)

This guide describes consumer-visible behavior of the stateless target service. Product applications own users, file ingestion, saves, document state, and presentation. Current runtime coverage remains narrower than this target and is identified by the [service interface](../interfaces/transnet.md).

## Translation

`POST /api/v1/translations` is the only new-turn entry point. The simple path asks for text, source language, target language, and `brief`, `standard`, or `full`. Professional callers may instead provide ordered document/localization segments or bounded image regions plus optional purpose, audience, register, terminology, alternatives, annotations, and freshness guidance. Every advanced value is request-scoped and creates no profile or translation memory.

Transnet automatically selects lexical, connected-text, structured-segment, or visual-region processing. A canonical exact result may need no generation. Ordinary work uses the fast profile of the one Gemma4-27B VLM; long content uses bounded chunks on the same model. Reasoning may escalate once only under the closed ambiguity, constraint, verified-path, or invalid-structure policy.

Text results return ordered meaning-specific translations. Segment and image results preserve request IDs, order, protected content, and formatting constraints. Typed annotations explain only material ambiguity, terminology, register, culture, formatting risk, or review need. Response level changes supporting breadth, not the selected translation, evidence state, constraints, or review outcome.

## Vision and documents

Island-port uploads files, validates them, renders PDFs, selects pages, and reconstructs output documents. Transnet receives only bounded inline PNG, JPEG, or WebP images with normalized regions, or already extracted structured segments. It never downloads a caller URL or stores file bytes, OCR-like output, or layout state.

Protected ranges must round-trip unchanged. Contradictory required terminology, protected content, or format rules fail explicitly rather than being silently ignored. Oversized documents are divided by island-port into bounded synchronous requests; request-local terminology guidance provides continuity without creating a durable job.

## Current information

Internet retrieval is disabled by default. `allowed` and `required` freshness are explicit opt-in because a derived query may disclose request material to an external search provider. Retrieval is one bounded search/fetch operation with strict public-network, redirect, media-type, byte, and deadline controls.

Live pages are untrusted and cannot change instructions or become canonical facts. Current claims cite response-local `live_external` sources and are discarded after the request. A live result never enters publication automatically.

## Lexical and concept detail

Words, established phrases, and specialist terms resolve to canonical senses or concepts. Homographs, parts of speech, phrase-level meanings, and field-specific senses remain separate. The initial result advertises relevant knowledge lenses rather than exposing graph controls.

`knowledge/views` presents a guided root-specific tree projection for meaning, contrast, usage, form, origin, domain, mechanism, or application. `knowledge/paths` returns only short, independently verified paths between canonical roots. Callers do not choose raw relation filters, depth, node limits, vector selectors, or arbitrary traversal.

The canonical source is an assertion graph, not a strict tree. A stable node may appear under several lenses, but every item retains one identity, an explicit path to the root, a relevance reason, applicability conditions, evidence, provenance, and release. Verified, inferred, exploratory, and live-external material remain visibly distinct.

## Content and degradation

Canonical-data reads are authoritative for facts, evidence, translations, and releases. Retrieval-data reads nominate nodes and relationships through the embedding projection. Similarity never establishes translation, synonymy, taxonomy, causality, mechanism, cultural meaning, or truth.

Unavailable retrieval may degrade to an explicit canonical basic card. Missing authoritative content, an incompatible release, or a failed required live retrieval never masquerades as an empty result. Live requests do not write aliases, cards, facts, domains, assertions, vectors, or releases.

## Related documents

- [System design](../transnet.md)
- [Transnet service interface](../interfaces/transnet.md)
- [Canonical-data interface](../interfaces/canonical-data.md)
- [Retrieval-data interface](../interfaces/retrieval-data.md)
- [Model runtime](../reference/model-runtime.md)
