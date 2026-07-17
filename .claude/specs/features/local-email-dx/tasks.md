# Tasks: Local Email DX

Threads A and B are independent. Within each, tasks are mostly sequential. `[P]` = parallelizable
with the other thread.

---

## Thread A — Stub render

### A1 [P] — Add `html2text` dependency (feature-gated)
- **Where:** root `Cargo.toml` `[workspace.dependencies]`; `adapters/notifier/Cargo.toml`.
- **Reuses:** existing `local-transport` feature.
- **Done when:** `html2text` is a workspace dep (with justification comment); notifier has it as
  `optional = true` and `local-transport = [..., "dep:html2text"]`; `cargo build --no-default-features
  --features standalone` still builds; default `cargo build --workspace` does **not** compile it.
- **Satisfies:** SR-R6, DEC-2.

### A2 — Implement `render_stub_email` + wire into `send`
- **Where:** `adapters/notifier/src/repositories/local_transport_sending.rs`.
- **Depends on:** A1.
- **Reuses:** `Message` DTO.
- **Done when:** a `fn render_stub_email(message: &Message) -> String` extracts links (href +
  visible URLs, deduped), converts the HTML body to plain text via `html2text`, and returns a
  bordered block listing To / Subject / Links / body. The stub branch in `send` replaces
  `tracing::info!(... body = %message.body ...)` with `println!("{}", render_stub_email(&message))`.
- **Satisfies:** SR-R1..R5.

### A3 — Tests for stub render
- **Where:** same file, `#[cfg(test)]`.
- **Depends on:** A2.
- **Done when:** a unit test asserts `render_stub_email` output contains recipient, subject, the
  magic-link URL on its own line, and no `<`-prefixed HTML tags. The existing
  `stub_transport_logs_body_including_magic_link` is re-pointed at `render_stub_email` (stdout, not
  tracing capture) or removed in favor of the new test.
- **Satisfies:** SR-R1..R5.
- **Gate:** `cargo test -p mycelium-notifier --no-default-features --features local-transport`.

---

## Thread B — File-transport wiring (#169)

### B1 [P] — `LocalEmailConfig` model
- **Where:** new `adapters/notifier/src/models/local_email_config.rs`; export in `models/mod.rs`.
- **Reuses:** `smtp_config.rs` `OptionalConfig`/`from_optional_config_file` pattern.
- **Done when:** `LocalEmailConfig { dir: PathBuf }` + `from_optional_config_file(file) ->
  Result<OptionalConfig<Self>, MappedErrors>` compiles under `local-transport`; absent `[localEmail]`
  → `Disabled`.
- **Satisfies:** FT-R1.

### B2 — ConfigHandler field + loading (standalone-gated)
- **Where:** `ports/api/src/models/config_handler.rs`.
- **Depends on:** B1.
- **Done when:** `#[cfg(feature = "standalone")] pub local_email: OptionalConfig<LocalEmailConfig>`
  added and loaded via `LocalEmailConfig::from_optional_config_file` in `init_from_file`.
  Postgres-backend build untouched.
- **Satisfies:** FT-R1, FT-R4.

### B3 — main.rs wiring
- **Where:** `ports/api/src/main.rs` (~line 1207-1227).
- **Depends on:** B2.
- **Done when:** `file_dir` derived from `config.local_email` and passed to
  `select_local_transport(smtp_transport, file_dir)`, replacing `None`. Fast-follow comment updated.
- **Satisfies:** FT-R2, FT-R3.

### B4 — Example config docs
- **Where:** `settings/config.standalone.example.toml`.
- **Depends on:** B2 (field name settled).
- **Done when:** a commented `[localEmail]` section documents `dir`, precedence (SMTP > File > Stub),
  and `.eml`-per-message behavior, matching the `[smtp]` section's doc style. The uncommented example
  config must still parse (field absent → `Disabled`).
- **Satisfies:** FT-R5.

### B5 — ConfigHandler end-to-end test
- **Where:** `ports/api/src/models/config_handler.rs` `#[cfg(test)]`.
- **Depends on:** B2, B3.
- **Done when:** a standalone test builds a config with `[localEmail] dir=...`, runs
  `init_from_file`, asserts `local_email` is `Enabled`, and that
  `select_local_transport(None, Some(dir))` is `LocalTransportKind::File`. Existing example-parse
  test still passes.
- **Satisfies:** FT-R6, DEC-5.

---

## Final gate (both threads)

```bash
cargo fmt --all -- --check
cargo build --workspace && cargo test --workspace --all      # postgres-backend clean
cargo build --no-default-features --features standalone
cargo test  --no-default-features --features standalone
```

## Docs / release
- CHANGELOG entry in `adapters/notifier/CHANGELOG.md` (and any conventional release notes) covering
  both threads.
