# Design: Email Template Web Parity

The structural work is identical for all 33 files: **one token map + eight component recipes,
applied three times.** This document is the single source of truth for the implementation — the
tasks reference recipe names from §3, not ad-hoc styling.

---

## 1. Token map (`templates/web/*.html` → e-mail)

| Role | Web source | E-mail value | Note |
|---|---|---|---|
| Canvas | `body { background: #f5f3ff }` | `#f5f3ff` on `<body>`, outer `<table bgcolor>` and `<style>` | |
| Card surface | `.card { background:#fff; border-radius:12px; box-shadow:0 2px 12px rgba(0,0,0,.08) }` | same three declarations inline on the 600px card `<table>` | radius/shadow ignored by Outlook (DEC-5) |
| Card padding | `48px 40px` | `40px` horizontal, `40px` top / `32px` bottom via padding + spacer rows | |
| Card width | `max-width: 420px` | `600px` | ADAPT-1 |
| Sans stack | `'Inter','Open Sans',Arial,sans-serif` | `'Inter','Open Sans',Helvetica,Arial,sans-serif` | `Helvetica` inserted for Apple clients |
| Mono stack | `'JetBrains Mono','Courier New',monospace` | `'JetBrains Mono','Courier New',Courier,monospace` | |
| Brand eyebrow | `.brand` — `#8b5cf6` 13px/600 uppercase `letter-spacing:.04em` | identical | replaces the dark header band |
| Heading | `h1` — `#4c1d95` 22px | `#4c1d95` 22px/1.3 weight 600, sans stack | Georgia and the 2px violet rule are dropped |
| Body copy | `p` — `#555` 14px/1.6 | identical | |
| Strong emphasis | `.code { color:#1a1a1a }` | `#1a1a1a` | was `#18181b` |
| Code chip | `.code` — `#bae6fd` radius 6px, mono 36px bold `letter-spacing:.18em`, `#1a1a1a`, padding `16px 40px` | identical, centred | was `#f4f4f5` + `#e4e4e7` border |
| Primary button | `button` — `#8b5cf6` radius 6px, `#fff` 14px/600, padding `12px` | `#8b5cf6` radius 6px, `#fff` 14px/600, padding `12px 32px`, **not** uppercase | was square uppercase `#7c3aed` |
| Field box | `input` — `1px solid #ddd` radius 6px padding `10px 12px` | identical, white fill | serves the URL block and labeled values |
| Field label | `label` — 13px/500 `#333` | identical | was 11px uppercase `#71717a` |
| Info callout | `.stakes` — `#f0f9ff`, `3px solid #7dd3fc` left, radius 4px, `#4c1d95` 12.5px/1.5, padding `10px 12px` | identical | was `#faf5ff` + `#7c3aed` |
| Warning callout | *(no web equivalent)* | `#fff7ed`, `3px solid #f59e0b` left, radius 4px, `#92400e` 12.5px/1.5 | DEC-4 — geometry from `.stakes`, amber hues kept |
| Muted note | `.note` — `#999` 12px | identical | expiry captions + footer |
| Link | `.copy-button { color:#6d28d9 }` | `#6d28d9`, `text-decoration:none` | was `#7c3aed` |
| Hairline | `.copy-button { border:1px solid #ddd6fe }` | `#ddd6fe` 1px | was `#e4e4e7` |

### Tokens deliberately not carried over

`#18181b` (dark header), `#f4f4f5` / `#e4e4e7` / `#71717a` / `#a1a1aa` (zinc scale), `#7c3aed`
(old violet), `#faf5ff`, Georgia serif, `text-transform: uppercase` on buttons. After the refactor
none of these hexes should appear under `templates/*/email/`.

### Documented e-mail adaptations

| ID | Adaptation | Why |
|---|---|---|
| ADAPT-1 | Card is 600px, not 420px | 600px is the e-mail client safe width; 420px would leave the copy cramped |
| ADAPT-2 | Eyebrow, heading and body copy are **left**-aligned (web centres everything) | The web card holds one short line; e-mails hold paragraphs. Only the code chip and the CTA are centred, matching the web's own centred treatment of those two elements |
| ADAPT-3 | Footer moves *inside* the card, below a `#ddd6fe` hairline, using the muted-note token | The web pages have no footer; the zinc band outside the card has no web analogue |
| ADAPT-4 | Spacer `<tr>` rows replace `margin-bottom` | `margin` is unreliable on `td` in Outlook |
| ADAPT-5 | No icons, no script, no flex, no CSS classes for layout | DEC-2 / DEC-3 / ET-R15 |
| ADAPT-6 | Webfonts via `<link>` **and** `@import`, with full inline fallbacks | DEC-3 — `<link>` survives in Apple Mail, `@import` in a few others, inline stacks cover the rest |

---

## 2. Canonical base (`templates/en-us/email/base.jinja`)

`es` and `pt-br` are byte-identical apart from `lang`, the `extends` path in children, and the two
footer strings. Blocks and context variables (`domain_name`, `support_email`, `domain_url`) are
unchanged.

```html
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" lang="en">

<head>
  {% block head %}
  <meta http-equiv="Content-Type" content="text/html; charset=utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <meta name="x-apple-disable-message-reformatting" />
  <title>{% block title %}{% endblock title %} — {{ domain_name }}</title>
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="crossorigin" />
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&amp;family=JetBrains+Mono:wght@400;700&amp;display=swap" rel="stylesheet" />
  <style type="text/css">
    @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;700&display=swap');
    body, table, td, p, a, span { -webkit-text-size-adjust: 100%; -ms-text-size-adjust: 100%; }
    table, td { mso-table-lspace: 0pt; mso-table-rspace: 0pt; border-collapse: collapse; }
    img { -ms-interpolation-mode: bicubic; border: 0; outline: none; text-decoration: none; }
    body { margin: 0 !important; padding: 0 !important; background-color: #f5f3ff; }
  </style>
  {% endblock head %}
</head>

<body style="margin: 0; padding: 0; background-color: #f5f3ff;">

<!-- Outer wrapper -->
<table width="100%" cellpadding="0" cellspacing="0" border="0" bgcolor="#f5f3ff"
  style="background-color: #f5f3ff;">
  <tr>
    <td align="center" style="padding: 40px 16px;">

      <!-- Card: 600px max, white, rounded, soft shadow -->
      <table width="600" cellpadding="0" cellspacing="0" border="0" bgcolor="#ffffff"
        style="max-width: 600px; width: 100%; background-color: #ffffff; border-radius: 12px; box-shadow: 0 2px 12px rgba(0,0,0,0.08);">

        <tr>
          <td style="padding: 40px 40px 0;">

            <!-- Brand eyebrow -->
            <table width="100%" cellpadding="0" cellspacing="0" border="0">
              <tr>
                <td style="font-family: 'Inter', 'Open Sans', Helvetica, Arial, sans-serif; font-size: 13px; font-weight: 600; color: #8b5cf6; letter-spacing: 0.04em; text-transform: uppercase;">
                  {{ domain_name }}
                </td>
              </tr>
              <tr>
                <td height="20" style="font-size: 0; line-height: 0;">&nbsp;</td>
              </tr>
            </table>

            {% block content %}
            <!-- Title -->
            <table width="100%" cellpadding="0" cellspacing="0" border="0">
              <tr>
                <td>
                  <h1 style="margin: 0; font-family: 'Inter', 'Open Sans', Helvetica, Arial, sans-serif; font-size: 22px; font-weight: 600; color: #4c1d95; line-height: 1.3;">
                    {% block contenttitle %}{% endblock contenttitle %}
                  </h1>
                </td>
              </tr>
              <tr>
                <td height="20" style="font-size: 0; line-height: 0;">&nbsp;</td>
              </tr>
            </table>

            <!-- Body content -->
            <table width="100%" cellpadding="0" cellspacing="0" border="0">
              {% block contenttable %}{% endblock contenttable %}
            </table>
            {% endblock content %}

          </td>
        </tr>

        <!-- Footer (inside the card, below a hairline) -->
        <tr>
          <td style="padding: 28px 40px 32px;">
            <table width="100%" cellpadding="0" cellspacing="0" border="0">
              <tr>
                <td height="1" bgcolor="#ddd6fe"
                  style="background-color: #ddd6fe; height: 1px; font-size: 0; line-height: 0;">&#8203;</td>
              </tr>
              <tr>
                <td height="16" style="font-size: 0; line-height: 0;">&nbsp;</td>
              </tr>

              {% if support_email %}
              <tr>
                <td style="font-family: 'Inter', 'Open Sans', Helvetica, Arial, sans-serif; font-size: 12px; color: #999999; line-height: 1.6;">
                  Questions?&#32;<a href="mailto:{{ support_email }}"
                    style="color: #6d28d9; text-decoration: none;">{{ support_email }}</a>
                </td>
              </tr>
              {% endif %}

              {% if domain_url %}
              <tr>
                <td style="font-family: 'Inter', 'Open Sans', Helvetica, Arial, sans-serif; font-size: 12px; color: #999999; line-height: 1.6;">
                  <a href="{{ domain_url }}" style="color: #6d28d9; text-decoration: none;">{{ domain_url }}</a>
                </td>
              </tr>
              {% endif %}

              <tr>
                <td height="8" style="font-size: 0; line-height: 0;">&nbsp;</td>
              </tr>
              <tr>
                <td style="font-family: 'Inter', 'Open Sans', Helvetica, Arial, sans-serif; font-size: 12px; color: #999999; line-height: 1.5;">
                  Powered by&#32;<a href="https://lepista.com.br"
                    style="color: #999999; text-decoration: none;">LepistaBioinformatics</a>
                </td>
              </tr>

            </table>
          </td>
        </tr>

      </table>
    </td>
  </tr>
</table>

</body>
</html>
```

### Per-locale footer strings

| Locale | `support_email` lead-in | Attribution |
|---|---|---|
| `en-us` | `Questions?` | `Powered by` |
| `es` | `&#191;Preguntas?` | `Desarrollado por` |
| `pt-br` | `D&#250;vidas?` | `Desenvolvido por` — **fixes ET-R17**, currently `Questions?` |

> No child `td` inside the card may declare its own `bgcolor` / `background-color` at the card's
> outer edge — a square child background would paint over the parent's rounded corners in
> webkit-based clients. The card table alone owns the white fill.

---

## 3. Component recipes

Each recipe is a `<tr>` fragment for `{% block contenttable %}`. The trailing spacer belongs to the
component, so blocks compose by concatenation.

### R-A — Body paragraph

```html
<tr>
  <td style="font-family: 'Inter', 'Open Sans', Helvetica, Arial, sans-serif; font-size: 14px; color: #555555; line-height: 1.6; padding-bottom: 24px;">
    … copy, with <strong style="color: #1a1a1a;">emphasis</strong> …
  </td>
</tr>
```

### R-B — Code chip + expiry caption (`activation-code`, `password-reset-initiated`)

```html
<tr>
  <td style="padding-bottom: 24px;">
    <table width="100%" cellpadding="0" cellspacing="0" border="0">
      <tr>
        <td align="center" bgcolor="#bae6fd"
          style="background-color: #bae6fd; border-radius: 6px; padding: 16px 40px; text-align: center;">
          <span style="font-family: 'JetBrains Mono', 'Courier New', Courier, monospace; font-size: 36px; font-weight: bold; color: #1a1a1a; letter-spacing: 0.18em;">{{ verification_code }}</span>
        </td>
      </tr>
      <tr>
        <td height="12" style="font-size: 0; line-height: 0;">&nbsp;</td>
      </tr>
      <tr>
        <td align="center"
          style="font-family: 'Inter', 'Open Sans', Helvetica, Arial, sans-serif; font-size: 12px; color: #999999; text-align: center;">
          … expiry caption …
        </td>
      </tr>
    </table>
  </td>
</tr>
```

### R-C — Primary CTA button (`magic-link-request`)

```html
<tr>
  <td align="center" style="padding-bottom: 24px; text-align: center;">
    <table cellpadding="0" cellspacing="0" border="0" align="center">
      <tr>
        <td bgcolor="#8b5cf6" style="background-color: #8b5cf6; border-radius: 6px;">
          <a href="{{ magic_link_url }}"
            style="font-family: 'Inter', 'Open Sans', Helvetica, Arial, sans-serif; font-size: 14px; font-weight: 600; color: #ffffff; text-decoration: none; display: inline-block; padding: 12px 32px;">
            … label …
          </a>
        </td>
      </tr>
    </table>
  </td>
</tr>
```

### R-D — Field box with label (`create-connection-string`, `create-user-account`, `guest-to-subscription-account`)

```html
<tr>
  <td style="padding-bottom: 24px;">
    <table width="100%" cellpadding="0" cellspacing="0" border="0">
      <tr>
        <td style="font-family: 'Inter', 'Open Sans', Helvetica, Arial, sans-serif; font-size: 13px; font-weight: 500; color: #333333; padding-bottom: 6px;">
          … label …
        </td>
      </tr>
      <tr>
        <td bgcolor="#ffffff"
          style="background-color: #ffffff; border: 1px solid #dddddd; border-radius: 6px; padding: 10px 12px;">
          <span style="font-family: 'Inter', 'Open Sans', Helvetica, Arial, sans-serif; font-size: 14px; color: #555555;">{{ value }}</span>
        </td>
      </tr>
    </table>
  </td>
</tr>
```

Monospace variant (`create-user-account` → `account_name`): swap the value `<span>` stack for
`'JetBrains Mono','Courier New',Courier,monospace`.

### R-E — Copy-paste URL block (`magic-link-request`)

Same geometry as R-D; the label uses the muted-note token because it is helper text, not a field
name. The URL stays visible as text (ET-R11).

```html
<tr>
  <td style="padding-bottom: 24px;">
    <table width="100%" cellpadding="0" cellspacing="0" border="0">
      <tr>
        <td style="font-family: 'Inter', 'Open Sans', Helvetica, Arial, sans-serif; font-size: 12px; color: #999999; padding-bottom: 6px;">
          … "button didn't work?" helper …
        </td>
      </tr>
      <tr>
        <td bgcolor="#ffffff"
          style="background-color: #ffffff; border: 1px solid #dddddd; border-radius: 6px; padding: 10px 12px;">
          <span style="font-family: 'JetBrains Mono', 'Courier New', Courier, monospace; font-size: 12px; color: #555555; word-break: break-all;">{{ magic_link_url }}</span>
        </td>
      </tr>
    </table>
  </td>
</tr>
```

### R-F — Info callout (all templates except the two amber ones)

```html
<tr>
  <td>
    <table width="100%" cellpadding="0" cellspacing="0" border="0">
      <tr>
        <td width="3" bgcolor="#7dd3fc"
          style="background-color: #7dd3fc; width: 3px; font-size: 0; line-height: 0;">&#8203;</td>
        <td bgcolor="#f0f9ff"
          style="background-color: #f0f9ff; padding: 10px 12px;">
          <span style="font-family: 'Inter', 'Open Sans', Helvetica, Arial, sans-serif; font-size: 12.5px; color: #4c1d95; line-height: 1.5;">
            … notice …
          </span>
        </td>
      </tr>
    </table>
  </td>
</tr>
```

> The 4px radius from `.stakes` is intentionally not applied here: a two-cell rule+fill construct
> cannot round only its outer corners without a wrapper that Outlook mishandles. The 3px left rule
> is the load-bearing part of the token. This is the one place the web geometry is approximated —
> recorded so it is not mistaken for an oversight.

### R-G — Warning callout (`mfa-disable`, `password-reset-confirmation`)

R-F with `#f59e0b` rule, `#fff7ed` fill, `#92400e` text (DEC-4).

### R-H — Spacer row

```html
<tr>
  <td height="N" style="font-size: 0; line-height: 0;">&nbsp;</td>
</tr>
```

---

## 4. Component usage per template

| Template | Recipes |
|---|---|
| `activation-code` | R-A, R-B, R-F |
| `password-reset-initiated` | R-A, R-B, R-F |
| `magic-link-request` | R-A, R-C, R-E, R-F |
| `create-connection-string` | R-A, R-D, R-F |
| `create-user-account` | R-A, R-D (mono), R-F |
| `guest-to-subscription-account` | R-A, R-D, R-F |
| `mfa-activation-start` | R-A, R-F |
| `mfa-activation-validated` | R-A, R-F |
| `mfa-disable` | R-A, R-G |
| `password-reset-confirmation` | R-A, R-G |

Human copy, `{% block %}` names and context variables are carried over verbatim from the current
files — only markup and inline styles change (except ET-R17's Portuguese footer fix).

---

## 5. Verification harness (scratchpad only — never committed)

Gate checks are blind to template breakage (spec §2), so verification is a rendered-output check.

```
$SCRATCHPAD/render_email_previews/
  Cargo.toml        — bin depending on tera only
  src/main.rs       — Tera::new("<repo>/templates/**/*"), superset context,
                      render every name matching `*/email/*.jinja`
                      → $SCRATCHPAD/previews/<locale>/<template>.html
```

Superset context (union of every variable used across the 10 templates plus the base):

| Variable | Sample |
|---|---|
| `domain_name` | `Mycelium` |
| `domain_url` | `https://mycelium.local` |
| `support_email` | `support@mycelium.local` |
| `verification_code` | `481902` |
| `magic_link_url` | `https://mycelium.local/_adm/beginners/users/magic-link/display?token=eyJhbGciOiJI…` (long, to exercise `word-break`) |
| `account_name` | `ada.lovelace` |
| `role_name` | `Subscriptions Manager` |
| `role_permissions` | `read, write` |
| `expires_in` | `30 days` |

Also render the `.subject` files, to prove ET-R18 left them intact and they still render.

Static assertions run with `grep` over `templates/*/email/`:

- Zero hits: `<svg`, `<script`, `display:\s*flex`, `Georgia`, `#18181b`, `#f4f4f5`, `#e4e4e7`,
  `#71717a`, `#a1a1aa`, `#7c3aed`, `#faf5ff`, `text-transform: uppercase` (outside the eyebrow).
- Locale structural equivalence: the tag + inline-token diff from spec §2 reports `SAME` for all
  11 files in both `en-us`↔`pt-br` and `en-us`↔`es`.

Manual UAT: open `previews/en-us/*.html` next to `templates/web/magic-link-display.html`.
