# Errors

- Core returns `Result<T>` from `crates/engine/src/error.rs` — the `FatimaError` alias, never a
  bare `Box<dyn Error>` and never a panic.
- **A failure with a shape gets a variant.** `FatimaError::message("...")` is for the one-off that
  genuinely carries nothing but a sentence. A path, a source error, a provider name, a key — that
  is a variant with fields, so the caller can match on it and the message stays in one place.
- Propagate with `?`. Don't catch a failure only to re-raise it as free text; that discards the
  `source` and the type in one line.
- **`miette` stops at the CLI boundary.** `crates/cli/src/commands/` renders a `FatimaError` as a
  diagnostic for a person. Nothing in `crates/engine` may depend on miette — the WASM build has no
  terminal to render into.
- Across the WASM boundary an error is an `ApiResponse::error` payload, serialized. **Never panic
  across it**: a panic in WASM aborts the JS caller with no recoverable message.
- The JS package decodes that envelope **once**, in a single module. Everything above it sees a
  typed error, not a wire shape.
