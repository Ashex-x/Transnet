# LLM translation service

## Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [API](#api)
- [Design](#design)
- [Testing](#testing)

## Overview

The `llm` module is the translation orchestrator and OpenAI-compatible provider adapter. It owns caller validation, provider and model selection, retry behavior, response-envelope decoding, and schema validation. HTTP routes and durable storage are outside this module.

## Architecture

```mermaid
flowchart LR
  request["TranslateRequest"] --> validate["Validate and classify"]
  validate --> select["Select provider and prompt"]
  select --> call["OpenAI-compatible request"]
  call --> parse["Parse and validate JSON"]
  parse --> response["TranslateResponse"]
  call -->|"failure"| retry["250 ms retry"]
  retry --> call
```

## API

`TranslationService::new` creates a reusable client with the configured timeout. `TranslationService::translate` accepts ownership of a `TranslateRequest` and returns a `TranslateResponse` or `TransnetError`.

A call makes at most `max_retries + 1` provider attempts. Validation errors do not perform provider I/O. Provider transport, HTTP status, envelope parsing, content parsing, and schema failures become `TransnetError::Llm` after attempts are exhausted.

## Design

Translations where both language values match the common-language allowlist use `normal_lang_base_url` and `normal_lang_model` when configured; otherwise they use the default provider. Matching is case-insensitive and accepts configured names and short codes.

Logs contain endpoint, model, attempt, and error metadata. Source text, response content, and bearer credentials are intentionally excluded because they may contain sensitive user or provider data.

Retries use a fixed 250 ms delay. This is intentionally simple for the local-service MVP; exponential backoff, jitter, and status-aware retry policy should be introduced before targeting a shared remote provider.

## Testing

Current unit tests cover helper selection and validation without network access. Run `cargo test -p transnet llm::tests`. Stubbed HTTP integration tests remain roadmap work and should cover retries, error-status redaction, empty choices, malformed envelopes, and schema failures.
