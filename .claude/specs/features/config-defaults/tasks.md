# Tasks: Config Defaults

**Feature:** config-defaults
**Design:** `./design.md`
**Status:** Done (pending final review)

- [x] CFG-T0 — Fast-forward `feat/config-defaults` to latest `develop` (includes merged PR #160).
- [x] CFG-T1 — `AccountLifeCycle` (CFG-01, CFG-02): defaults + round-trip test.
- [x] CFG-T2 — `WebhookConfig` (CFG-03..06): defaults + round-trip test.
- [x] CFG-T3 — `QueueConfig` (CFG-07, CFG-08): defaults + round-trip test.
- [x] CFG-T4 — `InternalOauthConfig` (CFG-09, CFG-10): defaults + round-trip test.
- [x] CFG-T5 — `ApiConfig` tuning fields (CFG-11..16: serviceIp/Port/Workers/gatewayTimeout/
      allowedOrigins/tls) + round-trip test.
- [x] CFG-T6 — `LoggingConfig` (CFG-17, CFG-18: level/format, incl. `impl Default for LogFormat`)
      + round-trip test.
- [x] CFG-T7 — CFG-19 (OC-1): `ApiConfig.cache` `Option<CacheConfig>` → `CacheConfig`, update the
      3 call sites in `check_credentials_with_multi_identity_provider.rs` (x2) and
      `recovery_profile_from_storage_engines.rs` (x1).
- [x] CFG-T8 — Full gate: `cargo fmt --all -- --check`, `cargo build --workspace`,
      `cargo test --workspace --all`. All green (208+22+9+... tests, 0 failures).
- [x] CFG-T9 — Standalone build: `cargo build -p mycelium-api --no-default-features --features standalone`.
- [x] CFG-T10 — Trim `settings/config.standalone.example.toml` (OC-2). `standalone-e2e-smoke.sh`
      generates its own inline config (doesn't reference the example file), so verified instead by
      booting the real binary with `SETTINGS_PATH` pointed at the trimmed file directly —
      `GET /health` returned `200`.
- [x] CFG-T11 — Update `spec.md` acceptance criteria checkboxes.
- [x] CFG-T12 — CFG-20 (user-driven follow-up): `LoggingConfig.target` defaults to
      `Some(LoggingTarget::Stdout)` instead of `None`.
- [x] CFG-T13 — CFG-08 value change (user-driven follow-up): `QueueConfig.consume_interval_in_secs`
      default changed `30` → `15`.
- [x] CFG-T14 — Beginner-documentation pass on `config.example.toml` and `config.for-docker.toml`
      (user-driven follow-up): annotated every now-optional field inline, fixed "SETTINGS SETTINGS"
      typos.
- [x] CFG-T15 — Deleted `config.for-docker.toml` (user-driven follow-up): redundant with
      `config.dev.for-docker.toml`; repointed `docker-compose.yaml`'s `myc-api` service volume mount
      to it. Removed personal email/name from `config.example.toml`.
- [x] CFG-T16 — Full gate re-verified green after CFG-T12..15 (fmt/build/test all pass).

Not yet done: commit + push + PR.
