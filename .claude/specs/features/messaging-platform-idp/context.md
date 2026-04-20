# Messaging Platform IdP — Context

**Status:** Telegram spec complete, implementation in progress  
**Date:** 2026-04-06 | **Last updated:** 2026-04-19

---

## What we're building

Telegram (and future WhatsApp) as Identity Providers in Mycelium. Users link their platform identity to a Mycelium account. Once linked, the platform identity authenticates gateway requests.

**Key decisions:**
- Identity linked to the **account**, not the user
- `AccountMetaKey::TelegramUser` / `::WhatsAppUser` already exist — reused
- Mycelium receives webhooks **directly** from platforms (not via n8n)
- Trust comes from platform signatures, not from intermediaries
- Same Telegram `from.id` can be linked to one account **per tenant** (multi-tenant allowed)

---

## Two authentication modes (Telegram)

| | Mode A — Token | Mode B — Body passthrough |
|---|---|---|
| Flow | `initData` → `/auth/telegram/login` → connection string | n8n forwards Telegram body → Mycelium resolves `from.id` |
| Auth | UserAccountScope connection string (Bearer or connection-string header) | IP allowlist + `identity_source: Telegram` on route |
| Best for | AI agents, MCP, REST clients | n8n multi-step workflows (Leg 2) |

Mycelium accepts both Bearer token (JWT) and connection string — no new auth infrastructure.

---

## Specs and tasks

| File | Description |
|---|---|
| `telegram.md` | Full implementation spec (threat model, crypto, endpoints, data model) |
| `telegram-tasks.md` | 21 tasks in 7 groups, ordered by dependency |

---

## Status

- [x] Telegram spec — `telegram.md`
- [x] All open questions resolved (OQ-1, OQ-2, OQ-2b, OQ-3)
- [x] Task breakdown — `telegram-tasks.md`
- [ ] Implementation in progress — see `telegram-tasks.md`
- [ ] WhatsApp spec — not started
- [ ] Milestone assignment (probably M3 — Auth Evolution)
