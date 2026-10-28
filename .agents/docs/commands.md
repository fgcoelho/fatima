# Commands

| Command                            | What it does                                                                                  |
| ---------------------------------- | --------------------------------------------------------------------------------------------- |
| `bun run dev`                      | turbo dev across the workspace — the Astro docs site on 8000                                  |
| `bun run build`                    | turbo build across the workspace                                                              |
| `bun run build:rust`               | release build of the `fatima` binary                                                          |
| `bun run build:binaries`           | cross-target binaries and the WASM bundle, via the `task` crate                               |
| `bun run check:rust` / `test:rust` | `cargo check --workspace` / `cargo test --workspace`                                          |
| `bun run checktypes`               | `tsc --noEmit` and `astro check` across the packages                                          |
| `bun run lint` / `format`          | oxlint + `oxfmt --check`, without / with writing                                              |
| `bun run analyze`                  | `knip` (dead code, deps) then `jscpd` (duplication). `knip:all` adds the unused-export report |
| `bun run codegraph`                | build `.codegraph/` when missing, off CI. `bun install` runs it                               |
| `cargo run -p fatima-cli -- <cmd>` | run the CLI from source — `generate run secrets validate vault`                               |
| `bun run bump` / `version`         | move the workspace version / set it explicitly                                                |
| `bun run verify:release`           | check every manifest agrees on the version before a tag                                       |
| `bun run publish:npm` / `:crates`  | publish `@fatima.dev/js` / the crates. CI runs these, not you                                 |

`playground/` is a scratch project with its own `fatima.json` and vault, for exercising the CLI by
hand. Nothing in it is shipped.
