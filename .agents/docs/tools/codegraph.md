# CodeGraph

The repo is indexed by CodeGraph into `.codegraph/`, and the MCP server keeps that index in sync
as files change. Rust under `crates/` and TypeScript under `packages/` and `scripts/` are both
indexed.

- Reach for `codegraph_explore` before grep or reading files on any structural question: how X
  works, how X reaches Y, who calls this, what breaks if this changes. Name a file or symbol to
  get its line-numbered source.
- Run `codegraph_impact` on a symbol before editing it and report the blast radius.
- Subagents and shells get the same answers from `npx -y @colbymchenry/codegraph@<version>
explore "<question>"`. The version is pinned once, in the `codegraph` entry of
  `.agents/.mcp.json`, and `scripts/dev/codegraph.ts` reads it from there. After a bump,
  `npx -y @colbymchenry/codegraph@<version> index` rebuilds the graph.
- `bun install` builds `.codegraph/` through `bun run codegraph` whenever it is missing, so every
  checkout has the graph before any agent starts. It skips CI (`CI` is set) and any directory that
  is not a git checkout.
