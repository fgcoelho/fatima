# Code

How code is written here, in Rust and in TypeScript alike.

## Tests before code

**Never create or modify production code before writing the corresponding test.**

The test is written first and must fail for the expected reason before the production
implementation is changed. A test written after the code is a test that was shaped by it.

Crate behaviour is covered from `crates/engine/tests/`, against the public API rather than a private
function reached through `pub(crate)`. A unit test lives beside its module in a `#[cfg(test)]`
block only when the thing under test genuinely has no reachable public surface.

## No comments

**Do not write comments in source code. No exceptions.**

The code must explain itself through clear and precise names, explicit types, small focused
functions, simple control flow, well-defined abstractions, and focused tests.

When something appears to require a comment, improve the implementation instead: rename it,
extract it, simplify it, improve its types, redesign it.

Do not add a comment to explain a workaround, an external constraint, a formula, an edge case, a
business rule, an implementation detail, non-obvious behavior, or a change you made. Express the
reasoning through code, tests, types, abstractions, or external documentation — reasoning that
outlives the code goes in `.agents/docs/specs/YYYY-MM-DD-<feature>/spec.md`.

**Never narrate the code or your edits, and never address the reader from inside a file.** Code,
tests, docs and copy are the product, not a channel back to me: no "as requested", no note
explaining what you changed or why, no apology, no TODO written at a person. Say it in the reply —
the diff already carries the rest.

## Think before coding

**Do not assume. Do not hide uncertainty. Surface trade-offs.** Before implementing anything:

- State your assumptions explicitly.
- When something is uncertain, ask.
- When multiple interpretations are possible, present them instead of silently choosing one.
- When a simpler solution exists, point it out.
- Push back when the requested approach introduces unnecessary complexity or risk.
- When something is unclear, stop and explain exactly what is unclear before proceeding.

## Simplicity first

**Write the minimum amount of code required to solve the problem. Nothing speculative.**

- Do not implement features that were not requested.
- Do not create abstractions for code that is used only once.
- Do not add flexibility or configurability that was not requested.
- Do not add error handling for scenarios that cannot occur.
- Do not design for hypothetical future requirements.
- If an implementation takes 200 lines but could reasonably take 50, rewrite it.

Before finishing, ask yourself:

> Would a senior engineer consider this implementation overcomplicated?

If the answer is yes, simplify it.

## Surgical changes

**Change only what is necessary. Clean up only what your changes affect.**

When modifying existing code:

- Do not improve unrelated code, comments, naming, formatting, or structure.
- Do not refactor code that is unrelated to the requested change.
- Follow the existing code style, even if you would normally choose a different approach.
- When you notice unrelated dead code, mention it instead of removing it.

When your changes make existing code unnecessary:

- Remove imports, variables, functions, files, and dependencies made unused specifically by your
  changes.
- Do not remove pre-existing dead code unless explicitly requested.

Every changed line must be directly traceable to the user's request.

## Compatibility lives at the edges, nowhere else

Fatima publishes four surfaces that other people depend on: the `fatima` CLI and its flags,
`fatima-engine` and `fatima-cli` on crates.io, `@fatima.dev/js` on npm, and the on-disk formats —
`fatima.json`, the generated SDKs, and the vault the CLI reads. Those are contracts. A vault
written by an older release must still open, and a change to `static/schema.json` is a change to
everyone's editor completions.

**Everything behind those four surfaces has no compatibility obligation at all.** Internal module
layout, private types, helper signatures, the shape of a struct nobody outside the crate names —
change them freely and update the callers in the same commit.

- Do not keep a deprecated internal function, field, or `pub(crate)` export for compatibility. Delete it.
- Do not write a shim, a deprecation path, or a fallback for an older internal shape.
- Choose the shape you would choose if the code had always looked that way.

When a change does reach one of the four surfaces, that is a decision, not an implementation
detail: it belongs in a `spec.md` before it belongs in a diff, and a format that gains a field
carries the old files forward rather than rejecting them.

## Types, errors and names

Rust:

- **No `unwrap()`, no `expect()`, no `panic!` in library or command code.** Return `Result` and let
  the boundary render it. A test may `unwrap()` freely; nothing that runs for a user may.
- Errors are `FatimaError` variants from `crates/engine/src/error.rs`, not stringly-typed messages —
  see `.agents/docs/errors.md`.
- **No `unsafe`.** If you believe you need it, stop and explain why before writing it.
- Prefer borrowing to cloning. Reach for `clone()` when the alternative is a lifetime that
  complicates every caller, not as the first move out of a borrow-checker error.
- `impl Trait` and generics over `Box<dyn Trait>` unless the type genuinely has to be erased.
- A module is named for what it holds. No `utils.rs`, no `helpers.rs`, no `common.rs`.

TypeScript:

- **No `as` casts, no `any`, no `!`.** `as const` and `satisfies` are fine. Fix types at the
  source: narrow with guards, refine control flow, or validate at the boundary.
- If the compiler complains, listen — don't silence it.
- **No default exports**, except where the framework demands them (config files, Astro pages).
- The WASM bindings in `packages/js/src/wasm/` are generated. Never hand-edit them.

Both:

- Match the surrounding code's style, naming and idiom.
- Independent async work runs concurrently — `Promise.all` in TypeScript, `futures::join!` or
  `try_join_all` in Rust. Never await in sequence when the iterations don't depend on each other.
