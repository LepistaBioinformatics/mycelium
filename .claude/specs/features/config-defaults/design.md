# Design: Config Defaults

**Feature:** config-defaults
**Spec:** `./spec.md`
**Status:** Design
**Created:** 2026-07-11

Resolves OC-1..OC-3 from `spec.md` and lays out the mechanical per-field changes for CFG-01..CFG-18.

---

## 1. Open concerns — resolutions

- **OC-1 (fold in):** Yes. Add `#[serde(default)]` to `ApiConfig.cache` and change its type from
  `Option<CacheConfig>` to `CacheConfig` (the field is never actually optional in behavior — every
  call site either uses `CacheConfig::default()` values or the configured ones; `None` was never a
  meaningful third state). Tracked as **CFG-19**.
- **OC-2 (trim):** After CFG-01..19 land, remove now-default-equal lines from
  `settings/config.standalone.example.toml` only (the "minimal onboarding" example), proving the
  defaults work end-to-end. The other example files stay explicit — they document the values, not
  just the minimum.
- **OC-3 (branch):** `feat/config-defaults`, off `develop` (already created, now fast-forwarded to
  latest `develop` including merged PR #160).

**Follow-up, user-driven (after initial implementation landed):**
- `LoggingConfig.target` given an explicit default too (**CFG-20**): `Some(LoggingTarget::Stdout)`.
  `otel.rs`'s match already treated `None` and `Some(Stdout)` identically (both fall into the
  wildcard arm → logs to stderr, a pre-existing naming/behavior mismatch left untouched), so this
  doesn't change behavior — it just gives the field a real default instead of `None`.
- `QueueConfig.consume_interval_in_secs`'s default changed from `30` to **`15`** (every existing
  example config already set it to `15` explicitly; the spec'd `30` was never actually used).
- `config.example.toml` and `config.for-docker.toml` reviewed as beginner onboarding docs: every
  now-optional field annotated inline (`# optional -- defaults to X`) rather than removed, since
  unlike the standalone example these two exist to document the value, not just prove omission
  works.
- `config.for-docker.toml` deleted — redundant with `config.dev.for-docker.toml` (which
  `docker-compose.yaml`'s `myc-api` service now mounts instead), and it had a real latent bug:
  missing `hmacPrimaryVersion`/`hmacSecrets` entirely, so `AccountLifeCycle` (which still requires
  `hmac_secrets`, no default — secrets never get one) would have failed to deserialize.
- Personal email/name in `config.example.toml` replaced with generic placeholders
  (`noreply@example.com`, `support@example.com`, `Mycelium`, `Mycelium Support`).

---

## 2. Per-struct changes

Convention (per spec.md §2): `#[serde(default = "fn_name")]` + private zero-arg `fn fn_name() -> T`
placed directly below the struct. `SecretResolver<T>` fields default via
`SecretResolver::Value(literal)`.

### `core/src/models/account_life_cycle_config.rs` — `AccountLifeCycle`

```rust
#[serde(default = "default_token_expiration")]
pub token_expiration: SecretResolver<i64>,
...
#[serde(default = "default_hmac_primary_version")]
pub(crate) hmac_primary_version: u32,
```
```rust
fn default_token_expiration() -> SecretResolver<i64> {
    SecretResolver::Value(3600)
}

fn default_hmac_primary_version() -> u32 {
    1
}
```

### `core/src/models/webhook_config.rs` — `WebhookConfig`

All four fields get `#[serde(default = "fn")]`:

```rust
fn default_consume_interval_in_secs() -> SecretResolver<u64> { SecretResolver::Value(30) }
fn default_consume_batch_size() -> SecretResolver<u64> { SecretResolver::Value(25) }
fn default_max_attempts() -> SecretResolver<u64> { SecretResolver::Value(5) }
fn default_accept_invalid_certificates() -> SecretResolver<bool> { SecretResolver::Value(true) }
```

### `adapters/notifier/src/models/queue_config.rs` — `QueueConfig`

```rust
fn default_email_queue_name() -> SecretResolver<String> {
    SecretResolver::Value("emails".to_string())
}

fn default_consume_interval_in_secs() -> SecretResolver<u64> {
    SecretResolver::Value(15) // changed from originally-spec'd 30, see CFG-08
}
```

### `lib/http_tools/src/models/internal_auth_config.rs` — `InternalOauthConfig`

```rust
fn default_jwt_expires_in() -> SecretResolver<i64> {
    SecretResolver::Value(43200)
}

fn default_tmp_expires_in() -> SecretResolver<i64> {
    SecretResolver::Value(300)
}
```

`jwt_secret` untouched (stays required).

### `ports/api/src/models/api_config.rs` — `ApiConfig` + `LoggingConfig`

`ApiConfig`:
```rust
#[serde(default = "default_service_ip")]
pub service_ip: String,
#[serde(default = "default_service_port")]
pub service_port: u16,
#[serde(default)]
pub allowed_origins: Vec<String>,           // Vec already implements Default -> bare `default`
#[serde(default = "default_service_workers")]
pub service_workers: i32,
#[serde(default = "default_gateway_timeout")]
pub gateway_timeout: u64,
pub logging: LoggingConfig,
#[serde(default)]
pub tls: OptionalConfig<TlsConfig>,          // matches AuthConfig.internal/.external pattern
#[serde(default)]
pub cache: CacheConfig,                      // CFG-19, type changed from Option<CacheConfig>
```

```rust
fn default_service_ip() -> String { "0.0.0.0".to_string() }
fn default_service_port() -> u16 { 8080 }
fn default_service_workers() -> i32 { 1 }
fn default_gateway_timeout() -> u64 { 60 }
```

`LoggingConfig`:
```rust
#[serde(default = "default_logging_level")]
pub level: String,
#[serde(default)]
pub format: LogFormat,                       // needs `impl Default for LogFormat` (-> Ansi)
#[serde(default = "default_logging_target")]
pub target: Option<LoggingTarget>,           // CFG-20, defaults to Some(LoggingTarget::Stdout)
```

```rust
fn default_logging_level() -> String {
    "mycelium_base=info,myc_api=info,myc_config=info,myc_core=info,myc_http_tools=info,\
     actix_web=info,myc_notifier=info,myc_diesel_sqlite=info,myc_moka_cache=info".to_string()
}
```

`LogFormat` currently has no `Default` impl (it's a bare enum, no unit variant tagging). Add:
```rust
impl Default for LogFormat {
    fn default() -> Self {
        LogFormat::Ansi
    }
}
```
(This is the one place a bare `#[serde(default)]` is used instead of `default = "fn"` — consistent
with the stated exception: "bare `#[serde(default)]` only when the type already has (or gets) its
own `impl Default`".)

---

## 3. `CacheConfig` type change (CFG-19) — call-site impact

`ApiConfig.cache: Option<CacheConfig>` → `CacheConfig`. Every existing call site must be checked and
updated (search `\.cache` on `ApiConfig`/`config.api.cache`). Expected shape of the fix: sites that
today do `config.cache.unwrap_or_default()` or `config.cache.as_ref().map(...)` simplify to using
the value directly; sites that already assumed `Some` unconditionally (if any) are unaffected.

---

## 4. Round-trip test pattern (one per requirement)

Each touched struct gets a test (or extends the existing `#[cfg(test)] mod tests`) proving: a TOML
fragment omitting the field deserializes successfully and yields the documented default. Example
shape (`WebhookConfig`):

```rust
#[test]
fn webhook_config_defaults_when_fields_absent() {
    let toml = "";
    let config: WebhookConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.consume_interval_in_secs, SecretResolver::Value(30));
    assert_eq!(config.consume_batch_size, SecretResolver::Value(25));
    assert_eq!(config.max_attempts, SecretResolver::Value(5));
    assert_eq!(config.accept_invalid_certificates, SecretResolver::Value(true));
}
```

Structs without a direct `Deserialize` entry point in isolation (`AccountLifeCycle`,
`ApiConfig`/`LoggingConfig` — nested under `[core.accountLifeCycle]`/`[api]`) get the fragment
wrapped in a minimal enclosing table matching real config shape, reusing whatever `TmpConfig`
wrapper the file already defines where present, or a local one-off `#[derive(Deserialize)]` test
wrapper otherwise.

`SecretResolver<T>` requires `PartialEq` for `assert_eq!` — confirm it derives `PartialEq` (used
already in `account_life_cycle_config.rs`'s existing tests, e.g.
`token_secret_resolver_exposes_the_configured_value`), so no new derive needed.

---

## 5. Example config trimming (OC-2)

In `settings/config.standalone.example.toml` only, remove lines that now equal the default:
`tokenExpiration`, `hmacPrimaryVersion`, all of `[core.webhook]`, `[queue].consumeIntervalInSecs`
(keep `emailQueueName` only if it differs — it doesn't, so it can drop too, pending re-check),
`jwtExpiresIn`, `tmpExpiresIn`, `serviceIp`, `servicePort`, `serviceWorkers`, `gatewayTimeout`,
`allowedOrigins` (only if empty — currently non-empty, so it stays explicit), `tls`,
`[api.logging].level` and `.format`. Add one comment noting these now have defaults, so a reader
comparing against `config.example.toml` isn't confused by the asymmetry.

---

## 6. Verification

- `cargo fmt --all -- --check`
- `cargo build --workspace`
- `cargo test --workspace --all` (new round-trip tests included)
- `cargo build -p mycelium-api --no-default-features --features standalone`
- `scripts/standalone-e2e-smoke.sh` against the trimmed `config.standalone.example.toml`
