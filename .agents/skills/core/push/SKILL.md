---
name: push
description: How to push `development` safely, survey the tree and remote drift, run the done.md gates, push, and report what went out. Use when asked to push, ship it, or sync the branch to origin. Never pushes `fatima` and never pushes a tag unless the user explicitly asks.
---

# Push

Work happens on `development`. The default branch is `fatima`. A push triggers no CI: releases
go out only when a `v*` tag is pushed, which runs `.github/workflows/release.yml`. So a branch
push is cheap to undo, and a tag push is not.

This skill covers the push itself. Commit mechanics (message format, splitting, authorship)
belong to `commit/SKILL.md`; finish committing there first. What the gates actually run lives in
`.agents/docs/commands.md` and `.agents/docs/done.md`; this skill only says when to run them.

## Procedure

1. Survey before pushing: `git status --short`, `git log --oneline origin/development..development`,
   `cat /proc/loadavg`, and `git fetch origin` plus `git log --oneline development..origin/development`
   to catch remote drift. If the remote has commits you lack, stop and tell the user; never
   force-push. Never push a dirty tree without deciding what the untracked files are.

2. Check the tip commit is fit to push. A tip commit whose subject is `wip`, or one made seconds
   ago in a file the user is editing, is a stop and ask, not a push. Never reword someone else's
   commit on your own initiative.

3. Coordinate with any peer session. `ListAgents` shows other sessions on this checkout. If one
   is active, ask before pushing and tell it when you are done, since the git index is shared
   between sessions on the same checkout.

4. Run the gates from `.agents/docs/done.md` for what the unpushed commits touched:
   - Rust: `bun run check:rust`, `bun run test:rust`, `cargo clippy --workspace --all-targets -- -D warnings`.
   - TypeScript or Astro: `bun run checktypes` and `bun run lint`.

   A gate whose binary is missing is skipped, not passed: say so in the report.

5. Push: `git push origin development`. Never use `--no-verify` to make a failure go away. It is
   justified only when the failure is provably external, and the reason goes in the report.

6. Never push `fatima`, and never push a tag (`git push origin v*`, `git push --tags`) unless the
   user explicitly asks. A tag starts a release that publishes to npm and crates.io.

7. Report: the branch and the commit range pushed (`git log --oneline <old>..<new>`), what shipped
   in plain words, which gates ran and passed, and any repair made along the way.

## Failure playbook

- Lint fails on formatting. Fix it with `bun run format`, then a `style` commit.
- A Rust gate fails. Fix the cause; do not add an `allow`. See `.agents/docs/done.md`.
- The push is rejected as non-fast-forward. Fetch, look at what arrived, and ask the user whether
  to rebase or merge. Do not force.
