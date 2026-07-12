# Feature: Field-level env/vault resolution for HttpSecret.token

Source: https://github.com/LepistaBioinformatics/mycelium/issues/165

## Problem

Docs (mag.lepista.io #downstream-apis) show `token = { env = "..." }` / `token = { vault = { path, key } }`
on `HttpSecret::AuthorizationHeader` / `HttpSecret::QueryParameter`. The actual type declares
`token: String`, so any config using the documented syntax fails to parse at boot
(`data did not match any variant of untagged enum SecretResolver`).

The `SecretResolver` pattern already works one level up, on the whole `HttpSecret` via
`ServiceSecret.secret: SecretResolver<HttpSecret>` — but that forces the entire `HttpSecret`
object (including non-sensitive `headerName`/`prefix`) to live inside the env var / vault secret,
instead of just the token value, as SmtpConfig's `password: SecretResolver<String>` does today.

## Requirements

- R1: `HttpSecret::AuthorizationHeader.token` and `HttpSecret::QueryParameter.token` become
  `SecretResolver<String>` instead of `String`, matching the docs and the `SmtpConfig` precedent.
- R2: `HttpSecret::encrypt_me` / `decrypt_me` / `redact_token` only operate on
  `SecretResolver::Value(_)` tokens (literal secrets stored at rest). `Env`/`Vault` tokens pass
  through untouched — they never hold a plaintext secret at rest.
- R3: The two call sites that build outgoing HTTP requests from a resolved `HttpSecret`
  (`ports/api/src/router/inject_downstream_secret.rs`, `core/src/use_cases/support/dispatch_webhooks.rs`)
  resolve `token` via `SecretResolver::async_get_or_error()` at point of use.
- R4: Existing tests in `core/src/domain/dtos/route.rs` updated to construct
  `token: SecretResolver::Value("...".to_string())`.
- R5: No change to the outer `ServiceSecret.secret: SecretResolver<HttpSecret>` behavior — both
  levels of resolution can now be used independently or together.

## Out of scope

- `adapters/diesel_postgres/src/migration/migrate_dek.rs` (`migrate_http_secret_json`) already only
  handles literal (bare-string) tokens read from JSON — since `SecretResolver::Value(T)` is
  `#[serde(untagged)]`, a literal token still serializes as a bare string, so this migration path is
  unaffected. No change needed there.
