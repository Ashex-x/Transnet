# Transnet Architecture

This document describes the architecture of the Transnet translation system: input analysis, translation types, workflows, and how they map to databases and LLM usage.

**Related Documents**:
- [docs/api.md](api.md) — API contract (web server and backend).
- [docs/database.md](database.md) — Web server (MySQL) and backend data stores.
- [docs/deployment.md](deployment.md) — Backend server deployment.

---

## Overview

Transnet supports multiple **input types** and **translation types**. The pipeline chooses workflows and data stores based on these two dimensions.

**Input types**: word, phrase, sentence, paragraph, essay.

**Translation types**: basic, explain, full_analysis.

Not every combination is supported; the following sections specify which are valid and how they behave.

---

## Input Analysis

The system classifies user input into one of five types. This classification drives which database(s) to query and which translation workflow to run.

| Input type | Description | Stored in DB? | Typical length |
|------------|-------------|---------------|----------------|
| **word**   | Single word (headword). | Yes — word dictionary / explain DB / RAG. | One token. |
| **phrase** | Multi-word phrase or collocation. Keyed by headword(s). | Yes — phrase DB (SQL), keyed by word. | Short fixed expression. |
| **sentence** | One or more clauses; full sentence. | No — not stored in translation DBs. | One sentence. |
| **paragraph** | One or more sentences; medium text. | No (translation output only). | Medium. |
| **essay** | Long-form text (article, essay, document). | No (translation output only). | Long. |

Input analysis (e.g. tokenization, language detection, length heuristics) determines `input_type` before the translation workflow is selected.

---

## Translation Types

| Type | Name | Description |
|------|------|-------------|
| **basic** | Basic | Dictionary/DB lookup when applicable; fallback to LLM. Output: translation plus core metadata (e.g. POS, phonetics, examples). |
| **explain** | Explain | Extends basic with explanatory content: meaning, story/history of the term, when/how to use, suitable context, lexical analysis. Uses a dedicated explain DB (SQL). |
| **full_analysis** | Full | Extends explain with a RAG-based relationship layer: related words/concepts by POS (nouns → related things, verbs → related actions, adj/adv → intensity). Relationships are defined by the LLM and stored in RAG. |

For **sentence**, only basic and explain are offered (no full_analysis). For **paragraph** and **essay**, only translation output is produced (no basic/explain/full); paragraph uses a local LLM, essay uses a cloud LLM (e.g. Qwen).

---

## Word Workflows

### Word — Basic

1. **Lookup**: Search the **word dictionary** by language pair (e.g. en→cn, cn→en). The dictionary is keyed by source/target language and headword.
2. **If found**: Return cached entry. No LLM call.
3. **If not found**:
   - Call **LLM** to translate and to produce: translations, synonyms, antonyms, example sentence(s), part of speech (POS), phonetic symbols.
   - Optionally use an **MCP** so the LLM can search the internet for more information.
   - **LLM decides** whether this result should be stored in the dictionary. If yes, persist the new entry.
4. **Output**: translations, synonym, antonym, example sentence, POS, phonetic symbols.

**Data store**: Word dictionary (e.g. SQL or key-value), keyed by language pair and word.

---

### Word — Explain

1. **Include Basic**: Run the **word — basic** workflow first (dictionary lookup or LLM + optional store).
2. **Explain DB**: Use a separate **explain database** (SQL). Primary key is the **translation word** (or a canonical form). Look up explanatory content.
3. **If missing or incomplete**: Call LLM to generate and then store:
   - The meaning of the word.
   - The story of the word (history, etymology, etc.).
   - When to use it and how to use it.
   - The most suitable context to use this word.
   - **Lexical analysis** — this output is also used to feed or link to the **phrase database** (phrases that contain or relate to this word).
4. **Output**: Everything from basic, plus: meaning, story, when/how to use, context, lexical analysis (and linkage to phrase data).

**Data store**: Explain DB (SQL). Primary key: translation word (or normalized form). Links to phrase DB via lexical analysis.

---

### Word — Full

1. **Include Explain**: Run the **word — explain** workflow first.
2. **RAG relationship DB**: A **RAG database** (e.g. vector store) stores **relationships between words**. Examples: *mother* → father, family, brother, etc. Relationship type depends on **POS**:
   - **Noun (n)**: Related items or things.
   - **Verb (v)**: Related actions or theories.
   - **Adjective / Adverb (adj./adv.)**: Related by different intensity (e.g. hot → warm, scorching).
3. **Relationship content**: The **LLM defines** the relationships (which words/concepts are related and how). These are embedded and stored in RAG; later queries retrieve “related words” by semantic similarity and POS.
4. **Output**: Everything from explain, plus: related words/concepts (by POS).

**Data store**: RAG (e.g. Qdrant). Vectors/keyed by word and POS; metadata holds relationship type and optional labels.

---

## Phrase Workflows

Phrases use a **different database** from words: an SQL store where entries are keyed by **word** (headword). Phrases are stored per headword so that “phrases containing X” can be retrieved quickly.

- **Phrase — basic**: Same idea as word — basic but against the **phrase DB**: lookup by language pair and key word(s). If not found, LLM generates translation and phrase-level metadata; LLM decides whether to store. Output analogous to word basic (translations, POS, examples, etc.) at phrase level.
- **Phrase — explain**: Same as word — explain but for phrases: extend basic with meaning, when/how to use, context, and lexical analysis; primary key in explain DB is the phrase (or canonical form). Lexical analysis can link to other phrases or back to words.
- **Phrase — full**: Same as word — full: include explain workflow plus **relationship RAG** for phrases (related phrases, collocations, or related concepts). Relationships are again defined by the LLM and stored in RAG.

**Data store**: Phrase DB (SQL), primary key word (headword). Explain and RAG for phrases follow the same pattern as for words (separate explain table/DB and RAG collection).

---

## Sentence Workflows

Sentences are **not stored** in the translation databases. No dictionary or phrase lookup for full sentences.

- **Sentence — basic**: Call **LLM directly** (no DB lookup). Output: translation and minimal metadata (e.g. rephrasing, tone) as defined by the API.
- **Sentence — explain**: Call **LLM directly** with a prompt that asks for explanatory content (meaning, usage, context) in addition to translation. Output: translation + explanation.

**No full_analysis** for sentences. **No persistence** of sentence translations into the word/phrase/explain/RAG stores.

---

## Paragraph and Essay

For **paragraph** and **essay**, the system **only outputs the translation** (no basic/explain/full tiers, no dictionary or RAG lookup for the whole text). The difference is which LLM is used:

| Input type | LLM used | Rationale |
|-------------|----------|-----------|
| **Paragraph** | **Server local LLM** | Moderate length; keep latency and cost on local hardware. |
| **Essay** | **Cloud LLM** (e.g. Qwen) | Long-form; use cloud for capacity and quality. |

Input analysis must distinguish paragraph vs essay (e.g. by length or structure) so the backend can route to the correct LLM.

---

## Data Stores Summary

| Store | Type | Key / structure | Used by |
|-------|------|------------------|--------|
| **Word dictionary** | SQL or key-value | Language pair + headword | Word — basic (lookup and optional store). |
| **Explain DB (word)** | SQL | Primary key: translation word | Word — explain; phrase — explain (phrase-level table). |
| **Explain DB (phrase)** | SQL | Primary key: word (headword) for phrase entries | Phrase — explain. |
| **Phrase DB** | SQL | Primary key: word (headword) | Phrase — basic (and explain keying). |
| **RAG (word relationships)** | Vector DB (e.g. Qdrant) | Word + POS; embeddings from LLM-defined relationships | Word — full; phrase — full. |
| **LLM + MCP** | External | — | Word/phrase when not in DB; MCP for internet search when needed. |

The **word dictionary** and **explain DB** are backend-owned (see [database.md](database.md)). The **web server** (MySQL) holds users, history, and favorites; it does not hold the above translation caches or RAG.

---

## LLM and MCP Usage

- **Word / phrase — basic**: LLM used when dictionary/phrase DB miss; LLM decides whether to store the result. Optional **MCP** allows the LLM to search the internet for more information before answering.
- **Word / phrase — explain**: LLM fills explain DB when entry is missing or incomplete; lexical analysis can drive phrase DB linkage.
- **Word / phrase — full**: LLM defines relationships; backend embeds and stores them in RAG.
- **Sentence**: LLM only; no DB read/write for sentences.
- **Paragraph**: Local LLM only; translation output only.
- **Essay**: Cloud LLM (e.g. Qwen) only; translation output only.

---

## Reference Matrix

| Input type | basic | explain | full_analysis |
|------------|-------|---------|----------------|
| **word**   | ✅ Dictionary → LLM (optional MCP); store if LLM says so. Output: translation, synonym, antonym, example, POS, phonetics. | ✅ Basic + explain DB (meaning, story, when/how, context, lexical analysis → phrase). | ✅ Explain + RAG relationships by POS (n/v/adj/adv). |
| **phrase** | ✅ Phrase DB (keyed by word) → LLM; store if LLM says so. | ✅ Basic + explain DB for phrases. | ✅ Explain + RAG for phrase relationships. |
| **sentence** | ✅ LLM only; no DB. | ✅ LLM only; no DB. | ❌ Not supported. |
| **paragraph** | Translation only; **local LLM**. | — | — |
| **essay** | Translation only; **cloud LLM** (e.g. Qwen). | — | — |

---

## References

- [docs/api.md](api.md) — Public web server API.
- [docs/deployment.md](deployment.md) — Backend translation service API.
- [docs/types.md](types.md) — Translation input types, auto-detection criteria, and mode definitions.
- [docs/database.md](database.md) — Web server MySQL; backend RAG/DB.
