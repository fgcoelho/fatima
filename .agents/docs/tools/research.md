# Research

Three MCP servers, each for a different kind of lookup. Reach for the fitting one before a general
web search: it is narrower, faster, and less often wrong.

## context7: library and framework docs

For the current behavior of a library, framework, SDK, or API: how a function signature changed,
what a config option does, what a new major version broke.

1. `resolve-library-id` with the package name to get its context7 id. Skip this when the id is
   already known from a prior call in the same task.
2. `query-docs` with that id and the specific question: not the package name alone. A narrow
   question returns a narrow, usable answer; a bare package name returns everything and nothing.

Prefer this over training knowledge for anything version-sensitive: a library's behavior a few
months old is exactly the kind of thing that has since changed.

## grepapp: how real code actually uses something

For how real-world projects call an API, structure a pattern, or work around a library's rough
edge: across public GitHub, full-text.

- `searchGitHub` with the exact symbol, call shape, or error string, not a description of intent.
  `useSyncExternalStore(subscribe` finds more than "how to use external store hook".
- Use it to settle a disagreement between what the docs say and what shipped code actually does,
  or to find the idiom for a library with thin documentation.

## exa: everything else

For open-web research: current events, comparisons between tools, a spec not covered by the other
two, or a question that isn't about one library's API surface.

- `web_search_exa` for a normal query.
- `web_fetch_exa` when a specific URL is already known and needs reading in full.
- `agent_run` for a multi-step research task that needs its own iteration, not a single search.

## Reporting back

State which source answered the question. A claim sourced from context7 or grepapp is a fact about
current code; a claim sourced from exa is a fact about the open web as of the search, which can be
wrong or stale: say so when it matters.
