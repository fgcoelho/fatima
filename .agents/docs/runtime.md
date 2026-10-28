# Runtime

`fatima-engine` performs no I/O of its own. Everything outside the process — HTTP, running a
command, touching the filesystem — goes through the `FatimaHost` trait in
`crates/engine/src/host.rs`. The CLI supplies a native host behind the `native` feature; `crates/wasm`
supplies one backed by JavaScript callbacks, and `@fatima.dev/js` provides those callbacks.

- **Never reach for `std::fs`, `std::process` or `reqwest` in `crates/engine` outside a
  `#[cfg(feature = "native")]` block.** The WASM target has none of them, and the failure is a
  build break in a crate you weren't editing.
- A new capability that touches the outside world is a method on `FatimaHost`, implemented once per
  host. It is not a direct call smuggled into a provider.
- Providers live in `crates/engine/src/providers/` and are reached through the registry, never
  constructed by name at a call site.
- The vault is encrypted at rest — `crates/engine/src/vault/crypto.rs` owns that, and nothing else
  reads or writes key material. A secret never lands in a log, an error message, or a snapshot
  fixture.
- `crates/cli` is the only crate that may draw a terminal. `ratatui` and `crossterm` stay behind
  `crates/cli/src/vault/` and `crates/cli/src/ui/`.
- `packages/web` is a static Astro build. It ships documentation, so it holds no secrets and calls
  no fatima API at runtime.
