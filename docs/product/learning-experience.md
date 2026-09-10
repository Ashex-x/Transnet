# English-learning experience

## Status

This document defines the target learner-facing behavior for Transnet basic core. A model-only structured lookup now implements the initial card fields, but canonical evidence, persistence, graph exploration, feedback, and practice remain proposed. [System design](../transnet.md) owns architecture; [learning API](../reference/learning-api.md) owns HTTP contracts.

## Audience and language behavior

Transnet is designed for people learning English from a supported source language. The learner chooses an explanation language, preferred English dialect, and approximate CEFR level. The explanation language can differ from the language of the current query.

Basic-core behavior:

- A non-English word returns matching source-language senses and natural English equivalents.
- An English word enters learner-dictionary mode instead of pretending to translate English to English.
- An inflected form resolves to its lemma while preserving the queried form, such as `went` to `go`.
- A supported idiom, phrasal verb, or short expression resolves as one lexical unit.
- An optional context sentence reranks senses but does not remove other plausible senses.
- A misspelling produces explicit ranked suggestions and never silently replaces the learner's text.
- A configured romanization system can resolve to the original script while showing both forms.
- Low-confidence language detection returns alternatives or asks the learner to choose.
- Sensitive vocabulary receives a neutral explanation and visible usage warning appropriate to the learner's settings.

The first release enables only languages with approved lexical sources, morphology support, and native-speaker evaluation. Adding a language is a content and quality release, not merely enabling a language code.

## Core journeys

### Look up and understand

1. The learner submits a word or short expression and optionally provides a source language and context sentence.
2. Transnet resolves the language, form, lexeme, part of speech, and possible senses.
3. The result keeps homographs and senses separate and ranks them by context and learner relevance.
4. The learner receives an evidence-backed English learning card with explicit coverage and provenance.
5. An authenticated learner may save a selected sense or retain the lookup in private history.

### Explore related language

1. The learner opens a sense in the graph or accessible list view.
2. The client requests a bounded neighborhood filtered by relationship, level, language, or part of speech.
3. The learner can expand nodes and drag a 3D layout without changing semantic data.
4. The learner can rate an edge as more or less personally useful or report a factual issue.
5. Personal ranking updates immediately; eligible factual reports contribute asynchronously to community review.

### Practice and review

1. The learner starts an adaptive session or accepts a practice suggestion after a lookup.
2. The scheduler selects due senses and weak skills rather than only recent spellings.
3. The learner submits one answer to a frozen exercise instance.
4. The server evaluates the answer and updates mastery exactly once.
5. The explanation links back to the selected sense and underlying evidence.

## Learning card

Every result is grouped by lexical entry and then by sense. “Show all” means all supported analyses within response limits; uncommon senses use progressive disclosure and cursor pagination.

| Section | Learner-facing content |
| --- | --- |
| Query analysis | Original and normalized form, language, script, lemma, morphology, confidence, and spelling suggestions |
| Headword | English lemma, syllables, pronunciation, licensed audio, spelling variants, and dialect |
| Parts of speech | Every supported part of speech ordered by relevance and source-qualified frequency |
| Forms | Plural, tense, participle, comparative, superlative, irregular forms, and derivational family links |
| Senses | Plain-English definition, localized gloss, domain, CEFR estimate, frequency band, and confidence |
| Usage | Formality, register, dialect, connotation, politeness, sensitivity, countability, transitivity, and position |
| Grammar | Complement frames, prepositions, articles, clause patterns, and other construction requirements |
| Collocations | Strong word combinations with grammatical roles, construction, restrictions, and examples |
| Examples | Short natural examples appropriate to the learner's level, with localized explanations when useful |
| Pitfalls | False friends, confusable words, unnatural literal translations, and common learner errors |
| Word history | Sourced etymology and semantic development, including uncertainty and date ranges |
| Learner history | Private lookups, selected sense, save state, and practice summary when enabled |
| Relations | Synonyms, near-synonyms, antonyms, broader/narrower terms, word families, confusables, associations, and scales |
| Practice | Suggested skills and current mastery for the selected sense |
| Provenance | Assertion-level evidence, source permissions, content release, and generation versions |

Each section reports one of these coverage states:

- `available`: requested and adequately supported.
- `partial`: requested but only partly supported.
- `unavailable`: requested but no suitable evidence exists.
- `disputed`: reputable sources conflict.
- `not_requested`: omitted by the request.
- `blocked_by_policy`: omitted because display is not permitted.
- `temporarily_unavailable`: a dependency failed and a fallback could not provide the section.

Generated examples, simplified explanations, and mnemonics are visibly marked. Missing fields are never invented to make a card look complete.

## Sense and part-of-speech behavior

Learning state and semantic relationships attach to a sense, not merely a spelling. Adjective `hot`, adverb `hotly`, and noun `heat` are separate lexemes connected by lexical relations. `Hot` meaning “high temperature” and `hot` meaning “currently popular” are separate senses with different examples and graph edges.

A card orders common and context-relevant senses first. Rare, dated, technical, regional, and offensive senses remain discoverable when evidence and policy permit them, but do not overwhelm the initial view.

## Usage guidance

Usage guidance is descriptive rather than prescriptive. Every register, dialect, frequency, CEFR, pronunciation, or “common mistake” claim names its source scope and version.

A near-synonym must include a contrast explaining when the words are not interchangeable. A dialect variant is not labeled incorrect merely because it differs from the configured target dialect. Sensitive words are explained faithfully without demeaning examples.

For beginners, cards show a localized gloss and short plain-English definition. For advanced learners, English-first definitions, nuance, contrast, collocation, and register become more prominent. The learner can always reveal or hide first-language scaffolding.

## Relationship exploration

The graph presents these families:

- Semantic equivalence: `synonym`, `near_synonym`, and `translation_equivalent`.
- Opposition: `antonym`.
- Hierarchy: broader `hypernym` and narrower `hyponym`.
- Whole and part: `holonym` and `meronym`.
- Learning difficulty: `confusable_with`.
- Topic: `associated_with`, clearly weaker than a lexical claim.
- Lexical family: `inflection_of`, `derivationally_related_to`, and `etymologically_derived_from`.
- Construction: collocation projections backed by structured grammatical records.
- Degree: ordered scale members projected as lower/higher neighbors.

Temperature words such as `cool < warm < hot < scorching` belong to a named, context-qualified scale. They are not a parent-child hierarchy. The server stores one ordered scale and projects adjacent edges for display.

The 3D graph is optional. A list or tree view exposes the same relations with keyboard navigation, reduced-motion support, text labels, and meaning that does not depend on color.

Dragging a node changes only client layout. An explicit save action may store private coordinates and camera state for the same root, filters, content release, and layout algorithm version. Spatial distance and coordinates are presentation hints, not lexical evidence.

## Relationship feedback

The UI separates personal usefulness from factual accuracy:

- Usefulness accepts `more`, `less`, or `reset` and affects only the learner's ranking.
- Accuracy accepts `accurate`, `wrong_sense`, `wrong_type`, `too_broad`, `missing_restriction`, `unsupported`, `unsure`, or `reset`.
- `unsure` is an abstention, not a negative vote.

A feedback-enabled graph edge exposes its canonical relation version. A newer relation version does not inherit an old judgment silently. Derived scale edges and visual-only edges are not directly voteable.

Community aggregation uses eligible current accuracy reports, capped trust weights, shrinkage toward neutral, a minimum number of distinct voters, abuse detection, and reversible moderator decisions. Feedback never directly changes source evidence, relation type, embeddings, or canonical status.

## Practice skills

| Skill | Example | Basic-core priority |
| --- | --- | --- |
| Meaning recognition | Choose the English sense for a localized cue in context | First |
| Sense discrimination | Select which sense fits an unseen sentence | First |
| English recall | Produce the English lemma from a localized meaning | First |
| Spelling and form | Produce an inflection or recognize an irregular form | First |
| Collocation | Fill the dependent word in a natural construction | First |
| Grammar pattern | Supply a preposition, article, count form, or complement | First |
| Register choice | Choose language suitable for the social context | Later |
| Contrast | Distinguish a near-synonym or confusable word | Later |
| Free production | Write a sentence satisfying a sense and usage restriction | Later |
| Listening/pronunciation | Recognize or produce a dialect-aware form | Later |

Mastery is tracked per `(learner, sense, skill)`. Recognition does not imply recall, correct inflection, natural collocation, or appropriate register.

## Exercise behavior

- Prefer curated items and deterministic templates over generation.
- Generate reusable candidates asynchronously from selected evidence.
- Require one intended answer or an explicit accepted-answer set.
- Reject ambiguous distractors and answer leakage.
- Freeze focus sense, secondary targets, prompt language, dialect, level, content release, generator, model, prompt, normalization, rubric, and evaluator assumptions.
- Hide answers until submission and accept harmless capitalization or punctuation differences.
- Use correctness and hints as the primary scheduling signals.
- Give response time only a capped learner-relative influence, and allow accessibility settings to disable it.
- Do not reduce mastery when free-form evaluation is uncertain.
- Explain the correct sense, grammar, collocation, or usage distinction after submission.
- Test transfer with unseen contexts instead of memorization of one example.

Raw answers are retained only for a short correction or dispute window and are then redacted while scores, misconception categories, and scheduler effects remain.

## Personalization

Personalization uses the minimum useful inputs: explanation language, known languages, English level, target dialect, active goal, optional interests, accessibility settings, recent sense choices, and skill mastery. Age is not collected unless an age-specific product and compliance design are approved.

Recommendations balance due review, prerequisite vocabulary, useful graph neighbors, and learner goals. Each recommendation gives a short reason such as “due for recall” or “helps distinguish two words you confused.” Personal dislike cannot hide a necessary correction, and community popularity cannot crowd out core or low-resource-language vocabulary.

Turning personalization off returns the common evidence-based order without deleting learning history. Personalized decisions record a version so behavior can be evaluated and reproduced.

## History and privacy controls

Word history and learner history are distinct. Word history is public sourced lexical content; learner history is private user-owned data.

History is disabled until the learner makes an informed choice. An authenticated learner can choose a retention period, use incognito per lookup, delete one event, clear all history, export data, or delete the account. Context sentences are not retained as history.

An incognito lookup has no lookup-history ID. If a client requests saved history while history is disabled, the lookup still succeeds incognito and reports `history_not_saved`.

Private queries, context, answers, notes, and comments never appear in logs or shared vector indexes. Learner data is not used for model training without a separate informed opt-in.

## Success measures

The primary product outcome is delayed recall and correct use of a sense in unseen context, not lookup count or time spent in the application.

Supporting measures include successful sense selection, useful card coverage, graph-to-practice conversion, answerability, mastery improvement by skill, seven-day and thirty-day retention, and quality parity across enabled languages and English dialects.

## Related documents

- [System design](../transnet.md)
- [Learning API](../reference/learning-api.md)
- [MySQL schema](../reference/mysql-schema.md)
- [Quality assurance](../guides/quality-assurance.md)
- [Overall plan](../todo.md)
