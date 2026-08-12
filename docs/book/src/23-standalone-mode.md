# Standalone Mode

Standalone mode runs Mycelium with **zero external runtime dependencies**: no PostgreSQL, no Redis,
no SMTP server, no HashiCorp Vault. It is a separate build of the same gateway, selected at
**compile time**, aimed at local development, edge deployments, and small teams that don't want to
run four services just to try Mycelium out.

---

## What changes

| Full mode | Standalone mode |
|---|---|
| PostgreSQL (`adapters/diesel_postgres`) | SQLite, auto-provisioned on first boot (`adapters/diesel_sqlite`) |
| Redis-backed cache (`adapters/kv_db`) | In-process cache via [moka](https://docs.rs/moka) (`adapters/moka_cache`) |
| SMTP (`lettre` `SmtpTransport`) | Stub transport (default, logs to stdout) or file transport (`.eml` files); real SMTP also works if you opt in with `[smtp]` |
| HashiCorp Vault | Not used |
| Operator-provided `tokenSecret`/HMAC secrets | Auto-generated on first boot, persisted (keyring or an encrypted local file) |

Everything else — the REST/JSON-RPC API, authentication flows, webhooks, MCP integration — is
identical. The two builds share the same `core` domain logic; only the adapters wired in
`ports/api/src/main.rs`'s `initialize_modules` differ.

---

## Building and running

```bash
cargo build --release --no-default-features --features standalone -p mycelium-api
```

`--no-default-features` is required — Cargo features are additive, and the default
`full` feature would otherwise stay enabled and trip a `compile_error!` guard (the two
are mutually exclusive; a single binary cannot link both Postgres-only and SQLite-only Diesel column
types).

Copy `settings/config.standalone.example.toml` and point `SETTINGS_PATH` at your copy:

```bash
SETTINGS_PATH=./config.toml myc-api
```

A minimal standalone config needs only `[core.accountLifeCycle]`, `[sqlite]`, `[queue]`, `[auth]`,
and `[api]` — `[redis]` and `[vault]` don't exist in standalone builds at all (the corresponding
Rust types aren't even compiled in). `[smtp]` is optional and opt-in: absent by default (falls
back to stub/file transport), but if present, standalone sends real email through it exactly like
full mode does — see [Email transport](#email-transport) below.

Since a fresh local install rarely has a real domain to put in `noreplyEmail`/`supportEmail`,
Mycelium's email validation accepts `localhost` as a domain in addition to the usual dotted form
(`user@localhost` is valid; `user@example` without a TLD still is not) — this applies in both build
modes, not just standalone.

### Docker

```bash
docker build -f Dockerfile.standalone -t mycelium-api-standalone .
docker run -p 8080:8080 -v ./data:/data mycelium-api-standalone
```

The image bakes in `settings/config.standalone.example.toml` as its default config, with `[sqlite]
path` resolving under the mounted `/data` volume — `docker run` with no extra flags boots
out of the box. Mount your own config with `-e SETTINGS_PATH=/path -v /host/path:/path` for a real
deployment.

---

## Email transport

By default, standalone builds don't deliver email — outbound mail goes to a **stub transport**
(logs subject, recipient, and any magic-link URL to `tracing`/stdout) so you can complete flows
like magic-link login without setting up anything. An opt-in **file transport** writes a `.eml`
per message to a configured directory instead.

If you want real delivery without running full mode's Postgres/Redis/Vault stack, configure
`[smtp]` — same shape as full mode:

```toml
[smtp]
host = "smtp.example.com"
username = "apikey"
password = { env = "MYC_SMTP_PASSWORD" }
port = 587
```

When present, standalone resolves a real `SmtpTransport` and uses it (SMTP takes precedence over
file, which takes precedence over stub — the same selection logic full mode's local-transport
tests already cover). When absent, behavior is unchanged from before: stub by default.

---

## Secrets

`tokenSecret`, the primary HMAC signing key, and (if internal auth is enabled) `jwtSecret` are each
resolved on every boot in this order:

1. **An operator-supplied `{ env = "MY_VAR" }` resolver** — treated as explicit, same as full mode.
   A bare literal in the config file (what the shipped example ships) does **not** count as explicit
   — it's the documented placeholder, and standalone mode ignores it in favor of step 2.
2. The OS keyring, if a backend is available.
3. An encrypted local file next to the SQLite database (`<sqlite-dir>/.secrets/`), `0600` permissions.
4. If none of the above has a value yet: **generate** a new secret and persist it (keyring first,
   falling back to the file).

Most containers, air-gapped hosts, and CI runners have no keyring/Secret-Service daemon, so step 3
is the de-facto primary path in practice — this is expected, not a degraded mode.

> **Back up `.secrets`.** Losing this directory (with no keyring copy) means losing the ability to
> decrypt everything the KEK protects and to verify previously-signed connection strings. Treat it
> like you would a database backup.

Internal (database-backed) JWT authentication is **opt-in** via `[auth.internal.define]` — note the
table shape: `internal = "enabled"` alone does **not** work, since `OptionalConfig`'s `Enabled`
variant is a newtype wrapping a table, not a bare string:

```toml
[auth.internal.define]
jwtExpiresIn = 43200                                            # 12 hours
tmpExpiresIn = 300                                               # 5 minutes
jwtSecret = "any-placeholder-value"                       # resolved per the order above
```

When enabled this way, `jwtSecret` follows the exact same resolution order as `tokenSecret`/HMAC —
including honoring an explicit `{ env = "..." }` value.

---

## Known limitations

- **No distributed session tracking.** Not a regression today (full mode doesn't have this either),
  but future gateway-level rate limiting or retry-loop detection that needs shared state won't work
  across multiple standalone processes.
- **Single instance only — no replication.** The SQLite file and the in-process cache are node-local.
  Don't run standalone behind a load balancer with multiple replicas.
- **The cache does not persist across restarts.** Token *invalidation* is durable (it's in SQLite),
  but the TTL'd profile/JWKS cache starts cold after a restart and repopulates on demand — a brief
  cache-miss window, not a correctness problem.
- **SQLite write concurrency is limited** to one writer at a time (WAL mode is enabled). Fine for
  single-instance, low-to-moderate write volume; a bottleneck under heavy concurrent writes.
- **Email is not actually delivered** in stub mode (the default) — read the magic-link URL from
  stdout. File mode writes a `.eml` per message to a configured directory instead of sending it.
  Configure `[smtp]` (see [Email transport](#email-transport)) for real delivery.
- **The secrets file holds key material in the clear (to the process)** at rest, protected only by
  filesystem permissions plus a locally-derived wrapping key. Protect and back it up (see above).

Standalone mode is not a smaller version of full mode's guarantees dressed up differently — it makes
different, documented trade-offs in exchange for zero external dependencies. Pick the mode that
matches your deployment's actual needs.
