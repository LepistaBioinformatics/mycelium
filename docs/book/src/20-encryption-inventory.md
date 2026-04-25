# Encryption Inventory

This page lists every field that Mycelium stores in an encrypted or hashed form,
together with the mechanism used and its migration status relative to the
envelope encryption rollout (Phases 1 and 2).

---

## Fields encrypted with AES-256-GCM

These fields hold reversible ciphertexts. Before Phase 1 they were all
encrypted with the global KEK directly (v1 format). After Phase 1 they use
per-tenant DEKs wrapped by the KEK (v2 format). The two formats are
distinguished by a `v2:` prefix in the stored value.

| Field | Table / column | Mechanism before Phase 1 | DEK scope | Migration phase |
|---|---|---|---|---|
| `Totp::Enabled.secret` | `user.mfa` (JSONB) | `Totp::encrypt_me` — KEK direct | system (UUID nil) | Phase 1 |
| `HttpSecret.token` (webhook) | `webhook.secret` (JSONB) | `WebHook::new_encrypted` → `HttpSecret::encrypt_me` — KEK direct | system (UUID nil) | Phase 1 |
| `TelegramBotToken` | `tenant.meta` (JSONB key) | `encrypt_string` — KEK direct | per-tenant | Phase 1 |
| `TelegramWebhookSecret` | `tenant.meta` (JSONB key) | `encrypt_string` — KEK direct | per-tenant | Phase 1 |
| `phone_number`, `telegram_user` | `account.meta` (JSONB) | plaintext | per-tenant | Phase 2 |
| `tenant.meta` (general keys) | `tenant.meta` (JSONB) | plaintext | per-tenant | Phase 2 |
| Subscription / TenantManager metadata | `account.meta` (JSONB) | plaintext | per-tenant | Phase 2 |

TOTP is user identity (user, manager, staff) and is never tenant-scoped;
every call site passes `tenant_id = None`, so the secret is encrypted under
the system DEK.

### DEK storage

Each tenant row in the `tenant` table now carries two additional columns:

| Column | Type | Description |
|---|---|---|
| `encrypted_dek` | `TEXT` (nullable) | AES-256-GCM ciphertext of the 32-byte DEK, wrapped by the KEK. `NULL` means the DEK has not been provisioned yet (lazy on first use). |
| `kek_version` | `INTEGER NOT NULL DEFAULT 1` | Tracks which KEK generation was used to wrap the DEK. Used during KEK rotation. |

The system tenant row (`id = 00000000-0000-0000-0000-000000000000`) stores the
DEK used for system-level secrets (webhook HTTP secrets, all TOTP).

---

## Fields hashed with Argon2 — outside encryption scope

These fields are **one-way hashes**. There is no plaintext to recover or
re-encrypt. They are unaffected by envelope encryption migration.

| Field | Table / column | Note |
|---|---|---|
| `password_hash` | `identity_provider` | Argon2id — verification only, no decryption |
| Email confirmation token | `UserRelatedMeta.token` (logical) | Argon2 one-way hash |

---

## Ciphertext format versions

| Version | Format | When written | How detected |
|---|---|---|---|
| v1 (legacy) | `base64(nonce₁₂ ‖ ciphertext ‖ tag₁₆)` | Before Phase 1 | No prefix |
| v2 (envelope) | `v2:base64(nonce₁₂ ‖ ciphertext ‖ tag₁₆)` | After Phase 1 | Starts with `v2:` |

Decrypt functions detect the prefix automatically and route to the correct
decryption path, so v1 and v2 data can coexist in the same deployment without
downtime.

---

## AAD (Authenticated Additional Data)

AAD prevents ciphertexts from being transplanted between tenants or between
fields. The AAD scheme is:

```
aad = tenant_id.as_bytes() || field_name_bytes
```

| Field constant | Bytes |
|---|---|
| `AAD_FIELD_TOTP_SECRET` | `b"totp_secret"` |
| `AAD_FIELD_TELEGRAM_BOT_TOKEN` | `b"telegram_bot_token"` |
| `AAD_FIELD_TELEGRAM_WEBHOOK_SECRET` | `b"telegram_webhook_secret"` |
| `AAD_FIELD_HTTP_SECRET` | `b"http_secret"` |

DEK wrap/unwrap uses only `tenant_id.as_bytes()` as AAD (no field suffix).

---

## `token_secret` is multi-purpose — rotation has side-effects

The `token_secret` configured in `AccountLifeCycle` is **not only** the KEK
source. Its bytes are also consumed directly by non-envelope code paths:

| Consumer | Role | Rotation impact |
|---|---|---|
| `AccountLifeCycle::derive_kek_bytes` | KEK for wrap/unwrap of all DEKs | Re-wrap DEKs via `myc-cli rotate-kek` (TODO). |
| `encrypt_string::build_aes_key` (v1 legacy path) | KEK for ciphertexts written before Phase 1 | Stays readable only while `token_secret` is unchanged; migrate to v2 before rotating. |
| `HttpSecret::decrypt_me` (v1 branch) | Indirect — routes through the legacy path | Same as above. |
| `Totp::decrypt_me` (v1 branch) | Indirect — routes through the legacy path | Same as above. |
| `UserAccountScope::sign_token` | Independent — consumes `hmacSecrets[hmacPrimaryVersion]`, no longer routes through `token_secret` | Decoupled from KEK rotation. Rotate via the separate versioned procedure documented in [HMAC Key Rotation](./22-hmac-key-rotation.md). |

Rotate `token_secret` only after:

1. `migrate-dek --dry-run` reports zero `v1` fields remaining, **and**
2. A `rotate-kek` pass (see the Envelope Encryption Migration Guide) has
   re-wrapped every tenant's DEK under the new KEK. HMAC-protected
   connection strings are **no longer tied to `token_secret`** — rotating
   the HMAC key is a separate, versioned procedure (see
   [HMAC Key Rotation](./22-hmac-key-rotation.md)).

---

See [Envelope Encryption Migration Guide](./21-envelope-encryption-migration.md)
for step-by-step operator instructions on KEK rotation.
See [HMAC Key Rotation](./22-hmac-key-rotation.md) for the connection-string
signing-key rotation procedure.
