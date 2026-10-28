---
name: yolopowers
description: Run the whole superpowers chain end to end on full auto — spec, plan, TDD implementation, code review, ship — with every human gate ripped out and every question ruled on instead of asked. Use when the user says "yolo", "full auto", "no questions", "take it from spec to shipped", or hands over a feature and wants it built without check-ins. Do NOT use when the user wants to be consulted on approach, or for a one-file mechanical edit (that is gohorse).
argument-hint: <what to build>
---

# Yolopowers

Superpowers with the gates ripped out. Spec, plan, TDD, review, ship — one
invocation, zero check-ins.

**Announce at start:** "Yolopowers on <feature>. Spec to shipped, no check-ins.
Rulings at the end."

## Override

Superpowers stops to ask. You don't. Wherever one of its skills says wait for
your human partner, skip the wait and take the answer here:

| Skill                            | Where it stops                                                                     | You                                                              |
| -------------------------------- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `brainstorming`                  | path classification, per-section approval, the `<HARD-GATE>`, the spec review gate | architectural path, self-review, write, commit, keep moving      |
| `brainstorming`                  | clarifying questions, one at a time                                                | read the code for the answer; rule on what the code can't settle |
| `writing-plans`                  | "Which approach?"                                                                  | subagent-driven-development                                      |
| `using-git-worktrees`            | worktree consent                                                                   | the project's branch policy is the answer — no consent round     |
| `executing-plans`                | "stop and ask" on any blocker                                                      | rule and keep going                                              |
| `subagent-driven-development`    | plan defects at the pre-flight scan                                                | rewrite the plan yourself, log the rewrite                       |
| `receiving-code-review`          | "clarify before implementing"                                                      | the reviewer is a subagent — decide and fix                      |
| `finishing-a-development-branch` | base branch confirmation, the 3-option menu                                        | integrate the way the project already integrates                 |

Below the gates, nothing changes. Test first. Review every task. Verify before
claiming. Yolopowers deletes the waiting, not the work — a run that skips the
failing test or the review loop isn't yolo, it's just broken.

## Rulings, not questions

Every question you swallowed becomes a line:

```
Ruling: <what you decided> — <why> — <what it costs if wrong>
```

Log it the moment you make it, in the same message as the work it unblocks —
the ledger at `.superpowers/sdd/<plan-basename>/progress.md` once
subagent-driven-development creates it, a `## Rulings` heading in the spec
before that. Every one of them reaches the final report. A ruling that dies in
the workspace was a decision made behind the user's back, and that is the one
thing this skill can't afford: the rulings list is what they read to find what
you got wrong, and rulings you got wrong are the expected cost of running this
way.

## Read the project first

You are about to make every call the user would have made. Before the first
stage, read the project's instruction files — `AGENTS.md`, `CLAUDE.md`, and
whatever they point at — plus its command table and its branch history. That
reading is what turns a ruling into a decision the project would have made
anyway. Nothing below hardcodes a command, a path, or a branch name: the
project states those, and where it doesn't, you rule and log it.

## The chain

Four stages, one continuous pass. No progress summaries between them.

**1. Spec** — `superpowers:brainstorming`, written where the project keeps its
specs and committed. The project's convention beats the sub-skill's default
path; if it states none, rule on one and log it. Pick the
approach you would have recommended, ruling-log it and the runner-up. If the
request spans independent subsystems, spec and build the first one only, and
ruling-log what you deferred. Specs state the target: write what the model
shall be, not a diff against what it was.

**2. Plan** — `superpowers:writing-plans`, written beside the spec under the
project's convention, spec path in the header. Two things carry the run:

- **Global Constraints get the project's rules verbatim** — copy into that
  section every rule the instruction files impose on an implementer: the
  testing law and what must fail first, the comment policy, the scope rules,
  the schema and migration policy, the generated files that are never
  hand-edited. An implementer subagent sees its task and that section, nothing
  else. A rule you leave out is a rule it breaks, and paraphrase is how rules
  get left out.
- Every task names the file it creates and the directory that owns it. Where
  the project encodes its layout — architecture tests, lint rules, a
  repository skill — an invented path fails there, long after the plan said it
  was fine.

**3. Build** — `superpowers:subagent-driven-development`, as written. Its task
loop is already continuous and already rules rather than stalls; TDD and code
review both live inside it. Follow the project's branch policy — never invent a
branch or a worktree it doesn't use. Commits follow the `commit` skill. If the
baseline is red before you start, fix it if it's yours, otherwise ruling-log
the failures by name — someone else's broken baseline doesn't get to eat the
run.

**4. Ship** — integrate the way the project integrates: a pull request where
that is the path, a commit to the trunk where it is. Nothing stays
uncommitted and nothing stays unpushed — on a gateless run, work that never
left the machine is work nobody can review.

## What still stops you

Three things: destroying data you can't get back, touching production, and
anything that would leak a secret. Everything else — ambiguity, conflicts,
plan defects, a cap you'd have asked to exceed — you rule on and keep going.

## Project gates

Non-negotiable before any completion claim, in the project's own commands:

- the type checker
- the linter, and the formatter if it can fail
- the tests your change touches, named by path

Bring up whatever the tests need — a database, a container — before the first
stage runs, not at the first red run. Run the whole suite only where the
project asks for it: elsewhere it is a cost, and in a project whose suite owns
a shared database it is a hazard. Where the test tiers read different
environments, know which one your change has reached and which still needs the
schema step. `superpowers:verification-before-completion` holds: no claim
without fresh output in the same message.

The laziest thing that passes the tests is the thing to build, and that applies
inside every stage — a spec proposing an abstraction the code doesn't need is a
spec to cut before you plan it.

## Final report

1. **Shipped** — one line, plus the pushed commit range.
2. **Rulings I made** — every `Ruling:` line, in order, each with its
   cost-if-wrong. Exhaustive.
3. **Parked** — findings adjudicated at the breaker, minors the final review
   let through.
4. **Gates** — typecheck, lint, specs, as run.
5. **Not done** — anything the run left out.

## Common rationalizations

| Excuse                                                     | Reality                                                                                           |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| "This one question is worth asking"                        | It's a ruling. Read the code, decide, log it.                                                     |
| "Yolo means skip the spec and plan"                        | Yolo means no check-ins. The artifacts are what keep a gateless run reviewable.                   |
| "Full auto, so skip the failing test first"                | The iron law isn't a gate the user holds. Not yours to drop.                                      |
| "A branch is safer for a run this big"                     | The project's branch policy already decided. A branch nobody asked for is a branch nobody merges. |
| "I'll run the whole suite to be sure"                      | The project names its gates. Run those, and the specs your change touches.                        |
| "The suite passed at the last task"                        | Fresh output, same message, or no claim.                                                          |
| "I'll summarize progress between stages so they can steer" | They asked for no check-ins. The final report is the steering.                                    |
| "The plan is wrong, I should stop"                         | Rewrite it, log the rewrite, keep going.                                                          |
| "Nobody reads the rulings list"                            | It's the only record of what you decided for them. Skip it and the run is unauditable.            |
