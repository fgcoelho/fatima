---
name: explore
description: Drive the running app in a real browser like a person and write every weird behaviour to the day file. Use when asked to explore, monkey-test, dogfood or QA the app, and on every tick of `/loop /explore`. Reporting only — never fixes.
argument-hint: "[area] [persona] [shared]"
---

# Explore

Use the Playwright MCP tools (`browser_navigate`, `browser_snapshot`, `browser_click`, `browser_type`,
`browser_fill_form`, `browser_press_key`, `browser_hover`, `browser_drag`, `browser_mouse_*`,
`browser_take_screenshot`, `browser_console_messages`, `browser_network_requests`, `browser_resize`,
`browser_evaluate`, `browser_navigate_back`, `browser_tabs`, `browser_close`) to act like a real
person on the running app and record anything a real person would call weird.

## 1. Arguments

`$ARGUMENTS` is any of: an area, a persona, or the word `shared`. `shared` means a dispatcher is
running several explorers at once against one already started app. One explorer runs one area as
one persona; there is no parallelism inside a tick, because you act sequentially and the browser
has one current tab.

## 2. The world

Fatima has no product app yet. The only app that runs today is the Astro documentation site in
`packages/web`: `bun run dev` serves it on port 8000. Its content is static, so there is no state
to wipe and no account to sign in to.

Without `shared`:

- Stop any running dev server: `pkill -f 'astro [d]ev' || true` (the bracket keeps the pattern from
  matching the pkill command's own argument), then wait until
  `curl -sf -m 5 -o /dev/null http://localhost:8000/` fails. A bare TCP connect to any `127.0.0.1`
  port succeeds on WSL with nothing listening, so the 5-second cap is what turns that into a
  failure.
- Start the dev server: `mkdir -p .playwright-mcp && bun run dev > .playwright-mcp/dev.log 2>&1 &`,
  then poll the same curl until it answers.

With `shared`, the dispatcher did that once. Never stop or restart the dev server.

Either way: the browser is `--isolated`, so its storage is empty on launch; if a browser is
already open from a previous tick, call `browser_close` first.

## 3. Pick the mission

Run `ls .agents/docs/explore/` and read the `##` section headings of the most recent files (each
heading is `<area> — <persona>`). Pick the area with the fewest sections and a persona different
from its last section, unless `$ARGUMENTS` names them. Read the sections for that area first, so
you do not re-report what they already hold — but do note in your section when one of their
findings no longer reproduces.

## 4. Areas and personas

Areas: read the site's navigation and sitemap and list its sections (landing page, docs pages,
search, theme toggle, code blocks, links). When the project gains a product app, list its
screens here instead.

Personas (fixed list):

- first-timer: reads nothing, clicks what looks obvious.
- impatient: double-clicks, navigates away mid-action, reloads while things are loading.
- power-user: keyboard shortcuts, every menu item, edge inputs, many items.
- adversarial: empty and huge inputs, unicode, pasting garbage, back button after every step, two
  tabs on the same page.

## 5. The mission

Take 20 to 40 actions in the chosen area as the chosen persona. Always `browser_snapshot` before
acting; never guess selectors. A `browser_snapshot` with a `target` invalidates every ref outside
that subtree, so take a full snapshot again before acting anywhere else. Freehand ink on the
tldraw canvas is a `browser_drag` or a mouse-move sequence, never a click.

After every state-changing action, check three signals:

- `browser_console_messages` for warnings and up.
- `browser_network_requests` for any 4xx/5xx or a request pending forever.
- new error or warning lines in the dev log: `grep -iE 'error|warn' .playwright-mcp/dev.log`.

Once per mission, also `browser_resize` to 1280x800 and to 400x800 and look for overflow, clipped
controls, or horizontal scrolling.

## 6. What counts as a finding

Uncaught console error or React warning, a failed or hanging request, a spinner or "connecting"
state longer than 10 seconds, an action with no visible effect, data that differs after a reload,
layout overflow, wrong or untranslated copy, a focus or keyboard trap, anything a normal person
would say "huh?" at.

Not a finding: frame stutter (dev builds are slower than production), a hydration warning on the
very first load of a fresh profile (reload once and continue). Under `shared`, state that changes
under you may be another explorer, so check their sections before reporting it.

## 7. On a finding

Take screenshots with `browser_take_screenshot` and a `filename` that starts with
`.playwright-mcp/` (for example `.playwright-mcp/trash-empty-state.png`); a bare filename lands
in the repo root and must not be committed. Reload and check whether it persists, try to
reproduce once more. Record: severity (`blocker` / `bug` / `papercut`), numbered repro steps,
expected vs actual, screenshot path, and the console/network/log excerpt. You may read source to
add a one-line "suspect" pointer (`path:line`), but you must not edit code.

## 8. Never

Edit or format code, commit, run the test suite, fix anything, open a second browser, or leave the
app on a different origin.

## 9. End of tick

Append one section to `.agents/docs/explore/YYYY-MM-DD.md` (today's date). If the file does not exist yet,
create it with a `# YYYY-MM-DD` heading first. The section is `## <area> — <persona>`, then one
line `- Actions: <count>`, then one `###` per finding in the format of section 7. Because several
explorers under `shared` write to the same file at once, append with a shell redirection
(`cat >> .agents/docs/explore/YYYY-MM-DD.md <<'EOF'` … `EOF`) and never rewrite the whole file with the Write
tool. Call `browser_close` so the next tick starts with empty browser storage. End the turn with
three lines: area, persona, findings count.
