# Feature Spec: Config Defaults

**Feature:** config-defaults
**Status:** Specified
**Created:** 2026-07-09
**Scope:** Medium (mechanical, established `#[serde(default = "fn")]` convention already in use — no new architecture)

---

## 1. Problem Statement

Several tuning/operational config fields across the gateway's TOML config are hard-required today
(no `#[serde(default)]`, not `Option<T>`) even though they have obvious sane defaults that almost
every deployment uses unchanged (e.g. `tokenExpiration = 3600`, `[api] servicePort = 8080`). A new
operator has to copy every one of these lines from an example file just to boot, even when they
don't want to change the value. This applies identically to both build modes (full/`postgres-backend`
and `standalone`), since the affected structs (`AccountLifeCycle`, `WebhookConfig`, `QueueConfig`,
`InternalOauthConfig`, `ApiConfig`, `LoggingConfig`) are shared, not backend-specific.

Goal: make every field that is a pure tuning knob optional with a sane default, while leaving every
field that is a secret, a hostname/URL, or an operator identity value (email, domain) hard-required
— exactly the split already reflected in the field list the user provided.

---

## 2. Verified current state (baseline)

Established by direct code inspection on 2026-07-09:

| Struct | File | Requiredness today |
|---|---|---|
| `AccountLifeCycle` | `core/src/models/account_life_cycle_config.rs` | `token_expiration: SecretResolver<i64>` required; `hmac_primary_version: u32` required (both no default). `locale` is already `Option<SecretResolver<String>>` — already optional, no change needed. |
| `WebhookConfig` | `core/src/models/webhook_config.rs` | All 4 fields (`consume_interval_in_secs`, `consume_batch_size`, `max_attempts`, `accept_invalid_certificates`) are `SecretResolver<T>`, required, no defaults. |
| `QueueConfig` | `adapters/notifier/src/models/queue_config.rs` | Both fields (`email_queue_name`, `consume_interval_in_secs`) required, no defaults. |
| `InternalOauthConfig` | `lib/http_tools/src/models/internal_auth_config.rs` | `jwt_expires_in`, `tmp_expires_in` required, no defaults. `jwt_secret` stays required (secret). |
| `ApiConfig` | `ports/api/src/models/api_config.rs` | `service_ip`, `service_port`, `service_workers`, `gateway_timeout`, `allowed_origins` required, no defaults. `tls: OptionalConfig<TlsConfig>` is **missing** the `#[serde(default)]` field attribute that `AuthConfig.internal`/`.external` (same `OptionalConfig<T>` type) already carry — an existing inconsistency, not just a gap. |
| `LoggingConfig` | `ports/api/src/models/api_config.rs` | `level`, `format` required, no defaults. `target: Option<LoggingTarget>` already optional. |

`SecretResolver<T>` (`lib/config/src/domain/dtos/secret_resolver.rs`) has no `Default` impl — any
default fn for a `SecretResolver<T>`-typed field must return `SecretResolver::Value(literal)`
explicitly.

**Established convention to follow** (already used in this codebase, e.g.
`ports/api/src/models/api_config.rs`'s `default_service_id`/`default_service_protocol`,
`core/src/domain/dtos/callback/mod.rs`'s `default_retry_count`/`default_timeout`):
`#[serde(default = "fn_name")]` with a private, zero-arg `fn fn_name() -> FieldType { ... }` placed
just below the struct. Bare `#[serde(default)]` only when the type already has (or gets) its own
`impl Default`. No struct in this codebase combines a whole-struct `impl Default` with per-field
`default = "fn"` calling into it — keep that pattern (self-contained per-field default fns), don't
introduce a new one.

---

## 3. Functional requirements

Each requirement has a traceable ID. Default values are exactly what the user specified.

### `AccountLifeCycle` (`[core.accountLifeCycle]`)

- **CFG-01** — WHEN `tokenExpiration` is absent THEN it SHALL default to `3600`.
- **CFG-02** — WHEN `hmacPrimaryVersion` is absent THEN it SHALL default to `1`. (This is only the
  *version number* selecting which entry in `hmacSecrets` is primary — the secret material itself
  stays required; defaulting the version number doesn't weaken anything.)

### `WebhookConfig` (`[core.webhook]`)

- **CFG-03** — WHEN `acceptInvalidCertificates` is absent THEN it SHALL default to `true`.
- **CFG-04** — WHEN `consumeIntervalInSecs` is absent THEN it SHALL default to `30`.
- **CFG-05** — WHEN `consumeBatchSize` is absent THEN it SHALL default to `25`.
- **CFG-06** — WHEN `maxAttempts` is absent THEN it SHALL default to `5`.

### `QueueConfig` (`[queue]`)

- **CFG-07** — WHEN `emailQueueName` is absent THEN it SHALL default to `"emails"`.
- **CFG-08** — WHEN `consumeIntervalInSecs` is absent THEN it SHALL default to `15` (changed from the
  originally-spec'd `30`, user-driven follow-up: matches the value every existing example config
  already set explicitly for `[queue]`).

### `InternalOauthConfig` (`[auth.internal.define]`)

- **CFG-09** — WHEN `jwtExpiresIn` is absent THEN it SHALL default to `43200`.
- **CFG-10** — WHEN `tmpExpiresIn` is absent THEN it SHALL default to `300`.
- (`jwtSecret` is unaffected — stays required, it's a secret.)

### `ApiConfig` (`[api]`)

- **CFG-11** — WHEN `serviceIp` is absent THEN it SHALL default to `"0.0.0.0"`.
- **CFG-12** — WHEN `servicePort` is absent THEN it SHALL default to `8080`.
- **CFG-13** — WHEN `serviceWorkers` is absent THEN it SHALL default to `1`.
- **CFG-14** — WHEN `gatewayTimeout` is absent THEN it SHALL default to `60`.
- **CFG-15** — WHEN `allowedOrigins` is absent THEN it SHALL default to `[]`.
- **CFG-16** — WHEN `tls` is absent THEN it SHALL default to `"disabled"` (`OptionalConfig::Disabled`)
  — fixes the existing inconsistency where `tls` lacked the `#[serde(default)]` attribute that its
  sibling `OptionalConfig` fields (`AuthConfig.internal`/`.external`) already have.

### `LoggingConfig` (`[api.logging]`)

- **CFG-17** — WHEN `level` is absent THEN it SHALL default to
  `"mycelium_base=info,myc_api=info,myc_config=info,myc_core=info,myc_http_tools=info,actix_web=info,myc_notifier=info,myc_diesel_sqlite=info,myc_moka_cache=info"`
  (the same string already used across every example config's dev/docker/standalone variant).
- **CFG-18** — WHEN `format` is absent THEN it SHALL default to `"ansi"` (`LogFormat::Ansi`).
- **CFG-20** — WHEN `target` is absent THEN it SHALL default to `Some(LoggingTarget::Stdout)`
  (user-driven follow-up; originally out of scope as "already optional, defaults to `None`" — but
  `None` and `Some(Stdout)` already produced identical behavior in `ports/api/src/otel.rs`'s match,
  so this just makes the existing behavior explicit as a real default value instead of `None`).

---

## 4. Out of scope

| Item | Reason |
|---|---|
| `token_secret`, `hmac_secrets[].secret`, `jwt_secret` | Secrets — must stay required, no safe default exists. |
| `domain_name`, `domain_url`, `noreply_email`, `support_email`, `noreply_name`, `support_name` | Operator identity — a default would be actively misleading (wrong domain/email shipped silently). Not in the user's example list either. |
| `[diesel]`, `[redis]`, `[smtp]`, `[sqlite].path`, `[vault]` | Hostnames/paths/credentials — inherently deployment-specific, already required (or already `OptionalConfig`/absent-by-feature for standalone). Not in the user's example list. |
| `locale` (`AccountLifeCycle`) | Already `Option<SecretResolver<String>>` — genuinely already optional at the parsing level; downstream call sites already handle `None` explicitly (`core/src/use_cases/support/dispatch_notification.rs:43`, `ports/api/src/rest/index/app_public_config_endpoints.rs:67`, `ports/api/src/rpc/dispatchers/beginners.rs:660`). No code change needed — noted here only so it isn't mistaken for a missed requirement. |
| `target` (`LoggingConfig`) | Originally out of scope as already-optional (see above) — later given an explicit default anyway, see **CFG-20**. |
| `CacheConfig` wiring (`ApiConfig.cache`) | **Found during research, not requested:** `CacheConfig` already has `impl Default` (`jwks_ttl: Some(43200)`, `email_ttl: Some(600)`, `profile_ttl: Some(600)`), but `ApiConfig.cache: Option<CacheConfig>` has no `#[serde(default)]`, so an absent `[api.cache]` table yields `None` (whatever the call site does with a bare `None`), not the struct's own `Default::default()` values reachable only when constructed programmatically. This is a separate, pre-existing inconsistency — flagged for the user to decide whether to fold into this feature or track separately (see Open Concerns). |
| Runtime behavior changes | This feature only affects what a config file must contain to parse successfully. No use-case/domain logic changes. Any config that already specifies these values explicitly continues to parse identically (explicit value overrides default) — purely additive/backward compatible. |
| CI/workflow changes | None needed — same gate checks as always. |

---

## 5. Open concerns

- **OC-1** — Should `CacheConfig`'s existing `Default` impl actually be wired to serde (add
  `#[serde(default)]` to `ApiConfig.cache`, changing its type from `Option<CacheConfig>` to
  `CacheConfig` with a default, or add a `default = "..."` fn returning
  `Some(CacheConfig::default())`)? Not in the user's original list — needs a decision: fold in now,
  or defer as a separate quick task.
- **OC-2** — Example config files (`config.example.toml`, `config.for-docker.toml`,
  `config.dev.for-docker.toml`, `config.standalone.example.toml`) currently show these fields
  explicitly. Once they're optional, should the examples be trimmed to show the *minimal* required
  config (demonstrating the new defaults actually work), or left as-is (explicit-but-optional, still
  useful as documentation of the default value)? Recommend trimming at least one file (e.g.
  `config.standalone.example.toml`, already the most "minimal onboarding" example) to prove it.
  **Resolved (user-driven, follow-up pass):** `config.standalone.example.toml` trimmed (OC-2
  original decision). `config.example.toml` and `config.for-docker.toml` reviewed as beginner-facing
  documentation instead — annotated every now-optional field with `# optional -- defaults to X`
  rather than removing the lines, since these two document the *value*, not just that omission
  works. `config.for-docker.toml` was then deleted entirely (redundant with
  `config.dev.for-docker.toml`, which `docker-compose.yaml`'s `myc-api` service now mounts instead —
  it also had a real pre-existing bug, missing `hmacPrimaryVersion`/`hmacSecrets` entirely, so it
  likely never booted as configured). Personal email/name placeholders in `config.example.toml`
  (`elias.samuel.galvao@gmail.com`, `Samuel Galvão Elias`) replaced with generic
  `noreply@example.com` / `support@example.com` / `Mycelium` / `Mycelium Support`.
  `config.dev.for-docker.toml` untouched per explicit instruction.
- **OC-3** — Branch strategy: `feat/standalone-mode` currently has an open PR (#160) under review.
  This feature is unrelated to standalone-mode specifically (though it benefits both build modes) —
  recommend a fresh branch off `develop` rather than adding more commits to the already-large #160.
  There's also a pre-existing **uncommitted, unrelated** Docker Compose profiles refactor sitting in
  the working tree (`docker-compose.yaml`/`docker-compose.common.yaml`, not authored by this
  session) — noted so it isn't mistaken for something this feature touched; it will carry over
  untouched onto any new branch created from the current working tree.

---

## 6. Acceptance criteria

- [x] All 20 requirements (`CFG-01`..`CFG-19`, `CFG-20`) implemented, each with a round-trip test
  proving the field can be omitted from a minimal TOML and the struct still deserializes with the
  documented default value. `CFG-19` (OC-1, folded in): `ApiConfig.cache` changed from
  `Option<CacheConfig>` to `CacheConfig` with `#[serde(default)]`. `CFG-20` (user-driven follow-up):
  `LoggingConfig.target` defaults to `Some(LoggingTarget::Stdout)`.
- [x] Full-mode gate checks pass unchanged: `cargo fmt --all -- --check`, `cargo build --workspace`,
  `cargo test --workspace --all`.
- [x] Standalone build unaffected: `cargo build -p mycelium-api --no-default-features --features
  standalone`.
- [x] At least one example config demonstrably shrinks (fields removed because they now default),
  and still boots (verified via `scripts/standalone-e2e-smoke.sh` for the standalone example, or an
  equivalent manual/full-mode check). Done manually: `config.standalone.example.toml` trimmed,
  binary boots, `GET /health` returns `200`.
- [x] No existing config file's *explicit* values change behavior (defaults only apply when a field
  is *absent*, never override a present value).
