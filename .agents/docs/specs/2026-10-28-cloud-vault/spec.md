# Cloud vault

- Date: 2026-10-28
- Status: Approved (sections 1 to 4 approved in conversation; section 5 and the rulings decided under yolopowers)

## Why

Fatima keeps secrets in an encrypted file inside the repository. Teams want the same secrets
without passing a vault password around, and without committing ciphertext next to the code. Fatima
grows a hosted SaaS for that, built on the same Rust engine, which runs natively in the CLI and as
WASM in the browser. The local file vault stays as the free, offline, open-source mode.

## Target

Four sub-projects, in this order. Each one gets its own plan under this directory.

1. **Engine v2.** The crate becomes `fatima-engine` in `crates/engine`, and the vault file moves to
   format version 2 with the key hierarchy below. No cloud.
2. **SaaS skeleton.** The studraw platform carved out and laid over this repository. Astro goes.
3. **Cloud vault.** The API, the browser app, teams, grants, machine tokens and CLI sync.
4. **Billing.** Plans and limits.

1 and 2 are independent. 3 needs both. 4 needs 3.

## Principles

- **End-to-end encrypted.** The server stores ciphertext, identifiers and timestamps. It can never
  decrypt a secret. A lost master password means lost data, the way Bitwarden works, and the product
  says so before an account is created.
- **One engine.** Every operation on secrets lives in the Rust engine. The CLI, `@fatima.dev/js` and
  the browser app call it; none of them reimplements crypto.
- **The local vault is a product, not a fallback.** It keeps working with no account and no network.

## 1. Engine v2

### The rename

`crates/core` becomes `crates/engine`, and the crate `fatima-core` becomes `fatima-engine` (Rust
path `fatima_engine`). `fatima-engine` is published as a new crate. `fatima-core` stays on
crates.io at its last version and receives no further releases.

### Primitives

RustCrypto and dalek crates, all of which build for native and `wasm32-unknown-unknown`:

| Purpose                         | Primitive                                            |
| ------------------------------- | ---------------------------------------------------- |
| Master password to master key   | Argon2id, 64 MiB memory, 3 iterations, 4 lanes        |
| Splitting a key by purpose      | HKDF-SHA256 with a distinct `info` per purpose        |
| Symmetric encryption            | XChaCha20-Poly1305 with associated data               |
| Sealing a key to a member       | X25519 sealed box (`crypto_box`)                      |

The Argon2id parameters are stored next to the salt in every vault, so they can be raised later
without breaking an existing vault.

### Key hierarchy

- **Master key**: Argon2id over the master password and a random 16-byte salt.
- **Encryption key and login key**: HKDF splits the master key. The encryption key wraps the
  account key. The login key is what the cloud authenticates with (section 4); the local vault
  has no use for it and does not derive it.
- **Account key**: random 32 bytes, wrapped by the encryption key. Changing the master password
  re-wraps this one value and nothing else.
- **Account key pair**: X25519. The public half is stored in clear; the private half is wrapped by
  the account key.
- **Environment key**: random 32 bytes per environment, sealed to each member's public key. One
  sealed copy is a grant. The local vault has exactly one member, its own account.
- **Values**: every secret name and value is encrypted with XChaCha20-Poly1305 under its
  environment key. The associated data is
  `fatima/v2/<projectId>/<environment>/<secretId>/<field>`, where `field` is `name` or `value`, so
  a ciphertext copied into another project, environment, secret or field fails to decrypt. Cloud
  records add their version to the associated data (section 4).
- **User keys** (`fatima_user_<id>_<secret>`): the 32-byte random secret goes through HKDF, not a
  password KDF, since it is not a password. The derived key wraps the environment keys the user key
  may read, and a second HKDF output is stored as the hash that identifies it.
- **User secrets** (a user key's own overrides): encrypted under a per-user-key, per-environment
  key, which is wrapped twice: by the account key and by the user key.

### File format 2

`.fatima/vault.json` gains `"version": 2`, a `projectId`, the Argon2id parameters in `kdf`,
`"xchacha20-poly1305"` in `encryption`, an `account` object (wrapped account key, public key,
wrapped private key), and one grant per environment in place of the password-wrapped environment
secret. Secrets and user secrets keep their v1 shape (an id, plus a name and a value per
environment), encrypted with the new primitives.

### Version 1 files

A version 1 file opens with the master password exactly as before, through the v1 code, which is
kept for reading only. The next save writes version 2: a new account key and key pair, an Argon2id
master key from the same password, and every value re-encrypted. **Environment keys keep their
bytes across the upgrade**, so a user key issued against the version 1 file keeps working: its
record is kept with its v1 scheme (scrypt and AES-256-GCM) and is read through the v1 code. New
user keys use the v2 scheme. Only a save from a password session upgrades a file. A user key
holds no master password, so a user secret written through a user key into a version 1 file is
written in the version 1 encoding and the file stays at version 1 until its next password save.

### Not in this sub-project

The storage interface of section 2 and the move of vault I/O behind `FatimaHost`. Today the vault
reads and writes through `std::fs` directly, which the browser cannot use. Both arrive with the
cloud backend in sub-project 3, where a second implementation exists to shape the interface.

## 2. Storage interface and the engine's API

`fatima-engine` does no I/O of its own. A `VaultStore` interface has two implementations, both of
which reach the outside world only through `FatimaHost`: `FileStore` over the host's file methods,
and `CloudStore` over `host.http_json`. The CLI, the browser and `@fatima.dev/js` get both backends,
and no host method is added.

The interface covers project metadata, listing an environment's records, writing records with the
version the writer expects, listing, adding and revoking grants, machine tokens, and the caller's
own sealed keys. A write that names a stale version fails with a conflict variant of
`FatimaError`. The engine reloads and shows the conflict, and never merges silently.

`fatima.json` gains `store`: `"local"`, the default and today's behaviour, or
`{ "cloud": { "project": "<id>" } }`. `static/schema.json` is regenerated from the config model; a
file without the field is local.

The CLI follows the Bitwarden CLI. `fatima login` takes email and master password and keeps the
account key in `~/.config/fatima/`, encrypted under a random session key. `fatima unlock` prints
that session key once, to be exported as `FATIMA_SESSION`. `fatima lock` forgets it. CI uses
`FATIMA_TOKEN`, a machine token, with no login. `fatima vault push` copies a local vault into a
cloud project: it creates the environments, re-encrypts every record under new environment keys,
seals them to the pusher, and switches `store` to cloud.

The JSON API (`handle_api_request_json`) gains the account, grant, token and push commands. The
browser app calls the same commands the CLI does.

A cloud project has no offline cache: `fatima run` against it needs the network.

## 3. SaaS skeleton

Carved from studraw's current tree with the template method: `git archive HEAD`, delete the study
product, substitute `studraw`, `Studraw` and `studraw.com` with `fatima`, `Fatima` and `fatima.dev`,
and let `bun run knip` name what in `libs/` and in the dependencies became orphaned.

Kept: server slices identity, user, access, billing, email, blob, trash, shared and devtools; ui
slices auth, settings, billing, admin, shell, site and widget; studraw's `folder` slice renamed to
`example`; the Inversify container, the oRPC contracts, Better Auth with its Stripe plugin, Drizzle
on Neon, Resend, the site and API Workers, OpenTofu infra minus Discord, the CI workflows,
`layout.json` and its oxlint plugin, husky, the maintenance and database CLIs, and the blog's MDX
pipeline. Cut: every study-shaped slice with its contracts, routes, i18n namespaces and migrations.

Studraw's root directories land beside `crates/`. `packages/js` stays the `@fatima.dev/js`
workspace and the site imports the engine from it. `packages/web` is deleted and its MDX pages move
under the site's `/docs`, keeping their URLs where possible. `packages/lsp` gives way to studraw's
`libs/@meta/lsp`. The root configs merge, and the Rust gates join the TypeScript ones in `done.md`
and the hooks. The site Worker serves `fatima.dev`; the API Worker is `fatima` at `api.fatima.dev`.
`layout`, `explore` and the contracts routing join `AGENTS.md`.

Done when typecheck, lint and tests pass, the site runs locally, a person can sign up, sign in and
open billing, and the example slice works from the screen to the database.

## 4. Cloud vault

**Sign in.** Better Auth's email and password sign-in receives the login key of section 1, never
the master password, and hashes it again. Before sign-in the server returns the account's salt and
Argon2id parameters. There is no OAuth sign-in, since a provider sign-in would need a second
password to unlock the vault.

**Server slices.** `keyring` (Argon2id parameters, salt, wrapped account key, public key, wrapped
private key), `team` (teams, members with roles owner, admin and member, invitations), `vault`
(projects owned by a team, environments, records: secret id, version, ciphertext, nonce, author,
time; a write appends a version), `grant` (a member's sealed environment key), `token` (name, hash,
wrapped environment keys, expiry, last use) and `audit` (who did what to which secret id, when, and
never a value). The server cannot read anything, but it enforces who may fetch ciphertext:
membership and a grant, or a token's scope.

**Invite.** The invitee signs up, which creates their key pair. An admin who holds the environment
keys confirms them; the admin's browser seals the keys to the new public key and shows its
fingerprint for an out-of-band check.

**Remove.** The admin's client rotates each environment the member could read: a new environment
key, every record re-encrypted, every remaining grant re-sealed. The server swaps keys, records and
grants in one transaction checked against the version, so a rotation that races an edit fails
whole.

**Browser app.** An unlock screen keeps keys in memory only and locks after inactivity. Projects
and environments show a table of secrets decrypted in WASM, editable, with per-secret history,
members and grants per environment, machine tokens, and the audit log. Every operation goes through
the engine's JSON API.

**Machine tokens** replace user keys in the cloud. A token is an id and a random secret; HKDF turns
the secret into the key that wraps the environment keys it may read; the server keeps only a hash.

Out of scope: OAuth and SSO, sharing between teams, an offline cloud cache, webhooks.

## 5. Billing

The free plan is the local vault and nothing else. Paid plans are per seat on a team and are
designed in their own round, with Better Auth's Stripe plugin from the skeleton. Machine tokens and
the audit log are paid features.

## Errors and testing

Every new failure is a `FatimaError` variant with fields where it has a shape: a wrong master
password, a ciphertext that fails authentication, an unsupported vault version, a version conflict.
None of them carries key material or plaintext.

Engine behaviour is tested from `crates/engine/tests/` against the public API, including a
committed version 1 vault fixture that must open, upgrade on save, and keep its user key working.
Primitives without a public surface are tested beside their module.

## Rulings

- Ruling: sub-project 1 is the only one built in this run; 2, 3 and 4 stay designed here and wait for their own plans. Yolopowers builds the first independent subsystem only. Cost if wrong: none; the order is the one approved.
- Ruling: the `VaultStore` interface and the move of vault I/O behind `FatimaHost` move from sub-project 1 to sub-project 3. An interface with one implementation is shaped by guesswork; the cloud backend is its second implementation. Cost if wrong: sub-project 3 carries a larger refactor of `store.rs`.
- Ruling: environment keys keep their bytes across the v1 to v2 upgrade. It is the only way a user key already in someone's CI keeps working without its raw value being re-entered. Cost if wrong: an environment key from v1 lives on until the first rotation, which sub-project 3 introduces.
- Ruling: the associated data of local values carries no version; cloud records add it. A local file has no concurrent writers for a version to protect against. Cost if wrong: a local value could be rolled back to an older ciphertext of the same secret by someone who can already write the file.
- Ruling: user keys keep their name and `fatima_user_` prefix in sub-project 1; machine tokens are their cloud successor. Renaming a CLI surface is a compatibility change with no payoff before the cloud exists. Cost if wrong: one rename later.
- Ruling: crypto crates are taken from the RustCrypto generation that matches the workspace's `rand 0.8` (`argon2 0.5`, `chacha20poly1305 0.10`, `crypto_box 0.9`, `hkdf 0.12`, `sha2 0.10`), not the newest pre-releases. Mixing RNG trait generations breaks the build. Cost if wrong: a dependency bump later.
- Ruling: Argon2id defaults are 64 MiB, 3 iterations, 4 lanes, Bitwarden's defaults. Cost if wrong: unlock is slower or weaker than wanted; the parameters live in the file and can move.
- Ruling: a user key writing a user secret into a version 1 file keeps the file at version 1. The user key cannot derive the new account material, and refusing the write would break an existing workflow. Cost if wrong: a file nobody unlocks with the password stays at version 1.
- Ruling: the associated data that wraps environment keys for a user key names the user key but not the environment. Renaming an environment from a password session cannot re-wrap what only the user key can open, and a wrapped key moved between one user key's own environments opens nothing new, since values stay bound to their environment. Cost if wrong: none found.
- Ruling: billing is sketched, not designed: free is local only, paid is per seat. Billing needs the cloud vault to exist before its limits mean anything. Cost if wrong: the section is rewritten in its round.
- Ruling: the ship stage pushes `development` to origin. AGENTS.md says to push only when the user says "push", and the user invoking yolopowers, whose last stage is the push, is that instruction for this run. `fatima` and tags are never pushed. Cost if wrong: commits reach `origin/development` before the user read them; a branch push triggers no release.
