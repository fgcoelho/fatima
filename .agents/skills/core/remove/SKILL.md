---
name: remove
description: How to delete something completely, so that nothing left in the project points at it or remembers it. Read this BEFORE deleting, not after. Use whenever anything is being removed, replaced, renamed away or migrated off of — code, a file, a directory, a function, a route, a table, a dependency, a config key, an env var, a feature flag, a doc section — and whenever you are tempted to leave behind a re-export, a shim, a deprecation note, a compatibility flag, or a test asserting the old thing is gone. If your summary of a change would contain the words "removed", "deprecated", "replaced" or "renamed", consult this skill first.
---

# Remove

The project records what the code **is**. What it _was_ lives in version control, which is better
at it. Every line that describes a removal is a line the next reader must read, hold, and
discover is irrelevant.

## Work outward from the definition

Start at the thing itself, then follow every edge until the graph stops. If deleting it leaves
something else unused — a helper only it called, a dependency only it needed, a config key only
it read — that is part of the same removal, not a follow-up.

## A removal is finished when nothing points at it

In the same change: the code, its tests, its exports, its imports, its registry or manifest
entries, its translation keys, its config and environment entries, its dependency declaration,
its build and CI references, and every doc line naming it. A removal that leaves any of those
behind is not a removal — it is a rename to invisible, and the leftovers outlive everyone who
knew why they were there.

Prove it rather than assume it:

1. Grep the retired name across the whole project, including where a plain identifier search
   misses: string-keyed registries, route tables, translation catalogs, generated manifests,
   lockfiles, fixtures, seeds, CI config, lint config, docs.
2. Grep the variants — kebab, snake, camel, plural, abbreviated, and the human-readable label.
3. Run the project's typecheck or build, its tests, and whatever it has for detecting unused
   code. Dead-code detection is what finds the export that now points at nothing.

## Never record that it is gone

Nothing in the project is a channel back to a reader about a change. No comment saying what used
to be here, no `// removed in favour of X`, no doc sentence in the past tense, no TODO to finish
the cleanup later. Say it in your reply and in the commit message; the diff carries the rest. A
changelog entry only if the project keeps one for people outside the repository.

The same goes for a renamed thing: the file is named what it is named. It is not "formerly `x`".

## Never assert the absence of what you removed

A test named _"the retired layer names are gone"_ is a memorial, not a rule. It fails only if
someone re-creates the exact thing under the exact old name — which nobody who never saw it
would do. What it reliably does instead is teach every future reader a vocabulary this codebase
does not use.

The distinction is what the check constrains:

- A **rule** constrains what may be written next. _A file is named for what it exports_ fails on
  a `helpers.ts` created tomorrow by someone who never saw the old one.
- A **memorial** names one specific thing that was deleted. It cannot catch anything a rule
  doesn't already catch.

A memorial is usually redundant on top of that: if the structure is already validated somewhere,
a second test asserting the deleted thing is absent adds no coverage and one more retired word.
**Delete these on sight** — finding one is reason enough.

## Delete, don't deprecate

No compatibility shim, no deprecation window, no re-export left behind so old imports keep
resolving, no flag that keeps the old path alive. Move the thing and update its callers in the
same change. If you can edit every caller, you do not have a compatibility problem.

The one exception is an interface with consumers you cannot edit. There, removal is a versioned,
announced event with a stated end date — still a removal, just scheduled. It is never a shim
added on a hunch that someone might be depending on this.

Anything not yet released has no consumers. Migrations, fixtures and seeds for unreleased schema
are edited in place, not layered.
