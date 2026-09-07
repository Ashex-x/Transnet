# Translation prompts

The `prompt` module maps an `InputType` and `TranslationMode` to a provider instruction containing the expected JSON schema. It has no network or parsing responsibilities. The schema text and `format` validation must change together.

Qwen model names select a stricter system prompt and cause the provider adapter to disable thinking through `chat_template_kwargs`. Unsupported combinations retain a diagnostic prompt for Rust API compatibility, although `TranslationService` rejects them before provider I/O.

Prompt output is currently verified indirectly by translation and schema tests. Add focused unit tests here when prompt selection or escaping gains nontrivial branching.
