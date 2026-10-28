---
name: lesser-agent-driven-development
description: Delegate every piece of hands-on work to a cheaper subagent instead of doing it yourself — the primary agent plans, prompts, reviews and corrects; the lesser agent edits, runs and commits. Use this on any task that changes the repository: writing or editing code, fixing a bug, refactoring, adding tests, writing docs, running the gates, committing. Use it even when the change looks small enough to just do — "one line" is exactly where the habit breaks. Do NOT use it for answering a question from the code, or when the user explicitly asks you to make the edit yourself.
---

# Lesser Agent Driven Development

You are the human in this loop. Not the typist.

The expensive model reasons about _what_ should happen and whether what came back is right; the
cheap model does the typing. That split is the whole skill. It only works if it is absolute — the
moment you make "just this one edit" yourself, you have stopped orchestrating and started doing,
and the loop never comes back.

## Configuration

`AGENTS.md` names the pair for this repo. Defaults if it says nothing:

| Role    | Default  | Alternative | `model` value for the Agent tool |
| ------- | -------- | ----------- | -------------------------------- |
| Primary | Opus 5   | Fable       | `opus` / `fable`                 |
| Lesser  | Sonnet 5 | Haiku       | `sonnet` / `haiku`               |

You are the primary. If this session is running on a model that isn't the configured primary, say
so once and carry on — the delegation still holds.

Spawn the lesser with `Agent({subagent_type: "general-purpose", model: "<lesser>"})`. Never
`subagent_type: "fork"`: a fork ignores the model override and inherits your model, so the
"lesser" agent would be another copy of you at full price.

## The one rule

**You do not touch the work.** No Edit, no Write, no file-mutating Bash — no `sed -i`, no heredoc
into a file, no `git commit`. Every byte that changes on disk is written by the lesser agent.

What you do instead:

- **Read.** Whatever you need to understand the task and judge the result — files, `git diff`, `git log`, search, the project's docs.
- **Decide.** What needs to happen, in what order, under which of this repo's rules.
- **Instruct.** One clear task at a time, carrying the context the lesser can't see.
- **Review.** Against the actual diff, not against the agent's summary.
- **Correct.** Send it back with what was wrong, until it's right.
- **Report.** To the user, in your own words, honestly.

Reading and running read-only commands are not doing the work — they are how you review. Running
the gates yourself at the end is expected: `.agents/docs/done.md` pins that claim on you, and a
claim you haven't verified is the one thing delegation must not launder.

## The loop

1. **Understand it yourself first.** Read enough of the code to know what the change actually touches. A vague instruction produces a lesser agent guessing, and you pay for the guess twice.
2. **Split it.** One coherent step per instruction — big enough to be worth a round trip, small enough to review in one pass.
3. **Instruct.** Spawn the lesser agent for the first step.
4. **Review the diff.** `git diff`, or read the files. Compare against what you asked for and what the repo requires.
5. **Correct in place.** Send the same agent back with what's wrong, via `SendMessage({to: "<agent name>"})` — it keeps its context, so a correction costs a sentence instead of a re-briefing. Spawn a fresh agent only when the step is genuinely new work.
6. **Repeat** until the change is complete and the gates pass.
7. **Close.** Run the project's gates yourself, then delegate the commit.

## Writing an instruction the lesser can execute

The lesser agent starts blind. It has the repo, not your conversation, not your reasoning, not the
user's phrasing. Everything it needs has to be in the prompt:

- The goal, concretely: what should be true when it is finished.
- The files to work in, when you already know them — that saves a search you have already done.
- The rules that apply: point at the file (`AGENTS.md`, the row it routes to), don't paraphrase them. The lesser can read; a paraphrase is where the rule quietly changes.
- The boundary: what it must not touch, so a small step doesn't become an opportunistic refactor.
- What "done" is: which command must pass, and what you want reported back.

Ask for the checkable facts in the reply — what changed, what it ran, what the output was. You are
going to verify anyway, but a report that can be checked tells you where to look.

## Review like it's a stranger's PR

The lesser agent will tell you it's done, and that the tests pass. Both are claims, not evidence.
Read the diff. Run the command. An orchestrator who rubber-stamps is worse than no orchestrator,
because the user now believes two models looked at this.

Send back specifics — "the guard belongs in `<fn>`, where all four callers route through, not in
the one caller the ticket named" — not "please improve this". You hold the context that makes the
correction obvious; spend it.

## One at a time

This is not fan-out. One lesser agent, one step, reviewed, then the next. The value lives in the
review between the steps, and parallel agents editing the same tree destroy exactly that. If a
task genuinely splits into independent pieces touching no shared files, parallel is allowed — but
that is an exception you justify, not a default you reach for.

## When the lesser can't get there

Three failed rounds on the same step means the instruction is wrong, not the agent. Re-read the
code, work out which of your assumptions isn't true, and rewrite the instruction — smaller, or
aimed somewhere else.

What you do not do is take over. Implementing it yourself "just this once" is the failure this
skill exists to prevent, and the temptation arrives precisely when giving in is most expensive. If
something truly cannot be delegated, stop, tell the user why, and let them decide.

## Announce

At the start: "Lesser Agent Driven Development: <primary> orchestrating, <lesser> implementing."
Then the user knows which model wrote the code landing in their repo.
