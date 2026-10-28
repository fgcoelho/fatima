# Done means done

Every task ends here, whatever its size: a question answered from the code, a one-line fix, a
whole feature. Nothing below is skipped because a change looked too small to break anything.

- Touched Rust? `bun run check:rust` and `bun run test:rust` both pass, and `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- Touched TypeScript or Astro? `bun run checktypes` and `bun run lint` both pass.
- No exceptions, no "should be fine". Never claim work is done without having run them.
- After any edit, a Stop hook runs the gates for the languages you touched and blocks on a finding. Fix it — don't work around it.
- The hook skips a check whose binary is missing rather than failing. **A skipped check is not a passing check** — if `cargo` is not on your PATH, say so in your report instead of claiming the Rust gates passed.
- Crate versions here may be newer than you know. Check the installed source under `~/.cargo/registry` or `node_modules/<pkg>` before assuming an API — don't write from memory.
- The task is not finished until the change is committed. Follow `commit/SKILL.md` — logical atomic Conventional Commits.

## Generated output is never hand-edited

`packages/js/src/wasm/`, everything under `.release/`, `Cargo.lock`, `bun.lock`, and
`static/schema.json`. Regenerate it instead — `static/schema.json` comes from the config model in
`crates/engine`, so a schema change starts there.

## What the rules are, and what they are not

`.oxlintrc.json` enforces the TypeScript structure and clippy enforces the Rust. A failure is the
rule speaking, not a hurdle to route around.

**No rule carries an exemption list, and no file disables one.** No `#[allow(...)]` on a lint you
find inconvenient, no `oxlint-disable` comment, no new entry in an `ignorePatterns` array to make a
finding go away. An `allow` is acceptable only where the lint is provably wrong about that specific
line, and then it is the narrowest scope that works — never a crate-level `#![allow]`.
