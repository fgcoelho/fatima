# AGENTS.md

Fatima is safe secrets and environment management, and it is growing a hosted SaaS beside the CLI.
A Cargo workspace in `crates/` holds it: the `fatima` CLI with its terminal vault UI, the
`fatima-engine` library every surface is built on, a `fatima-wasm` build, and the `task` crate that
drives releases. `packages/js` wraps the WASM build as `@fatima.dev/js`, and `packages/web` is the
Astro documentation site. bun, turbo, and Oxc for lint and format.

## Agent

How to behave, on every task, before anything in `## Routing`.

- Invoke ponytail on any code, library or design choice. Use codegraph before grep or Read to
  explore, and before editing a symbol. Research with context7 (library docs), grep.app (real
  usage) and exa (open web) before guessing.
- Long-term recall and capture go through ai-memory. The harness's own `memory/` directory is the
  user's explicit notes: write to it only when the user asks to record something there, and never
  to save session learnings.
- Act on what is clear. Ask only when blocked on a decision that is the user's to make, and give a
  recommendation with the question, not a survey of options.
- Work on `development`, straight on it: no feature branches, no worktrees. Commit by path, since
  another session may share the tree, and read `git diff --cached` before every commit.
- Push only when the user says "push", and never push `fatima` (the default branch) or a tag.
- Go wide: independent work goes out as parallel subagents in one message.
- Report faithfully. A failing check is reported with its output, a skipped step is said to be
  skipped, and nothing is called done until it clears `.agents/docs/done.md`.

## Subagents

Every task runs under Subagent Driven Development: the primary agent plans, dispatches, reviews and
runs the gates, and implementer subagents write the code. Read
`.agents/skills/superpowers/subagent-driven-development/SKILL.md` before changing anything.
Independent tasks go out in parallel waves; implementers never run the typecheck, lint, tests or
build, and the primary runs them once per wave, one at a time.

| Role        | Agent    |
| ----------- | -------- |
| Primary     | Opus 5   |
| Implementer | Sonnet 5 |

Every implementer brief names the rows of `## Routing` that apply to its task and tells it to
invoke ponytail per `## Agent`, because a subagent reads only what its brief points at.
`.agents/skills/core/lesser-agent-driven-development/SKILL.md` stays available when the user asks
for it.

## Routing

This file routes. Every rule lives in exactly one place, and that place is never here, so read the
row before you do the thing rather than after a rule fires.

| Doing                                                       | Read                                                                 |
| ----------------------------------------------------------- | -------------------------------------------------------------------- |
| Writing code                                                | `.agents/docs/code.md`                                               |
| Raising an error                                            | `.agents/docs/errors.md`                                             |
| Understanding runtime                                       | `.agents/docs/runtime.md`                                            |
| Deleting, replacing, renaming away                          | `.agents/skills/core/remove/SKILL.md`                                |
| Running anything                                            | `.agents/docs/commands.md`                                           |
| Using long-term memory                                      | `.agents/docs/tools/ai-memory.md`                                    |
| Exploring code structure, impact                            | `.agents/docs/tools/codegraph.md`                                    |
| Researching a library, API, pattern                         | `.agents/docs/tools/research.md`                                     |
| Using a service (database, billing, email, hosting, search) | `.agents/docs/tools/services.md`                                     |
| Fixing a bug                                                | `.agents/skills/superpowers/systematic-debugging/SKILL.md`           |
| Exploring what to build                                     | `.agents/skills/superpowers/brainstorming/SKILL.md`                  |
| Verifying before completion                                 | `.agents/skills/superpowers/verification-before-completion/SKILL.md` |
| Planning a feature                                          | `.agents/docs/planning.md`                                           |
| Running a security review (pentest)                         | `.agents/skills/core/pentest/SKILL.md`                               |
| Writing copy                                                | `.agents/skills/marketing/copywriting/SKILL.md`                      |
| Writing docs, or any prose                                  | `.agents/skills/core/writing/SKILL.md`                               |
| Writing an ADR                                              | `.agents/docs/templates/adr.md`                                      |
| Committing                                                  | `.agents/skills/core/commit/SKILL.md`                                |
| Pushing                                                     | `.agents/skills/core/push/SKILL.md`                                  |

Every path above exists. If one of them is dead, fix this table in the commit that discovers it,
because a pointer to a file that is not there costs more than no pointer at all.
