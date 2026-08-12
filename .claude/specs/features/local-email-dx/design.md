# Design: Local Email DX

Two threads, one file in common (`local_transport_sending.rs`). Kept independent — either can land
without the other.

---

## Thread A — Stub render

### Where

`adapters/notifier/src/repositories/local_transport_sending.rs`, in the `Ok(_)` branch of
`RemoteMessageWrite::send` for the `LocalTransportKind::Stub(_)` case. Replace the current
`tracing::info!(... body = %message.body ...)` with a stdout block.

### Rendering pipeline

1. Extract links from `message.body` (HTML) — both `href="..."` attribute values and any visible
   `http(s)://…` URLs — deduped, order-preserved.
2. Convert `message.body` (HTML) → plain text via `html2text` (`from_read` with a sane wrap width,
   e.g. 72). This collapses the table layout and inline styles into readable lines.
3. Compose a bordered block and `println!` it:

```
┌──────────────────────────────────────────────────────────────────────┐
│  STUB EMAIL — not actually delivered                                    │
├──────────────────────────────────────────────────────────────────────┤
│  To:      user@mycelium.com                                             │
│  Subject: Your Login Link                                               │
│  Links:                                                                 │
│    - https://app.example.com/magic-link/abc123                          │
├──────────────────────────────────────────────────────────────────────┤
│  Here's your login link for example.com. Click the button and you'll    │
│  see a 6-digit code to enter in the app — valid for 15 minutes...       │
└──────────────────────────────────────────────────────────────────────┘
```

Exact box-drawing is an implementation detail; the block MUST be visually distinct, list To /
Subject / Links, and include the plain-text body. Keep a small helper (e.g.
`fn render_stub_email(message: &Message) -> String`) so it is unit-testable without capturing
stdout — the `send` path just `println!`s its return value.

### Dependency

- Add `html2text` to `[workspace.dependencies]` in the root `Cargo.toml` (with a comment: used by
  the notifier's local-transport stub to render HTML emails as terminal-readable text).
- In `adapters/notifier/Cargo.toml`, add `html2text` **only** under the `local-transport` feature
  (optional dep + feature-enables it), so the full build never compiles it. Pattern:

  ```toml
  [dependencies]
  html2text = { workspace = true, optional = true }

  [features]
  local-transport = ["lettre/file-transport", "dep:html2text"]
  ```

- Verify the resolved `html2text` version's `from_read` signature at build time (recent versions
  return `Result` and take a width arg) — adapt the call to whatever version cargo resolves.

### Tests (SR)

- `render_stub_email` returns a string containing: the recipient, the subject, the magic-link URL
  as its own line, and body text with **no** `<` HTML tags. (Covers SR-R1..R5.)
- Keep the existing `stub_transport_logs_body_including_magic_link` semantics **or** replace it: the
  block now goes to stdout via `println!`, so the old `tracing`-capture assertion no longer applies.
  Re-point that test at `render_stub_email` (assert URL + recipient present) rather than capturing
  the subscriber.

---

## Thread B — File-transport wiring (#169)

### Config shape (DEC-4)

New `[localEmail]` table in standalone config, one optional field. Because it resolves to an
`OptionalConfig` (externally tagged, `Enabled` aliased to `define`/`set`), the directory lives
under the `.define` sub-table — same convention as `[auth.internal.define]`:

```toml
[localEmail.define]
dir = "./data/emails"
```

Resolves to `OptionalConfig<LocalEmailConfig>` where `LocalEmailConfig { dir: PathBuf }`. Absent
table → `Disabled` (no error), exactly like `[smtp]`.

### New config model

`adapters/notifier/src/models/local_email_config.rs` (new), mirroring `smtp_config.rs`:

```rust
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalEmailConfig {
    pub dir: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OptionalTmpConfig {
    #[serde(default)]
    local_email: OptionalConfig<LocalEmailConfig>,
}

impl LocalEmailConfig {
    pub fn from_optional_config_file(file: PathBuf) -> Result<OptionalConfig<Self>, MappedErrors> { ... }
}
```

Export from `adapters/notifier/src/models/mod.rs`. Gate the module so it only builds where needed
(mirror how `smtp_config`/`local-transport` are gated — simplest is to compile it under
`local-transport`, and only reference it from the `standalone` config handler).

### ConfigHandler wiring

`ports/api/src/models/config_handler.rs`:

```rust
#[cfg(feature = "standalone")]
pub local_email: OptionalConfig<LocalEmailConfig>,
// ...
#[cfg(feature = "standalone")]
local_email: LocalEmailConfig::from_optional_config_file(file.clone())?,
```

### main.rs call site

Replace the hardcoded `None`:

```rust
let file_dir = match config.local_email.to_owned() {
    OptionalConfig::Enabled(cfg) => Some(cfg.dir),
    OptionalConfig::Disabled => None,
};
// ...
transport: select_local_transport(smtp_transport, file_dir),
```

Update the "documented fast-follow" comment to reflect that file wiring now exists.

### Tests (FT)

- `config_handler.rs` standalone test: a config with `[localEmail.define] dir = ...` → `init_from_file`
  → `local_email` is `Enabled` → `select_local_transport(None, Some(dir))` is
  `LocalTransportKind::File`. (FT-R6 / DEC-5.)
- Existing `config_standalone_example_toml_parses_into_config_handler` must still pass with the
  commented-out `[localEmail]` in the example (i.e. absent → `Disabled`).

---

## Gating plan (the trap — both threads)

| Symbol | Gate |
|---|---|
| stub renderer + `html2text` | `local-transport` feature only |
| `LocalEmailConfig` model | `local-transport` feature only |
| `ConfigHandler.local_email` field + wiring + main.rs `file_dir` | `#[cfg(feature = "standalone")]` only |

`standalone` → enables `mycelium-notifier/local-transport`, so the config handler (standalone) can
always see `LocalEmailConfig` (local-transport). `full` compiles none of it. Confirm
with the standalone gate in spec §6.
