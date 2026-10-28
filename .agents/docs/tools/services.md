# Services

MCP servers for the services Fatima's hosted side will run on. Each reads its credentials from
environment variables, which go in your own `.env` and never in the repo.

## neon

Postgres branches, schemas and queries. Read-only: the server URL carries `readonly=true`, so it
can inspect but not change. Reads `NEON_API_KEY` and `NEON_PROJECT_ID`.

## cloudflare

Workers, DNS, R2 and the rest of the Cloudflare account. Full access to whatever the token
allows. Reads `CLOUDFLARE_API_KEY`.

## stripe

Customers, products, prices, subscriptions and invoices. Full access to whatever the key allows.
Reads `STRIPE_SECRET_KEY`. Use a restricted key in test mode while developing.

## resend

Sending email and managing domains, audiences and API keys. Full access. Reads `RESEND_API_KEY`.

## playwright

A Chromium browser the agent drives to check a page or a flow. Runs isolated, with screenshots
and traces written to `.playwright-mcp/`, which is ignored. Needs no credentials.

## search-console

Google Search Console data for `fatimajs.dev`: queries, pages, sitemaps. Scoped to that site by
`GSC_SITE_URL` and `GSC_ALLOWED_SITES`, and `GSC_DENY_TOOLS` blocks the destructive tools
(`delete_site`, `unverify_site`, `update_site_owners`, `request_indexing`, `get_indexing_status`).
Reads a service account key from `.keys/google-cloud-mcp.json`, which is ignored.

## github

Repositories, issues, pull requests and workflows. Full access to whatever the token allows.
Reads `GITHUB_API_KEY`.

## Writes need the user's say-so

A write through a service MCP is an outward-facing action: a Stripe object created, a Cloudflare
resource changed, an email sent, an issue or pull request opened. Ask the user before doing one,
every time, even when the tool would let you. Reads need no permission.
