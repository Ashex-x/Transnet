# Translation Types

This document defines **translation input types**, how the backend auto-detects them, and the JSON types for the `translation` field in the response. It complements:
- `architecture.md` — workflows, data stores, and RAG.
- `api.md` — public web server API.
- `deployment.md` — backend translation service API.

---

## Overview

The backend **automatically classifies** input text into one of five input types (`word`, `phrase`, `sentence`, `paragraph`, `essay`) using heuristics based on length, structure, and language-specific patterns.

The detected `input_type` is **always returned** in the translation response.

The `translation` field in the response has one of **9 possible types**, depending on the `(input_type, mode)` combination.

---

## Translation Field Types

### 1. TranslationWordBasic

Used for `input_type = "word"` + `mode = "basic"`.

```json
{
  "headword": "fox",
  "part_of_speech": "noun",
  "phonetic": "/fɒks/",
  "translations": ["zorro", "raposa"],
  "synonyms": ["zorra", "lobo"],
  "antonyms": [],
  "examples": [
    {
      "source": "The quick brown fox",
      "translation": "El rápido zorro marrón"
    }
  ]
}
```

---

### 2. TranslationWordExplain

Used for `input_type = "word"` + `mode = "explain"`.

```json
{
  "headword": "fox",
  "part_of_speech": "noun",
  "phonetic": "/fɒks/",
  "translations": ["zorro", "raposa"],
  "synonyms": ["zorra", "lobo"],
  "antonyms": [],
  "examples": [
    {
      "source": "The quick brown fox",
      "translation": "El rápido zorro marrón"
    }
  ],
  "explain": {
    "meaning": "A small carnivorous mammal of the dog family, typically with a pointed snout and bushy tail.",
    "story": "The word 'fox' comes from Old English 'fox', related to German 'Fuchs' and Dutch 'vos'.",
    "when_to_use": "Use when referring to the animal, or metaphorically for someone clever or cunning.",
    "how_to_use": "El zorro es astuto. (The fox is clever.)",
    "context": "Common in nature, fables, and idiomatic expressions.",
    "lexical_analysis": {
      "root": "Old English 'fox'",
      "related_phrases": ["fox hole", "fox trot", "outfox"]
    }
  }
}
```

---

### 3. TranslationWordFullAnalysis

Used for `input_type = "word"` + `mode = "full_analysis"`.

```json
{
  "headword": "fox",
  "part_of_speech": "noun",
  "phonetic": "/fɒks/",
  "translations": ["zorro", "raposa"],
  "synonyms": ["zorra", "lobo"],
  "antonyms": [],
  "examples": [
    {
      "source": "The quick brown fox",
      "translation": "El rápido zorro marrón"
    }
  ],
  "explain": {
    "meaning": "A small carnivorous mammal of the dog family, typically with a pointed snout and bushy tail.",
    "story": "The word 'fox' comes from Old English 'fox', related to German 'Fuchs' and Dutch 'vos'.",
    "when_to_use": "Use when referring to the animal, or metaphorically for someone clever or cunning.",
    "how_to_use": "El zorro es astuto. (The fox is clever.)",
    "context": "Common in nature, fables, and idiomatic expressions.",
    "lexical_analysis": {
      "root": "Old English 'fox'",
      "related_phrases": ["fox hole", "fox trot", "outfox"]
    }
  },
  "relationships": {
    "related_words": [
      {
        "word": "wolf",
        "type": "related_animal",
        "similarity": 0.85
      },
      {
        "word": "dog",
        "type": "related_animal",
        "similarity": 0.78
      },
      {
        "word": "cunning",
        "type": "related_concept",
        "similarity": 0.72
      }
    ],
    "by_pos": {
      "nouns": ["wolf", "dog", "animal"],
      "verbs": ["hunt", "chase"],
      "adjectives": ["cunning", "clever"]
    }
  }
}
```

---

### 4. TranslationPhraseBasic

Used for `input_type = "phrase"` + `mode = "basic"`.

```json
{
  "phrase": "kick the bucket",
  "headword": "bucket",
  "part_of_speech": "phrase",
  "translations": ["estirar la pata", "palmar la"],
  "examples": [
    {
      "source": "Old Mr. Smith finally kicked the bucket.",
      "translation": "El viejo Sr. Smith finalmente estiró la pata."
    }
  ]
}
```

---

### 5. TranslationPhraseExplain

Used for `input_type = "phrase"` + `mode = "explain"`.

```json
{
  "phrase": "kick the bucket",
  "headword": "bucket",
  "part_of_speech": "idiom",
  "translations": ["estirar la pata", "palmar la"],
  "examples": [
    {
      "source": "Old Mr. Smith finally kicked the bucket.",
      "translation": "El viejo Sr. Smith finalmente estiró la pata."
    }
  ],
  "explain": {
    "meaning": "To die, often used informally or euphemistically.",
    "story": "Origin unclear; possibly from the motion of kicking over a bucket when slaughtering pigs, or from suicide by standing on a bucket and kicking it away.",
    "when_to_use": "Use in informal contexts when referring to death, typically with a humorous or resigned tone.",
    "how_to_use": "Mi abuelo estiró la pata ayer. (My grandfather kicked the bucket yesterday.)",
    "context": "Informal, idiomatic, euphemistic.",
    "lexical_analysis": {
      "structure": "verb + article + noun",
      "idiomatic": true,
      "related_phrases": ["buy the farm", "six feet under", "push up daisies"]
    }
  }
}
```

---

### 6. TranslationPhraseFullAnalysis

Used for `input_type = "phrase"` + `mode = "full_analysis"`.

```json
{
  "phrase": "kick the bucket",
  "headword": "bucket",
  "part_of_speech": "idiom",
  "translations": ["estirar la pata", "palmar la"],
  "examples": [
    {
      "source": "Old Mr. Smith finally kicked the bucket.",
      "translation": "El viejo Sr. Smith finalmente estiró la pata."
    }
  ],
  "explain": {
    "meaning": "To die, often used informally or euphemistically.",
    "story": "Origin unclear; possibly from the motion of kicking over a bucket when slaughtering pigs, or from suicide by standing on a bucket and kicking it away.",
    "when_to_use": "Use in informal contexts when referring to death, typically with a humorous or resigned tone.",
    "how_to_use": "Mi abuelo estiró la pata ayer. (My grandfather kicked the bucket yesterday.)",
    "context": "Informal, idiomatic, euphemistic.",
    "lexical_analysis": {
      "structure": "verb + article + noun",
      "idiomatic": true,
      "related_phrases": ["buy the farm", "six feet under", "push up daisies"]
    }
  },
  "relationships": {
    "related_phrases": [
      {
        "phrase": "buy the farm",
        "type": "idiom_death",
        "similarity": 0.88
      },
      {
        "phrase": "push up daisies",
        "type": "idiom_death",
        "similarity": 0.82
      },
      {
        "phrase": "bite the dust",
        "type": "idiom_death",
        "similarity": 0.79
      }
    ],
    "related_concepts": ["death", "informal", "euphemism", "idiom"]
  }
}
```

---

### 7. TranslationSentenceBasic

Used for `input_type = "sentence"` + `mode = "basic"`.

```json
{
  "tone": "neutral",
  "rephrasing": "El astuto zorro marrón salta por encima del perro perezoso."
}
```

---

### 8. TranslationSentenceExplain

Used for `input_type = "sentence"` + `mode = "explain"`.

```json
{
  "tone": "neutral",
  "rephrasing": "El astuto zorro marrón salta por encima del perro perezoso.",
  "explain": {
    "meaning": "The sentence describes a fox jumping over a lazy dog, a pangram containing all letters of the English alphabet.",
    "usage": "This is a well-known sentence used for testing fonts and typing practice.",
    "context": "Often used in computing and design contexts as a sample sentence."
  }
}
```

---

### 9. TranslationParagraphEssayBasic

Used for `input_type = "paragraph"` OR `input_type = "essay"` + `mode = "basic"` (both share the same translation-only structure).

```json
{
  "text": "The quick brown fox jumps over the lazy dog. The dog doesn't seem to mind.",
  "translation": "El rápido zorro marrón salta sobre el perro perezoso. Al perro no parece importarle."
}
```

**Note**: This type has NO additional fields. Paragraph and essay are translation-only (no explain/full_analysis).

---

## Type Mapping Table

| Input Type  | Mode                | Translation Type                 |
| ----------- | ------------------- | -------------------------------- |
| `word`      | `basic`             | `TranslationWordBasic`           |
| `word`      | `explain`           | `TranslationWordExplain`         |
| `word`      | `full_analysis`     | `TranslationWordFullAnalysis`    |
| `phrase`    | `basic`             | `TranslationPhraseBasic`         |
| `phrase`    | `explain`           | `TranslationPhraseExplain`       |
| `phrase`    | `full_analysis`     | `TranslationPhraseFullAnalysis`  |
| `sentence`  | `basic`             | `TranslationSentenceBasic`       |
| `sentence`  | `explain`           | `TranslationSentenceExplain`     |
| `paragraph` | `basic`             | `TranslationParagraphEssayBasic` |
| `essay`     | `basic`             | `TranslationParagraphEssayBasic` |

---

## Detection Algorithm

The backend applies the following detection logic **in order**:

1. **Tokenize** input text using language-specific tokenization.
2. **Count words** and characters.
3. **Check structure** for punctuation, line breaks, paragraph markers.
4. **Classify**:
   - If `word_count == 1` → `word`
   - Else if `2 <= word_count <= 8` and `character_length <= 80` → `phrase`
   - Else if `3 <= word_count <= 40` and `character_length <= 300` and has sentence-ending punctuation → `sentence`
   - Else if `word_count <= 500` and `character_length <= 4000` → `paragraph`
   - Else → `essay`

**Language-specific considerations**:
- For languages without word boundaries (e.g., Chinese, Japanese), use character count or specialized segmentation (e.g., CJK tokenization) as a proxy.
- Adjust thresholds based on language where typical word/sentence lengths differ significantly from Indo-European languages.

---

## Unsupported Combinations

The following (`input_type`, `mode`) combinations MUST return HTTP `422`:

| Input Type  | Invalid Mode               |
| ----------- | -------------------------- |
| `sentence`  | `full_analysis`            |
| `paragraph` | `explain`, `full_analysis` |
| `essay`     | `explain`, `full_analysis` |
