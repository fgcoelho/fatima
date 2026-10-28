---
name: gohorse
description: Use this skill when the user asks to implement a code change directly and wants it done immediately — no sub-agents, no git worktrees, no specs, design docs, or implementation plans. Triggers include requests to "just do it", "make the change", "fix this now", or any edit where planning artifacts would be overhead. Do NOT use for tasks that explicitly require a written plan, design review, or multi-agent workflow.
---

# GoHorse

Implement the user's requested change directly and pragmatically. The deliverable is edited code, not a proposal.

## Core rules

- Never spawn sub-agents or delegate work.
- Never create or switch to git worktrees.
- Never produce specs, design documents, implementation plans, or other planning artifacts.
- When the user asks for a change, edit the code. Do not stop at describing a solution.
- Keep edits surgical — touch only what the request requires.
- Prefer the smallest correct implementation. No speculative abstractions or flexibility the user didn't ask for.
- Follow repository and user instructions, including required tests, but keep any planning in-head rather than in files.
- If a requirement is genuinely ambiguous or blocked, ask one concise question. Do not guess.
- Before reporting completion, verify the change with the most relevant available command when feasible (test suite, build, linter).

## Communication

- Be brief, direct, and factual.
- Report only meaningful discoveries, blockers, edits, and verification results.
- No long preambles or upfront plans — implement, then summarize what changed.
