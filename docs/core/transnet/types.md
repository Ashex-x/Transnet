# Translation types

The `types` module owns the transport-independent request, response, configuration, classification, and domain-error contracts. Provider-specific envelopes remain private to `llm`.

Automatic classification applies ordered heuristics: one word becomes `word`; short unpunctuated text becomes `phrase`; a bounded single sentence becomes `sentence`; text within 500 words and 4,000 Unicode scalar values becomes `paragraph`; longer text becomes `essay`. Empty text classifies as a paragraph but is rejected before provider I/O.

Translation IDs are process-local monotonic counters. They reset when the server restarts and must not be used as durable persistence identifiers.

Unit tests cover each classification boundary category. Run `cargo test -p transnet types::tests`.
