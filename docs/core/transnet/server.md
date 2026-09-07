# Transnet server

The `transnet-core` binary loads server and provider TOML relative to the crate manifest, initializes structured tracing, binds the Axum listener, and handles Ctrl-C or Unix termination signals. Manifest-relative paths make startup independent of the shell working directory.

`config/transnet.toml` and `config/transnet_llm.toml` are active. `config/transnet_api.toml` and `config/transnet_db.toml` describe deferred work and are not read. `logging.file` is reserved; current logs go to standard output.

The configuration smoke test parses the checked-in server file. Run `cargo test -p transnet --bin transnet-core`.
