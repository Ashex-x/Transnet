# English-learning experience

中文：[英语学习体验](../../docs_cn/product/learning-experience_cn.md)

This document translates the [system design](../transnet.md) into learner-facing behavior. The system design owns product semantics; the [Island-port interface](../interfaces/port.md) owns transport details.

Status: target product behavior. The current runtime implements translation and a structured lexical-lookup subset.

## Audience and language behavior

Transnet supports adult English learners from CEFR A1 through C2. English is always the target language. The learner may choose an explanation language, English dialect, and amount of first-language support, but those presentation settings are not evidence of level, interests, or mastery.

A word, term, idiom, phrasal verb, or established lexical phrase opens a translation-wiki page. A clause, sentence, or passage returns a translation first. A short ambiguous fragment receives the simpler translation response unless the router confidently recognizes a lexical unit.

Inflected forms resolve to their lemma while preserving the queried form. Homographs, parts of speech, and senses remain distinct. Misspellings produce explicit suggestions, low-confidence language detection exposes alternatives, and sensitive vocabulary receives neutral, scoped usage guidance.

## Core journeys

### Translate a sentence or passage

1. The learner submits text and optional language, dialect, and register preferences.
2. The agent preserves meaning, tone, register, and paragraph structure in natural target-language wording.
3. The response contains only the translation unless an ambiguity, idiom, consequential register choice, or cultural context merits a tip.
4. At most two one-sentence tips appear; detailed lexical teaching requires a separate request.

### Look up and explore a lexical unit

1. The agent resolves the query to a canonical MySQL basic card and selected sense.
2. It follows the card's Qdrant root ID and retrieves a bounded set of eligible nodes and typed edges.
3. The translation-wiki page presents the concise card first, followed only by well-supported sections relevant to that sense.
4. Verified relationships remain separate from embedding-only exploratory associations.
5. Selecting a related node expands one bounded neighborhood; the agent does not present a similarity chain as a factual path.

### Bookmark and learn

1. The learner explicitly bookmarks a selected word or phrase sense.
2. The system creates a complete frozen learning-card revision linked to the source basic card and knowledge release.
3. Current bookmarks and at most 200 compact events from the previous 30 days supply the only personal strategy evidence.
4. The scheduler selects due bookmarked cards and relevant transfer tasks by independently demonstrated skills.
5. The learner attempts one bounded activity, receives at most two high-value corrections, retries when useful, and later transfers the skill to a new context.
6. Removing the bookmark stops future scheduling; clearing history immediately removes its strategic influence.

## Translation-wiki page

The basic card supplies the canonical form, sense, part of speech, concise translations and definitions, pronunciation and morphology summaries, CEFR difficulty, domain tags, knowledge-node roots, and release metadata. The page may add relevant taxonomy, intensity scales, valency, collocations, fixed phrases, nuance, connotation, register, morphology, idioms, cultural context, and technical knowledge from Qdrant.

Sections are ordered by usefulness and omitted when empty or weakly supported. Near-synonyms include a contrast. Intensity scales are not represented as taxonomic parents. Register and cultural claims state their applicable dialect, region, period, domain, or social context.

The page distinguishes sourced knowledge, generated teaching material, and uncertain inference. It never promotes vector proximity to translation, synonymy, hierarchy, causation, or shared mechanism.

## Learning card

A learning card exists only after a bookmark. It is a private practice artifact rather than a copy of the full translation-wiki page. Each immutable revision freezes the front, back, example, pronunciation cue, hints, practice prompts, selected targets, inferred generation context, source release, generator, prompt, rubric, and evaluator versions.

Learning state attaches to the selected sense or phrase and tracks only applicable dimensions, including recognition, recall, spelling, morphology, collocation, grammar, sentence composition, writing, register, cultural pragmatics, listening, and pronunciation. Success in one dimension never advances another without evidence.

A new knowledge release does not silently rewrite a learner card. Refreshing creates a traceable revision while preserving compatible review state. A corrected, quarantined, or withdrawn source requires regeneration before the next review.

## Practice and feedback

Practice moves from recognition toward independent production. Activities include meaning and sense selection, English recall, spelling repair, dictation, morphology, collocation, grammar patterns, sentence construction, writing, register choice, cultural communication, listening, and pronunciation.

Every generated activity retains its target, prompt, accepted evidence or rubric, allowed variants, difficulty, hints, and relevant versions. Deterministic checks own exact answers. Open writing and communication use explicit multidimensional rubrics. Pronunciation feedback requires acoustic and alignment evidence rather than transcript text alone.

Free production returns `correct`, `needs_revision`, or `needs_review`. Only a sufficiently confident, evidence-backed result changes mastery. Feedback preserves the learner's intended meaning and voice, distinguishes correctness from naturalness, and does not reject valid dialect or stylistic variation.

## Scheduling and transfer

The scheduler uses a versioned FSRS-style model of difficulty, stability, and retrievability. Objective correctness and hint use are primary signals. Response time has only bounded learner-relative influence and may be disabled for accessibility. Uncertain evaluation does not reduce mastery.

Daily practice combines due bookmarks, unresolved compact misconceptions, and transfer tasks derived from directly relevant knowledge edges. Related nodes are not automatically turned into study targets. Later prompts change wording, content, or social setting so the learner demonstrates transfer instead of memorizing an example.

## Personalization and privacy

Learning strategy is reconstructed from current bookmarks and bounded recent history. It may infer approximate level, domain relevance, weak skills, and review priority with explicit uncertainty. Bookmarks outweigh browsing history, and historical influence decays. Sparse or contradictory evidence produces a neutral general-English strategy.

Explanation language, dialect, and scaffolding preferences control presentation only. Incidental words, writing, conversations, raw answers, passages, and recordings never become durable strategy inputs or study targets. Qdrant contains no private learner data.

Compact history contains canonical IDs, selected sense, action, timestamp, and minimal review outcome, hint count, or misconception category. Raw queries, writing, answers, conversations, generated explanations, and recordings are excluded. Recordings are not retained by default; only the minimum derived result needed for review may remain.

The learner can inspect, pause, reprioritize, refresh, or remove every learning card and can clear short history. The interface must make these controls understandable and must not use learner data for training without a separate informed choice.

## Accessibility and cultural safety

Graph exploration has an equivalent keyboard-accessible list or tree presentation, reduced-motion support, text labels, and meaning that does not depend on color or spatial distance. Audio provides selectable playback speed without mechanically stretching speech, visible generated-voice labeling, and text alternatives.

Cultural coaching describes likely interpretation in a stated relationship, setting, medium, and region. It offers alternatives with different warmth or formality and presents variation and uncertainty instead of universal claims about a group.

## Success measures

The primary outcome is delayed recall and natural use in an unfamiliar context. Supporting measures include successful sense resolution, useful knowledge coverage, bookmark-to-practice conversion, answerability, transfer by skill, seven-day and thirty-day retention, pronunciation assessability, and quality parity across enabled languages and English dialects.

## Related documents

- [System design](../transnet.md)
- [Island-port interface](../interfaces/port.md)
- [MySQL interface](../interfaces/mysql.md)
- [Qdrant interface](../interfaces/qdrant.md)
- [Quality assurance](../guides/quality-assurance.md)
- [Overall plan](../todo.md)
