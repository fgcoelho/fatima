---
status: accepted
date: 2026-08-27
---

# Record decisions forced on us from outside as MADR in `.agents/docs/adr/`

## Context and Problem Statement

A decision about work we designed lives in that work's `spec.md`, so the decision and its rationale
cannot drift apart. The exception is a choice forced on us from outside - by a runtime or a
dependency - that no spec of ours produced: it has no feature to belong to, and lives in a file of
its own.

There are no such files here yet. Without a template the first one is written from scratch, the
second is written from the first, and by the fourth there are four shapes: one opening with the
constraint, another with the solution; one listing what it rejected in a section of its own, another
dissolving the rejection into the prose; none saying what state the decision is in, or when it was
last touched. Reading two in a row then costs relearning the order of the parts. So the question is
what shape the first one takes, answered before there is a first one to argue with.

## Decision Drivers

- The options considered have to be visible as a list, not buried in prose: the reason the chosen
  one won is made of them.
- The state of a decision - proposed, accepted, superseded - has to be readable without `git log`.
- The template must not cost a new tool: these are Markdown files in a repository, read by people
  and by agents, and they stay that way.
- A published, recognizable order beats an order of our own, because whoever arrives already knows
  it.

## Considered Options

- MADR 4.0.0, full template
- MADR 4.0.0, minimal template
- Nygard - title, status, context, decision, consequences
- No template, each file written its own way

## Decision Outcome

Chosen option: "MADR 4.0.0, full template", because it is the only template published at
[adr.github.io](https://adr.github.io/adr-templates/) that gives the considered options - and the
pros and cons of each - sections of their own, and because it is plain Markdown with no tool to
install.

In practice:

- the files live in `.agents/docs/adr/`, named `NNNN-title-with-dashes.md`, numbered in the order the
  decisions were taken;
- `.agents/docs/templates/adr.md` is the template to copy;
- the optional `decision-makers`, `consulted` and `informed` keys are not used here: there is one
  team, and `git log` says who wrote it.

### Consequences

- Good, because the rejected alternative gets a fixed place: whoever reopens the choice finds what
  was already measured instead of measuring it again.
- Good, because `status` and `date` say, at the top, whether the record still holds.
- Good, because a superseded ADR is neither deleted nor edited into something else: it gets
  `superseded by`, and its number keeps resolving the links that point at it.
- Bad, because the full template has nine sections, and a small decision will carry empty ones or
  have them removed by hand - MADR allows removing the optional ones, which is what we do.
- Neutral, because the numbering means looking at the directory before creating a file, so as not
  to reuse a number.

### Confirmation

Code review against `.agents/docs/templates/adr.md`, and the rule written in `.agents/docs/planning.md`.
There is no test: an ADR is prose, and no assertion tells good prose from bad.

## Pros and Cons of the Options

### MADR 4.0.0, full template

Front matter with `status` and `date`, then _Context and Problem Statement_, _Decision Drivers_,
_Considered Options_, _Decision Outcome_ with _Consequences_ and _Confirmation_, _Pros and Cons of
the Options_, and _More Information_.

- Good, because the considered options and the pros and cons of each have sections of their own.
- Good, because `## Confirmation` asks how compliance with the decision is verified, and in this
  repository the answer usually exists: a test already holds the choice in place.
- Good, because it is the template of the organization that publishes adr.github.io, maintained and
  versioned.
- Bad, because it is the longest of the three - for a one-line decision the template outweighs it.

### MADR 4.0.0, minimal template

Only _Context and Problem Statement_, _Considered Options_ and _Decision Outcome_.

- Good, because it fits on one page.
- Bad, because it has nowhere to put the measurement, and a decision forced from outside is made of
  measurements.
- Bad, because it loses _Confirmation_, which is the link between the record and the test that
  holds it.

### Nygard

- Good, because it is the original format and the most widespread.
- Good, because it has an explicit `## Status`.
- Bad, because it has no options section: the rejected alternative goes back to being a paragraph
  inside the context, which is the problem this decision exists to solve.

### No template

- Good, because it costs nothing today.
- Bad, because the cost is not today's: it belongs to whoever reads the fourth record and still has
  to work out where each one put what it rejected.
- Bad, because nothing forces the state of the decision to be recorded, and a document describing a
  choice already reverted looks like one describing the choice in force.

## More Information

- [adr.github.io](https://adr.github.io/) - the organization, the templates and the practices.
- [MADR](https://adr.github.io/madr/) - the template adopted here, at 4.0.0.
- `.agents/docs/planning.md` - where a decision lives, and why most of them are not ADRs.
