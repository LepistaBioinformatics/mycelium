# Feature Spec: Email Template Web Parity

**Feature:** email-template-web-parity
**Milestone:** Brand consistency across gateway-rendered surfaces
**Status:** Implemented (2026-07-29) on `feat/email-template-web-parity` — pending user UAT before commit
**Created:** 2026-07-29
**Scope:** Large (33 template files across 3 locales; one design decision applied uniformly).
Presentation-only — no Rust code, no config, no behaviour change.

---

## 1. Objective

The gateway renders two families of user-facing HTML from the same `TEMPLATES` (Tera) registry:

- **Web pages** — `templates/web/*.html` (4 files): magic-link code display, magic-link error,
  instance bootstrap claim form, instance bootstrap error.
- **Transactional e-mails** — `templates/{en-us,es,pt-br}/email/*.jinja` (33 files: 1 base +
  10 messages, ×3 locales).

They currently look like two unrelated products. The e-mails use a dark zinc header band with a
Georgia serif wordmark, zinc-neutral surfaces, square corners and a violet `#7c3aed` accent. The
web pages use a violet-tinted canvas, a white rounded card with a soft shadow, an Inter type stack,
a violet `#8b5cf6` uppercase brand eyebrow and a sky `#bae6fd` code chip.

Bring the e-mail templates onto the **web templates' visual language** — same palette, same type
roles, same component geometry — while keeping every construct that e-mail clients actually render.

---

## 2. Verified current state (baseline)

Established by direct inspection on 2026-07-29.

| Concern | Reality |
|---|---|
| Template engine | **Tera**, `lazy_static TEMPLATES` in `core/src/settings.rs`. Loads `${TEMPLATES_DIR:-templates}/**/*`. `autoescape_on([".jinja", ".subject"])`. A load failure only `tracing::warn!`s and falls back to `Tera::default()` — broken templates fail at **render** time, not at boot. |
| E-mail render path | `core/src/use_cases/support/dispatch_notification.rs` — resolves `{locale}/{prefix}.jinja`, falls back to `{prefix}.jinja`, renders body + `.subject` separately. |
| Locale sets | `templates/en-us/email/`, `templates/es/email/`, `templates/pt-br/email/`. Verified **structurally identical** (same tags, same inline colors, same font stacks) — only the human copy differs. Each child does `{% extends "<locale>/email/base.jinja" %}`, so the three `base.jinja` files must stay separate. |
| Message templates (×3 locales) | `activation-code`, `create-connection-string`, `create-user-account`, `guest-to-subscription-account`, `magic-link-request`, `mfa-activation-start`, `mfa-activation-validated`, `mfa-disable`, `password-reset-confirmation`, `password-reset-initiated`. |
| Base blocks | `head`, `title`, `content`, `contenttitle`, `contenttable`. Children only override `title`, `contenttitle`, `contenttable`. |
| Base context variables | `domain_name`, `support_email` (optional), `domain_url` (optional). |
| Test coverage of rendered HTML | **None.** The `dispatch_notification` tests assert only `CreateResponseKind::Created` / `is_err()`. `local_transport_sending.rs` tests build a synthetic `Message`, never a rendered template. `cargo fmt --check && cargo build --workspace && cargo test --workspace --all` passes on structurally broken templates. |
| Gate reach | `ports/api` has `default = ["full"]`; `local-transport` is only enabled by `standalone` / `postgres-only`. So the default workspace test run does **not** even compile the notifier's local-transport tests — the gate is blinder than "no HTML assertions" alone implies. |
| Web page style tokens | Inter 400/500/600 + JetBrains Mono via Google Fonts `<link>`; canvas `#f5f3ff`; card `#fff` / radius 12px / `0 2px 12px rgba(0,0,0,.08)` / padding `48px 40px` / max-width 420px; brand eyebrow `#8b5cf6` 13px/600 uppercase tracking `.04em`; `h1` `#4c1d95` 22px; body `#555` 14px/1.6; code chip `#bae6fd` radius 6px mono 36px bold tracking `.18em` on `#1a1a1a`; primary button `#8b5cf6` radius 6px 14px/600; input `1px #ddd` radius 6px padding `10px 12px`; info panel `#f0f9ff` + `3px #7dd3fc` left border, text `#4c1d95` 12.5px; error `#ef4444` on `#fef2f2`; muted note `#999` 12px; link/secondary violet `#6d28d9`, hairline `#ddd6fe`. |
| Web-only mechanisms | Inline SVG badges/icons, `display:flex` layout, `<script>` clipboard + form handling, external stylesheet `<link>`, CSS classes. None of these survive the e-mail client matrix intact. |
| Known defect in scope | `templates/pt-br/email/base.jinja:106` renders the English string `Questions?` inside the Portuguese footer (`es/` and `en-us/` are correct). |
| Amber tier | `mfa-disable` and `password-reset-confirmation` use an amber callout (`#f59e0b` / `#fff7ed` / `#92400e`). The web templates have no amber role. |

---

## 3. Decisions (resolved gray areas)

- **DEC-1 — Source of truth is `templates/web/*.html`.** Confirmed by the user. The webapp carries
  a newer canonical "Lepista DS" (Bricolage/Hanken Grotesk, brand violet `#7b49a0`, cyan
  neobrutalist hard shadows) and `templates/web/` is stale relative to it. Aligning the e-mails to
  the **web templates** is the explicit ask; re-basing either surface onto the webapp DS is a
  separate, later decision.
- **DEC-2 — No icons in e-mail.** The web pages' inline SVG badges (check, error, eye, copy) are
  dropped rather than translated. Gmail and Outlook strip inline SVG, leaving a blank hole.
  Hierarchy is carried by type, color and colored table cells — which is already how the current
  e-mails work.
- **DEC-3 — Webfonts as progressive enhancement.** Emit the Google Fonts `<link>` **and** an
  `@import` in the `<style>` block, and always declare the full fallback stack inline on every text
  element. Apple Mail / iOS Mail get Inter and JetBrains Mono; Gmail and Outlook fall back to
  Helvetica/Arial and Courier New with no layout damage.
- **DEC-4 — Amber is a documented semantic extension.** The warning tier is kept (not remapped to
  the web's red `#ef4444`), because "MFA disabled" / "your password changed" are security-relevant
  notices, not errors. Amber is restyled to the web callout *geometry* (3px left rule, 4px radius,
  12.5px text) while retaining its amber hues, which come from the same Tailwind family the web
  tokens draw on.
- **DEC-5 — Graceful degradation over VML.** `border-radius` and `box-shadow` are declared inline
  and simply ignored by Outlook/Word-engine clients, yielding a square, shadowless card. No VML
  round-rect or shadow hacks are introduced.
- **DEC-6 — Three `base.jinja` files stay three files.** Children reference their base by
  locale-prefixed path. Unifying would mean rewriting 30 `extends` lines to gain one file.
- **DEC-7 — Verification is a throwaway render harness.** Because the gate checks are blind to
  template breakage (see §2), correctness is established by rendering all 33 templates to HTML in
  the scratchpad and opening them. The harness is **not** committed.

---

## 4. Requirements

### P1 — Visual parity of the shell ⭐ MVP

**User Story:** As a recipient of a Mycelium e-mail, I want it to look like the Mycelium pages I
land on, so that the message is recognisably from the same product and not a phishing attempt.

| ID | Requirement |
|---|---|
| ET-R1 | WHEN any e-mail template renders THEN the canvas SHALL be `#f5f3ff` and the message SHALL sit on a white card with `border-radius: 12px` and `box-shadow: 0 2px 12px rgba(0,0,0,0.08)` declared inline. |
| ET-R2 | WHEN the base template renders THEN the dark `#18181b` header band and the Georgia serif wordmark SHALL be gone, replaced by the web brand eyebrow — `{{ domain_name }}` in `#8b5cf6`, 13px, weight 600, uppercase, `letter-spacing: 0.04em` — rendered inside the card. |
| ET-R3 | WHEN the base template renders `contenttitle` THEN the heading SHALL use the Inter-first sans stack at 22px in `#4c1d95`, and the 2px violet underline rule below it SHALL be removed (the web pages have no such rule). |
| ET-R4 | WHEN any body copy renders THEN it SHALL use the Inter-first sans stack at 14px, `line-height: 1.6`, color `#555`; inline `<strong>` emphasis SHALL use `#1a1a1a` instead of `#18181b`. |
| ET-R5 | WHEN the footer renders THEN it SHALL sit inside the card below a 1px `#ddd6fe` hairline, use 12px `#999` muted note styling, and render `support_email` / `domain_url` links in `#6d28d9`. The zinc `#f4f4f5` footer band with `#e4e4e7` borders SHALL be gone. |
| ET-R6 | WHEN the base `<head>` renders THEN it SHALL include the Google Fonts `<link>` and `@import` for Inter + JetBrains Mono, AND every text-bearing element SHALL still carry a complete inline `font-family` fallback stack. |
| ET-R7 | WHEN a template is rendered by a Word-engine client (Outlook desktop) THEN the layout SHALL remain intact with square corners and no shadow — no VML is introduced (DEC-5). |

**Independent test:** render `activation-code` for all three locales, open in a browser, and compare
side by side with `templates/web/magic-link-display.html`. Canvas, card, eyebrow, heading and body
type must match.

### P1 — Visual parity of the content components ⭐ MVP

**User Story:** As a recipient, I want the code, button and callout in the e-mail to look exactly
like the ones on the page I'm being sent to, so the flow feels continuous.

| ID | Requirement |
|---|---|
| ET-R8 | WHEN a verification code renders (`activation-code`, `password-reset-initiated`) THEN the chip SHALL be `#bae6fd` with `border-radius: 6px`, JetBrains-Mono-first stack, 36px bold, `letter-spacing: 0.18em`, color `#1a1a1a`, centred — replacing the `#f4f4f5` box with the `#e4e4e7` border. |
| ET-R9 | WHEN the expiry caption below a code renders THEN it SHALL use the web muted-note token (12px, `#999`). |
| ET-R10 | WHEN the CTA button renders (`magic-link-request`) THEN it SHALL be `#8b5cf6` with `border-radius: 6px`, white 14px/600 label in the Inter-first stack, padding `12px 32px`, and SHALL NOT be uppercase — replacing the square uppercase `#7c3aed` button. |
| ET-R11 | WHEN the copy-paste URL block renders (`magic-link-request`) THEN it SHALL adopt the web input token — `1px solid #ddd`, `border-radius: 6px`, padding `10px 12px`, white background — with the URL in the JetBrains-Mono-first stack at 12px, `word-break: break-all`. The visible URL SHALL remain present as text (it is what makes the standalone stub transport readable). |
| ET-R12 | WHEN a labeled value renders (`create-connection-string`, `create-user-account`, `guest-to-subscription-account`) THEN the label SHALL use the web label token (13px, weight 500, `#333`) and the value SHALL sit in the web input token box; monospace values keep the JetBrains-Mono-first stack. |
| ET-R13 | WHEN an informational callout renders THEN it SHALL use the web info-panel token — background `#f0f9ff`, 3px `#7dd3fc` left rule, `border-radius: 4px`, text `#4c1d95` at 12.5px/1.5 — replacing the violet `#faf5ff` / `#7c3aed` callout. |
| ET-R14 | WHEN a warning callout renders (`mfa-disable`, `password-reset-confirmation`) THEN it SHALL keep the amber hues (`#fff7ed` fill, `#f59e0b` rule, `#92400e` text) but adopt the web callout geometry from ET-R13 (DEC-4). |
| ET-R15 | WHEN any template renders THEN it SHALL contain no `<svg>`, no `<script>`, no `display:flex`, and no layout that depends on CSS classes or the external stylesheet (DEC-2, DEC-3). |

**Independent test:** render `magic-link-request` (button + URL block + info callout),
`activation-code` (code chip), `create-user-account` (labeled value) and `mfa-disable` (amber
callout) and confirm each component against its web counterpart.

### P2 — Locale integrity

| ID | Requirement |
|---|---|
| ET-R16 | WHEN the three locales are compared THEN they SHALL remain structurally identical — same tags, same inline tokens — differing only in human copy. |
| ET-R17 | WHEN the `pt-br` footer renders THEN the support-e-mail lead-in SHALL be Portuguese (`Dúvidas?`), not the current English `Questions?`. |
| ET-R18 | WHEN this feature completes THEN every `.subject` file SHALL be byte-identical to its current content (subjects carry no styling). |

**Independent test:** the structural-equivalence diff from §2 (tags + inline colors + font stacks)
must report `SAME` for all 11 files across `en-us` vs `pt-br` and `en-us` vs `es`.

### P2 — Render verification

| ID | Requirement |
|---|---|
| ET-R19 | WHEN all 33 templates are rendered with a superset context THEN every render SHALL succeed (no Tera syntax, inheritance or undefined-variable failure). |
| ET-R20 | WHEN the feature is delivered THEN no render harness, preview script or fixture SHALL be committed to the repository (DEC-7). |

**Independent test:** the scratchpad harness exits 0 for 33/33 templates and `git status` shows only
`templates/{en-us,es,pt-br}/email/*.jinja` as modified.

---

## 5. Out of scope

| Item | Reason |
|---|---|
| Changing `templates/web/*.html` | They are the reference, not the target. |
| Adopting the webapp's Lepista DS | DEC-1 — separate decision, would move both surfaces. |
| Adding new e-mail templates or new context variables | Presentation-only refactor. |
| Rust changes (`settings.rs`, `dispatch_notification.rs`, notifier adapters) | No behaviour change is required. |
| `.subject` files | ET-R18 — no styling to align. |
| Plain-text (`multipart/alternative`) e-mail bodies | The `Message` DTO carries a single HTML `body`; adding a text part is a separate feature. |
| Dark-mode e-mail (`prefers-color-scheme`) | The web reference has no dark variant. |
| New locales | Out of scope. |
| Committing a preview/render harness | ET-R20 / DEC-7. |
| VML round-rect / shadow shims for Outlook | DEC-5. |

---

## 6. Requirement traceability

| ID | Story | Phase | Status |
|---|---|---|---|
| ET-R1 … ET-R7 | P1: Shell parity | T2, T3 | Implemented |
| ET-R8 … ET-R15 | P1: Component parity | T4–T7, T8 | Implemented |
| ET-R16 … ET-R18 | P2: Locale integrity | T3, T8 | Implemented |
| ET-R19 … ET-R20 | P2: Render verification | T1, T8, T9 | Implemented |

**Coverage:** 20 total, 20 mapped to tasks, 0 unmapped

### Verification results (2026-07-29)

| Check | Result |
|---|---|
| Harness render | 64/64 PASS — 30 e-mail bodies + 30 `.subject` + 4 `web/*.html` reference pages (the 3 abstract `base.jinja` render only through children) |
| Banned tokens under `templates/*/email/` | 0 hits for `<svg`, `<script`, `display:flex`, `Georgia`, `#18181b`, `#f4f4f5`, `#e4e4e7`, `#71717a`, `#a1a1aa`, `#7c3aed`, `#faf5ff`, `#3f3f46`, `BlinkMacSystemFont` |
| `text-transform: uppercase` | exactly 3 — the brand eyebrow in each `base.jinja` |
| Locale structural equivalence | `SAME` for all 11 files, `en-us`↔`pt-br` and `en-us`↔`es` (diff over tags, colors, font stacks, font weights, paddings, radii, spacer heights, letter-spacing) |
| `.subject` files | byte-identical to baseline renders (ET-R18) |
| Diff scope | 33 files, all `templates/{en-us,es,pt-br}/email/*.jinja`; no Rust, no `templates/web/`, no harness committed |
| `cargo fmt --all -- --check` | pass |
| `cargo build --workspace` | pass |
| `TEMPLATES_DIR=$PWD/templates cargo test --workspace --all` | pass — 331 + 37 + 32 + 20 + … / 0 failed |

**Incidental fixes** (pre-existing inconsistencies removed by normalising to the recipes):
`es/magic-link-request` URL box padding was `12px 16px` while the other locales used `10px 14px`;
`create-connection-string` / `create-user-account` value spans carried `font-weight: 600` in
`en-us`/`pt-br` but not in `es`.

---

## 7. Success criteria

- [ ] All 33 `.jinja` files render without error via the scratchpad harness (ET-R19).
- [ ] An e-mail and a web page opened side by side share canvas, card, brand eyebrow, heading,
      body type, code chip, button and callout treatments (ET-R1–R14).
- [ ] Zero `<svg>`, `<script>` or `display:flex` occurrences under `templates/*/email/` (ET-R15).
- [ ] Structural-equivalence diff reports `SAME` for all 11 files in both locale pairs (ET-R16).
- [ ] `git diff --stat` touches only `templates/{en-us,es,pt-br}/email/*.jinja` (ET-R20).
- [ ] `cargo fmt --all -- --check`, `cargo build --workspace`, `cargo test --workspace --all` pass
      — necessary but, per §2, **not sufficient**; the harness render is the real gate.
