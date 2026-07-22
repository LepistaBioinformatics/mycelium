# Feature Spec: Local Email DX (Human-Friendly Stub Render + File-Transport Wiring)

**Feature:** local-email-dx
**Milestone:** Standalone / local dev experience
**Status:** Implemented (2026-07-17) — pending user UAT before commit
**Created:** 2026-07-17
**Scope:** Large (two independent threads sharing one adapter file: a terminal-UX change in the
stub transport and the config-plumbing fast-follow for the file transport, issue #169). Both are
`standalone`/`local-transport`-feature-gated and must not touch the `full` build.

---

## 1. Objective

Two independent pain points in the standalone build's local email path, fixed in one pass:

- **Thread A — Stub render (UX).** When neither SMTP nor a file directory is configured, the local
  transport falls through to the log-only **stub** (`StubTransport`). Today it logs the raw HTML
  `body` via `tracing::info!` (`body = %message.body`). The email body is a full HTML document
  (table-based layout, inline styles), so reading a magic-link URL/code out of the terminal is
  painful. Make the stub print a **human-friendly, bordered ASCII block to stdout** with the
  recipient, subject, extracted links, and the HTML rendered down to plain text.

- **Thread B — File transport wiring (issue #169).** The file transport (`.eml` per message) is
  fully implemented and unit-tested in `select_local_transport(smtp, file_dir)`, but is
  **unreachable from config**: the only call site (`ports/api/src/main.rs`) hardcodes the second
  argument to `None`, with a comment calling the wiring a "documented fast-follow". Add an optional
  standalone-only config field resolving to a directory path, wire it through `ConfigHandler` into
  the `select_local_transport` call, keeping precedence **SMTP > File > Stub**.

Both threads target the same user: someone running a standalone deployment for local dev / CI /
demo who does not want to stand up Postgres + real SMTP but needs to read magic-link codes.

---

## 2. Verified current state (baseline)

Established by direct code inspection on 2026-07-17.

| Concern | Reality |
|---|---|
| Transport selection | `select_local_transport(smtp: Option<SmtpTransport>, file_dir: Option<PathBuf>) -> LocalTransportKind` (`adapters/notifier/src/repositories/local_transport_sending.rs`). Precedence SMTP > File > Stub. Three unit tests already cover the selection matrix; **do not change this function's logic.** |
| Stub logging | In `LocalTransportMessageSendingRepository::send`, after a successful stub send: `tracing::info!(subject, to, body = %message.body, "Stub transport: email not actually delivered")`. `body` is a full HTML document. Test `stub_transport_logs_body_including_magic_link` asserts the log output contains the URL and recipient. |
| Message DTO | `core/src/domain/dtos/message.rs` — `Message { from, to, cc, subject, body }`. `body` is a single `String` and is **always HTML** (`build_lettre_message` sets `ContentType::TEXT_HTML`). No plain-text alternative is plumbed anywhere. |
| Email templates | `templates/{en-us,es,pt-br}/email/*.jinja`, table-based HTML layouts with inline CSS (e.g. `magic-link-request.jinja`, `password-reset-*`). The stub is generic — it serves every template, not just magic-link. |
| Config call site | `ports/api/src/main.rs` ~line 1227: `transport: select_local_transport(smtp_transport, None)`. `smtp_transport` derives from `config.smtp: OptionalConfig<SmtpConfig>` (standalone). Comment above: "File-transport configuration wiring remains a documented fast-follow." |
| ConfigHandler | `ports/api/src/models/config_handler.rs`. `smtp` is `#[cfg(feature = "standalone")] pub smtp: OptionalConfig<SmtpConfig>`, loaded via `SmtpConfig::from_optional_config_file`. Postgres-backend has its own mandatory `smtp: SmtpConfig`. Has a standalone test `config_standalone_example_toml_parses_into_config_handler`. |
| SmtpConfig loader pattern | `adapters/notifier/src/models/smtp_config.rs` — `TmpConfig`/`OptionalTmpConfig` newtype wrappers + `from_default_config_file`/`from_optional_config_file` returning `OptionalConfig<SmtpConfig>` (`#[serde(default)]`). This is the pattern to mirror for the new file-dir config. |
| Feature gating | `local-transport` (notifier) gates the transport code and enables `lettre/file-transport`. `standalone` (`ports/api/Cargo.toml`) enables `mycelium-notifier/local-transport` — so both halves compile together in the standalone build. `full` compiles **none** of this code. |
| Example config | `settings/config.standalone.example.toml` — documents `[sqlite]`, `[queue]`, commented `[smtp]`. No file-transport section exists. |
| HTML→text dep | No HTML-to-text crate exists anywhere in `Cargo.lock`. `regex = "1"` is a workspace dep but notifier does not currently depend on it. |

---

## 3. Decisions (resolved gray areas)

- **DEC-1 (Thread A output channel).** Stub renders a **bordered ASCII block via `println!` to
  stdout**, dev-only. Rationale: prettiest/most readable for local dev; deliberately bypasses log
  level/format config so it is **not** shipped to structured sinks (JSON logs / SigNoz). The
  existing `tracing::info!` line is replaced by this block (the stub is only ever selected in
  standalone local runs; there is no observability value in shipping "email not delivered" HTML to
  SigNoz). *(User decision, 2026-07-17.)*

- **DEC-2 (Thread A HTML→text).** No existing workspace crate does HTML→text; per user guidance use
  the **`html2text`** crate (added as a workspace dep, justified by DEC-1 — turning the table-based
  HTML email into readable terminal text). Notifier depends on it only under `local-transport`.
  Links (`href` + visible URLs) are surfaced separately at the top of the block so a magic-link
  URL is trivially copyable. *(User decision, 2026-07-17.)*

- **DEC-3 (Thread A generality).** The renderer is **template-agnostic** — it extracts recipient,
  subject, all links, and the plain-text body from any HTML message. No magic-link-specific
  hard-coding (password-reset and future templates hit the same stub).

- **DEC-4 (Thread B config shape).** Mirror the existing `smtp` handling exactly: a small
  `OptionalConfig`-wrapped config resolving to a single directory `PathBuf`, standalone-feature
  only, loaded with the same `TmpConfig`/`from_optional_config_file` newtype pattern. No rich
  transport config object. Field name: **`localEmailDir`** under a dedicated `[localEmail]` table
  (see design). Precedence stays SMTP > File > Stub via the untouched `select_local_transport`.

- **DEC-5 (Thread B test interpretation).** The issue's "end-to-end" test cannot literally invoke
  `main.rs` (binary entrypoint). Interpreted pragmatically: a `ConfigHandler`-level test proving a
  config with the dir set parses → the field resolves to `Enabled(dir)` → feeding it to
  `select_local_transport(None, Some(dir))` yields `LocalTransportKind::File`. Documented as the
  wiring's coverage.

---

## 4. Requirements

### Thread A — Stub render (`SR`)

- **SR-R1** — When the stub transport successfully "sends", the gateway MUST print a bordered,
  human-readable block to **stdout** (not through `tracing`) containing at minimum: recipient
  (`to`), subject, and every link found in the HTML body.
- **SR-R2** — The block MUST include the HTML body rendered down to **plain text** (tags stripped /
  converted), readable in a terminal without HTML noise.
- **SR-R3** — Links (magic-link URL and any other `href`/visible URL) MUST be surfaced as
  standalone, copyable lines, not buried inside the wrapped body text.
- **SR-R4** — The renderer MUST be template-agnostic (works for magic-link, password-reset, and any
  future HTML email) — no per-template special-casing.
- **SR-R5** — The block MUST clearly state the email was **not actually delivered** (stub).
- **SR-R6** — The change MUST be confined to the `local-transport` feature; the `full`
  build is unaffected and pulls in no new dependency.

### Thread B — File transport wiring (`FT`, issue #169)

- **FT-R1** — Add an optional, **standalone-feature-only** config field resolving to a directory
  `PathBuf` for local `.eml` delivery. Absent config MUST resolve to "disabled" (no load error),
  mirroring `smtp`'s `OptionalConfig` default.
- **FT-R2** — Wire the resolved directory through `ConfigHandler::init_from_file` into the
  `select_local_transport(smtp_transport, file_dir)` call in `main.rs`, replacing the hardcoded
  `None`.
- **FT-R3** — Precedence MUST remain **SMTP > File > Stub**. `select_local_transport`'s logic and
  its existing unit tests MUST NOT change.
- **FT-R4** — MUST NOT affect the `full` build (which has no local transport; real SMTP
  is mandatory there). New field is `#[cfg(feature = "standalone")]` only.
- **FT-R5** — `settings/config.standalone.example.toml` MUST document the new field (commented, with
  precedence/behavior explained), consistent with the existing `[smtp]` section's doc style.
- **FT-R6** — A `ConfigHandler`-level test MUST prove config-with-dir → resolves →
  `select_local_transport` yields `File` (DEC-5).

---

## 5. Out of scope

- Adding plain-text alternatives to the email templates or `Message` DTO (only the HTML body
  exists; the stub renders from it).
- Any change to `select_local_transport`'s selection logic or the file/smtp transports themselves.
- Any `full`/full-mode behavior.
- Colorized ANSI output (plain bordered text only, portable across terminals/CI logs).

---

## 6. Gate checks

Default workspace gate (full) — proves Thread B didn't leak into full mode:

```bash
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace --all
```

Standalone-specific gate — proves both threads compile and pass:

```bash
cargo build --no-default-features --features standalone
cargo test  --no-default-features --features standalone
# notifier alone:
cargo test -p mycelium-notifier --no-default-features --features local-transport
```
