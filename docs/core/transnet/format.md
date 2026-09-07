# Provider response format

The `format` module extracts JSON from raw provider content and verifies the response schema selected by input type and mode. It accepts direct JSON, fenced JSON, and prose containing an object. Validation is structural and deliberately leaves translated values to the provider.

`parse_llm_response` fails when content contains no complete JSON object or the extracted content is invalid JSON. `validate_translation_structure` fails when required fields are missing, have the wrong JSON type, or represent an unsupported type/mode combination.

Unit tests cover raw and fenced parsing, representative word, sentence, and paragraph schemas, missing fields, and unsupported combinations. Run `cargo test -p transnet format::tests`.
