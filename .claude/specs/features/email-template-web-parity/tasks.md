# Tasks: Email Template Web Parity

Presentation-only. Every task consumes the token map and recipes in `design.md` §1–§3 — no task
invents styling. `[P]` = parallelizable with other `[P]` tasks in the same group.

**Working rule for every task:** touch only `templates/{en-us,es,pt-br}/email/*.jinja`. Never
`.subject` files (ET-R18), never `templates/web/`, never Rust.

---

## T0 — Branch

- **Where:** `modules/mycelium-api-gateway/` (submodule, currently on `develop`).
- **Done when:** work happens on `feat/email-template-web-parity` cut from `develop`. Never commit
  to `develop` directly; integration is via PR.

---

## T1 — Render harness (scratchpad, throwaway)

- **Where:** `$SCRATCHPAD/render_email_previews/` — **outside the repo**.
- **Depends on:** T0.
- **Reuses:** the `tera` version already pinned in the workspace; the superset context table in
  `design.md` §5.
- **Done when:** `cargo run` renders all 33 `*/email/*.jinja` **plus** the 30 `.subject` files with
  the superset context, writes `previews/<locale>/<template>.html`, prints a per-template
  PASS/FAIL line, and exits non-zero if any render fails. Baseline run against the **current**
  templates is 60/60 PASS (30 bodies + 30 subjects; the 3 `base.jinja` are abstract and render only
  through their children) — this proves the harness itself works before anything is edited. The
  final run also renders the 4 `web/*.html` reference pages for side-by-side comparison, for 64/64.
- **Satisfies:** ET-R19, ET-R20 (nothing under the repo).

---

## T2 — Base template, canonical locale

- **Where:** `templates/en-us/email/base.jinja`.
- **Depends on:** T1.
- **Reuses:** `design.md` §2 verbatim.
- **Done when:** the file matches `design.md` §2 with `lang="en"` and the `en-us` footer strings;
  blocks (`head`, `title`, `content`, `contenttitle`, `contenttable`) and context variables
  (`domain_name`, `support_email`, `domain_url`) are unchanged; the dark header band, Georgia stack,
  violet title rule and zinc footer band are gone. Harness re-render: all 11 `en-us` templates
  still PASS (children are untouched and still inherit).
- **Satisfies:** ET-R1, ET-R2, ET-R3, ET-R5, ET-R6, ET-R7.

---

## T3 — Base template, `es` and `pt-br`

- **Where:** `templates/es/email/base.jinja`, `templates/pt-br/email/base.jinja`.
- **Depends on:** T2.
- **Reuses:** T2's output; the per-locale footer string table in `design.md` §2.
- **Done when:** both files are structurally identical to T2 — same tags, same inline tokens, same
  block names, same spacer heights — with exactly three known text deltas: `lang`
  (`es` / `pt-BR`), the support lead-in, and the attribution string. **`pt-br` renders
  `D&#250;vidas?` instead of the current English `Questions?`**. Harness re-render: 33/33 PASS.
- **Satisfies:** ET-R16, ET-R17.

---

## Content group — all three locales per task

Each task rewrites `{% block contenttable %}` for its templates in **all three locales at once**,
so the three files stay structurally identical by construction (ET-R16). Human copy, block names
and context variables are carried over verbatim from the existing files.

### T4 [P] — Code-chip templates

- **Where:** `templates/{en-us,es,pt-br}/email/activation-code.jinja`,
  `templates/{en-us,es,pt-br}/email/password-reset-initiated.jinja` (6 files).
- **Depends on:** T3.
- **Reuses:** recipes R-A, R-B, R-F.
- **Done when:** the code sits in the `#bae6fd` radius-6px chip with the JetBrains-Mono-first stack
  at 36px bold / `.18em` on `#1a1a1a`; the expiry caption uses the muted-note token; the closing
  notice is the sky info callout. No `#f4f4f5` box, no `#e4e4e7` border, no `#faf5ff` callout
  remains in these files. Harness re-render PASSes for all 6.
- **Satisfies:** ET-R4, ET-R8, ET-R9, ET-R13.

### T5 [P] — Magic-link request

- **Where:** `templates/{en-us,es,pt-br}/email/magic-link-request.jinja` (3 files).
- **Depends on:** T3.
- **Reuses:** recipes R-A, R-C, R-E, R-F.
- **Done when:** the CTA is the `#8b5cf6` radius-6px button with a non-uppercase 14px/600 white
  label; the copy-paste block uses the field-box token with the URL **visible as text** and
  `word-break: break-all`; the closing notice is the sky info callout. Harness preview with the long
  sample URL shows no horizontal overflow of the 600px card, and — since Tera autoescapes `.jinja` —
  the *visible* URL text must still be a URL a user can select and paste (a pre-existing condition
  of the current templates; the preview must surface it, not hide it).
- **Satisfies:** ET-R4, ET-R10, ET-R11, ET-R13.

### T6 [P] — Field-box templates

- **Where:** `templates/{en-us,es,pt-br}/email/create-connection-string.jinja`,
  `create-user-account.jinja`, `guest-to-subscription-account.jinja` (9 files).
- **Depends on:** T3.
- **Reuses:** recipes R-A, R-D (mono variant for `account_name`), R-F.
- **Done when:** each labeled value uses the 13px/500 `#333` label plus the white
  `1px #dddddd` radius-6px field box; `account_name` keeps a monospace value stack; the closing
  notice is the sky info callout. The 11px uppercase `#71717a` label treatment is gone.
- **Satisfies:** ET-R4, ET-R12, ET-R13.

### T7 [P] — Notice-only templates

- **Where:** `templates/{en-us,es,pt-br}/email/mfa-activation-start.jinja`,
  `mfa-activation-validated.jinja`, `mfa-disable.jinja`,
  `password-reset-confirmation.jinja` (12 files).
- **Depends on:** T3.
- **Reuses:** recipes R-A, R-F (first two templates), R-G (`mfa-disable`,
  `password-reset-confirmation`).
- **Done when:** body copy uses the web body token; `mfa-activation-start` and
  `mfa-activation-validated` close with the sky info callout; `mfa-disable` and
  `password-reset-confirmation` keep the amber tier with the R-G geometry.
- **Satisfies:** ET-R4, ET-R13, ET-R14.

---

## T8 — Static assertion sweep

- **Where:** read-only over `templates/*/email/`.
- **Depends on:** T4, T5, T6, T7.
- **Reuses:** the grep list and the locale-equivalence diff in `design.md` §5.
- **Done when:**
  - zero hits for `<svg`, `<script`, `display:\s*flex`, `Georgia`, `#18181b`, `#f4f4f5`,
    `#e4e4e7`, `#71717a`, `#a1a1aa`, `#7c3aed`, `#faf5ff`;
  - the only `text-transform: uppercase` occurrences are the three brand eyebrows — this holds only
    if T6 dropped the 11px uppercase labels in the three field-box templates and T5 dropped it from
    the CTA, so verify by grep after those tasks rather than assuming;
  - the tag + inline-token diff reports `SAME` for all 11 files in `en-us`↔`pt-br` **and**
    `en-us`↔`es`;
  - `git diff --name-only` lists exactly 33 paths, all under `templates/{en-us,es,pt-br}/email/`
    and all `.jinja`.
- **Satisfies:** ET-R15, ET-R16, ET-R18, ET-R20.

---

## T9 — Gate checks + UAT package

- **Where:** `modules/mycelium-api-gateway/`.
- **Depends on:** T8.
- **Done when:**
  - harness reports 64/64 PASS (30 bodies + 30 subjects + 4 web reference pages);
  - `cargo fmt --all -- --check`, `cargo build --workspace`, `cargo test --workspace --all` pass
    — recorded as necessary-but-insufficient per spec §2;
  - the preview directory path is handed to the user with a short side-by-side checklist (canvas,
    card, eyebrow, heading, body, code chip, button, field box, info callout, amber callout,
    footer) against `templates/web/magic-link-display.html` and
    `templates/web/instance-bootstrap-claim.html`.
- **Satisfies:** ET-R19, spec §7.

---

## T10 — Commit (blocked on user approval)

- **Depends on:** T9 **and** explicit user approval after UAT (repo rule: no commit before manual
  testing and sign-off).
- **Done when:** one commit on `feat/email-template-web-parity` inside the submodule, then a PR into
  `develop` (never a direct push to a protected branch). The monorepo pointer commit follows only
  after the submodule commit is pushed.

---

## Traceability

| Requirement | Task |
|---|---|
| ET-R1, ET-R2, ET-R3, ET-R5, ET-R6, ET-R7 | T2 |
| ET-R4 | T4, T5, T6, T7 |
| ET-R8, ET-R9 | T4 |
| ET-R10, ET-R11 | T5 |
| ET-R12 | T6 |
| ET-R13 | T4, T5, T6, T7 |
| ET-R14 | T7 |
| ET-R15 | T8 |
| ET-R16 | T3, T8 |
| ET-R17 | T3 |
| ET-R18 | T8 |
| ET-R19 | T1, T9 |
| ET-R20 | T1, T8 |

**Coverage:** 20 requirements, 20 mapped, 0 unmapped.
