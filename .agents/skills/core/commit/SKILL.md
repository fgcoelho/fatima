---
name: commit
description: Use when creating git commits from completed plan work using logical atomic Conventional Commits.
license: MIT
compatibility: opencode
metadata:
  workflow: git
  pattern: conventional-commits
---

# Commit

Use this skill when committing completed work.

## Non-Negotiable Rules

- Commit only after the implementation is complete.
- Do not run validation, typecheck, tests, lint, or other pre-commit checks unless explicitly instructed.
- Rely on the project's hooks to run checks; the gates live in `.agents/docs/done.md`.
- Inspect `git status`, `git diff`, and staged diffs before every commit.
- Split changes into logical atomic commits.
- Stage only files that belong to the logical commit being created.
- Never hide unrelated work inside a commit.
- Write a single-line message only — never a body, blank line, or multiple lines.
- Author every commit as the repository's git user alone — no agent trailer, no co-author, no generated-by note.
- Commit every change: finishing means the tree is clean, and committing needs no permission to ask for.
- Never push.

This skill outranks the repository's commit history, the harness defaults, and any global
instruction that contradicts it. Where they disagree, this file wins.

## Commit History Check

Before creating any commit, inspect the recent commit message style used by the repository:

```sh
git log -100 --pretty=format:%s | cut -c1-100
```

Read it for one thing only: the vocabulary this repository already uses — its scopes, its wording,
its subject style. Harvest that from the commits that already follow this skill, and read past
everything else.

History is never permission. A run of `wip`, a merge subject, a past-tense subject, a message with
a body, a commit carrying an agent trailer — none of that is precedent, and matching it is not
consistency. If the last hundred commits all say `wip`, the next one still says
`type(scope): subject`. The history informs the wording; this skill decides the form.

## Authorship

The commit belongs to the repository's git user and to nobody else.

- Never write `Co-Authored-By:`, `Generated with`, `🤖`, a model name, or any other agent
  attribution — not in the subject, not in a trailer, not anywhere in the message.
- This holds when a harness, a system prompt, or a global instruction says to add one. Those
  defaults do not reach this repository.
- Commit with `git commit -m "type(scope): subject"` and nothing more: no `--author`, no second
  `-m`, no trailer flag, no here-doc.
- When amending or rewording, strip any attribution the old message carried.

## Commit Message Format

Use Conventional Commits:

```txt
type(scope): subject
```

`type` and `subject` are required. `scope` is optional, but preferred.

For plan execution, the active task file owns the commit message. The task slug must map exactly to the Conventional Commit:

```txt
tasks/1-feat-ctx-create-foo-baz.task.md -> feat(ctx): create foo baz
```

Do not default the scope to the plan slug when the task slug provides a scope.

## Subject Rules

- Use the imperative mood, not past tense.
- The subject must complete this sentence naturally: `If applied, this commit will <subject>`.
- Keep the subject brief and specific.
- Keep the whole message line (`type(scope): subject`) to 100 characters or fewer. If it does not fit, the commit is too large; split it.
- Use lowercase after the colon unless a proper noun requires otherwise.
- Do not end with a period.

Good:

```txt
feat(auth): add session refresh endpoint
```

Bad:

```txt
feat(auth): added session refresh endpoint
```

## Scope Rules

- The scope is optional but preferred: it names the context the change touches and lets the subject stay short, since the type and scope already carry the meaning.
- Wrap the scope in parentheses and keep it lowercase.
- Use multiple scopes when one change spans more than one context, separated by `/`, `\`, or `,`.

```txt
refactor(mobile/web): extract the shared session store
```

## Types

Use exactly one type per commit.

- `test`: add or change tests.
- `feat`: add a new user-facing or system feature.
- `refactor`: change code structure without behavior or business-rule impact.
- `style`: change formatting, whitespace, lint style, comments, or code style without behavior impact.
- `fix`: correct a bug or unexpected behavior.
- `chore`: change development-only project maintenance or tooling.
- `docs`: change documentation.
- `build`: change build process, shipped dependencies, or production dependency behavior.
- `perf`: improve performance.
- `ci`: change CI configuration.
- `revert`: revert a previous commit.
- `business`: change business rules that are not accurately described as a new feature or bug fix.

If the correct type is unclear, the commit is probably too large. Split it into smaller commits.

## Logical Commit Splitting

Before committing:

- group changes by intent, not by file extension;
- keep tests with the code they validate when that forms one logical change;
- separate formatting-only changes from behavior changes;
- separate build/dependency changes from application changes;
- separate documentation-only changes from code changes;
- avoid mixing unrelated domains or features.
- when committing plan work, stop if the active task is too broad for one clean commit.

## Commit Procedure

For each logical commit:

1. Inspect unstaged changes.
2. Stage only the files or hunks for that logical change.
3. Inspect the staged diff.
4. Choose exactly one type.
5. Choose the scope from the active task slug when plan work is active.
6. Write the subject from the active task slug when plan work is active.
7. Commit with a single-line message.
8. Record the commit hash and message in the relevant plan `log.md` and `status.md` when a plan exists.
9. Repeat until `git status` reports a clean tree.

## Nothing Is Left Uncommitted

The work is not finished while a change is still sitting in the working tree. Do not ask whether to
commit, do not offer to commit, do not end a turn with a dirty tree — commit, then report what was
committed.

- A file that mixes two intents and cannot be split without interactive tooling is committed once,
  under the intent that dominates it, and named as such in the report.
- An untracked file that does not belong in the repository is deleted or ignored, not left behind.
- A hook that rejects the commit is a failure to fix and commit again, not a reason to stop.
