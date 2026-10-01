# Knowledge application

中文：[知识 application](../../../docs_cn/reference/application/knowledge_cn.md)

This module owns sense resolution, domain assessment, release-pinned retrieval, relationship ranking, page composition, and deterministic projection for word and established-phrase requests.

Status: current canonical lookup uses read-only retrieval ports and assembles each card within the request. Query-derived snapshot caching and its public cache contracts have been removed. Graph topology caching remains release-pinned canonical content caching. Domain assessment and the complete page composition below are target behavior, not default runtime composition.

The canonical-only service selects one authority-owned release pin, then composes reviewed translation candidates, deterministically ranked lexical candidates, and unambiguous sense details through a read port. It forms the existing lookup-card type with a canonical-only content pin; it does not fabricate a vector collection or treat intentional lexical-only operation as vector degradation. The executable constructs and retains this dependency only when explicitly configured probes the active release for readiness, and exposes it through the frozen BasicCard lookup and release-pinned sense routes. The external island-port server still needs the matching internal contract. Request-local lookup spellings are bounded and deduplicated: the baseline normalized form is strongest, and cautious whitespace or surrounding-punctuation variants are lower-priority spelling candidates. Published alias, morphology, transliteration, and semantic membership remains authority-owned rather than guessed from a query string.

The public release-pinned sense follow-up uses the caller's `CanonicalReleasePin` and sense ID without selecting active content again. Source attribution carried by existing candidate/evidence types comes from the authority's reviewed release metadata, never a source-ID-derived label. The HTTP boundary exposes this seam through `POST /api/v1/senses/get`; the application module remains independent of transport DTOs.

The offline `KnowledgePublicationService` is a separate application boundary over `KnowledgePublicationPort`. It validates one immutable release-pinned plan, begins or replays its build intent, reads authoritative status as the only resume-progress source, submits only remaining bounded node batches before node freeze, then remaining edge batches before edge freeze, and finally reconciles the frozen artifacts. It stores no publication progress or cache. Only a successful authority reconciliation may return `PublicationActivationCandidate`; activation and rollback remain external authenticated publisher/control-plane operations through island-port and are never part of the online request runtime.

## Resolution and assessment

Normalization creates bounded language-aware lookup forms but never serves as canonical identity. Resolution uses the closed order exact canonical -> exact published alias -> bounded inflection -> bounded spelling correction -> bounded transliteration -> semantic nomination. A score or vector signal cannot promote a lower class above an eligible higher class. Only candidates in the best available class survive; one sense resolves, multiple distinct senses require clarification or preserved ambiguity, and no candidate is an explicit not-found result. Materially plausible homographs, parts of speech, phrase-level meanings, and field-specific senses remain separate.

Domain assessment uses a bounded published inventory and returns `existing`, `proposed_new`, `general`, or `uncertain`. Inventory failure produces `uncertain`; it cannot prove that a domain is new. Proposals are request-local and appear only where the full response permits them.

## Retrieval and composition

Vector similarity nominates candidates and never establishes a fact. Every displayed canonical node, edge, evidence item, and revision is hydrated from authoritative structured data under the request's release pin. Ranking filters by sense, domain, language, region, period, conditions, and evidence policy without changing identity, direction, provenance, or verification state.

A model may organize supplied facts and write concise explanations, but cannot invent endpoints or change fact metadata. Deterministic validation checks every reference. Empty or weak sections are omitted.

`brief`, `standard`, and `full` are projections of the same validated superset. Response level changes breadth, not fact choice or truth status.

## Verification

Test candidate precedence, ambiguity, closed domain outcomes, unavailable inventory, release consistency, hydration, evidence filtering, graph bounds, hallucinated references, empty sections, and projection monotonicity.
