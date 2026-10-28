---
name: layout
description: Where a file lands, what it may be named, what it may export and what it may import — all declared in a layout.json at the project root. Use before creating or moving any file, when choosing a directory or a filename shape, when adding an import that crosses a directory, when adding a directory or a file kind, and whenever a lint rule or a test that reads layout.json fails. Read layout.json rather than inferring the shape from neighbouring files.
---

# Layout

A project with a `layout.json` at its root has written down where its code lives. It is the
single statement of that: nothing is generated from it and no document restates it, a lint rule
reads it, a test walks the filesystem against it, and every entry carries a `description`
explaining itself. **Read it. Do not infer the shape from neighbouring files** — a neighbour can
be wrong in a way the rule has not caught yet.

The format knows nothing about any particular project. It declares a tree of directories, a
catalog of filename shapes, and the edges between them; the same schema describes a monorepo, an
`src/` layout or a flat one. Its `$schema` points at a generated JSON Schema — regenerate that
with whatever script the project names, and never hand-edit it.

## Looking a file up

`directories` is a tree, keyed by directory name, and every node has the same shape: a
`description`, what it holds (`kinds`), what it contains (`directories`), and what it may import
(`reach`, `types`).

1. **Walk the path** — segment by segment, from the top-level `directories` map down. The first
   node whose children do not name the next segment is where the walk stops. A key may be a
   pattern: a name is read before the patterns, and the patterns in the order they are written,
   so a `*` beside `discord` takes every directory that is not `discord`.
2. **A node may name a `slot`** instead of declaring its own shape. Slots are the top-level
   catalog of the specific shapes this repository repeats: every slice under a root names the
   same one, so the slice is described once and each directory only says which shape it has.
   Anything the node writes itself replaces what the slot supplies. A directory that only needs a
   flag set writes the flag rather than naming a slot for it.
3. **Naming a slot makes the directory one instance of that shape.** An address is then the root,
   the instance, and the path inside it: `server/billing/queries/` is addressed `server/queries`,
   and so is `server/discord/voice/queries/`, because whatever names the run of directories in
   between is a convenience. That is what lets one slot describe every slice, and it is the
   boundary `confined` means.
4. **Read `kinds`** — the node's `kinds` name entries in the top-level `kinds` catalog. Each says
   what filenames it matches (`holds`, a list of globs), what the file may `export`, and what its
   source must or must not contain.

A directory that names no `kinds`, and contains nothing that does, is **opaque**: it declares reach
but not placement, so any directory beneath it, any filename, and any import that stays inside it
are its own business. Opacity is read off the declaration; there is no field for it. `universal` names directories
that hold the same kinds wherever they appear: a `components/` directory holds components and a
`hooks/` directory holds hooks, at any depth, in any tree.

`reference` on a node points at a document explaining its reasoning, inherited by everything
beneath it. Where a node sets none, its `description` is the reasoning.

## What a file may export

A kind's `exports` is the file's **whole** export surface, not a minimum. Every export must match
an entry, so a form the list leaves out is a fault — `export default` and re-exports included.

An entry is a declaration form (`class`, `const`, `default`, `enum`, `function`, `interface`,
`re-export`, `type`) with an optional count: none means exactly one, `?` zero or one, `*` any
number, `+` one or more. So `["const+", "type*"]` reads _one or more consts, any number of types,
and nothing else_ — no class, no function, no default export.

A kind with no `exports` may export anything. `eponymous` is the separate name check: the file gives
its name to what it exports, so every exported function name must contain the filename's stem.

Everywhere else, the **namespace rule** applies: two or more loose exported functions in a `.ts`
file must travel as one object named for the module.

## Reach

Reach is **positive and complete**. A directory lists what it may import; anything unlisted is
refused. There is no denylist and no exemption list.

- `reach` — what it may import as a value.
- `types` — what it may additionally `import type`.

Both are **inherited downward**: a file may reach what any directory above it reaches, which is
why an outer `reach` stays short.

A target is a **directory address** — the root, the instance the file is in, then its path inside
that instance, so a file in `server/billing/queries/` is addressed `server/queries`, and so is one
in `server/discord/voice/queries/`. A bare address matches either relative
to a directory the two share, or absolutely from the top; `./name` insists on the relative
reading and `/name` on the absolute, which is how a unit's own `./scripts` stays distinct from a
top-level `/scripts`. A target may also be written as a tsconfig path alias — `@database/drizzle`
— which resolves to the directory that alias points at and matches absolutely, so the layout names
a root the same way an import does. A scoped package is told apart from an alias by
`tsconfig.json`: `@better-auth/stripe` resolves through no `paths` entry, so it stays a package.
Naming a directory admits everything beneath it, so `"models"` admits
`models/entities` while `"models/enums"` admits only that one. `"*"` admits everything, which is
what a directory that reaches the whole project says rather than listing every entry.

How an import resolves:

1. The specifier becomes a project path, through the tsconfig aliases, or a package name.
2. A package is free unless some `reach` in the file names it. Naming a package restricts it
   everywhere: from that point on, only a directory whose `reach` names it may import it. There
   is no separate list, because naming it is the declaration.
3. A path resolves to a node, and that node's address is matched against the accumulated `reach`
   — or, for `import type`, `reach` plus `types`.
4. A directory always reaches itself and the ones containing it, and an opaque directory's internal
   imports are its own business.
5. A node with `"confined": true` is additionally reachable only from inside the same unit it sits
   in. Reach already decides _which_ directories may name it; this adds that they must belong to
   the same unit. It is waived when the file has no `counterpart` beside it — which is how a file
   of a paired kind that pairs with nothing stays reachable without an exemption list — and it is
   silent toward an importer that is inside no unit of its own.

## Changing it

Adding a directory, a filename shape or an import edge means editing `layout.json` — that is the point,
and the `description` you write there is the documentation. Widening a `reach` to make a lint
failure go away is a decision about the architecture, not a formality: prefer moving the file.

A layout lint message names what refused the file. `layout.json` is where the rule and the reason
for it both live — read the `description` on the node or the kind it names.
