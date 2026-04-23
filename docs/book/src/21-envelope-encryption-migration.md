# Envelope Encryption Migration Guide

This guide is for operators running Mycelium with the global `token_secret` key
who need to migrate to the envelope encryption scheme (KEK/DEK per tenant).

---

## Overview

| Before | After |
|---|---|
| All secrets encrypted with a single global key (direct KEK) | Each tenant has a random DEK, encrypted by the KEK and stored in the database |
| Rotating the KEK invalidates all encrypted data | Rotating the KEK re-encrypts only the DEKs — O(number of tenants), not O(number of records) |
| No ciphertext versioning | The `v2:` prefix identifies data in the new scheme; the legacy format (v1) continues to be read |

The new version is **fully backward-compatible**: data encrypted in the old
format continues to be decrypted correctly. Migration is optional at first and
can be done incrementally.

---

## Prerequisites

- Mycelium API Gateway updated to the version with envelope encryption support
- Access to the PostgreSQL database
- Access to the configured `token_secret` (via env, Vault, or config file) — **do not change this value before completing the migration**
- `myc-cli` available in PATH

---

## Migration steps

### 1. Verify compatibility (no downtime required)

The new version supports reading both `v1` (old format) and `v2` (new format)
data simultaneously. It is safe to deploy the new version while the database is
still in `v1` format.

```bash
# Confirm the installed version supports envelope encryption
myc-cli --version
```

### 2. Run the SQL migration

```bash
psql $DATABASE_URL < path/to/20260421_01_envelope_encryption.sql
```

This adds the `encrypted_dek` and `kek_version` columns to the `tenant` table.
Both are nullable — existing tenants will have `NULL` until the next step.

### 3. Simulate the migration (dry-run)

```bash
SETTINGS_PATH=settings/config.toml myc-cli migrate-dek --dry-run
```

Expected output: a list of tenants with the count of `v1` fields to migrate.
No writes are performed.

### 4. Run the migration

```bash
SETTINGS_PATH=settings/config.toml myc-cli migrate-dek
```

The command is **idempotent** and **resumable**:

- Fields already in `v2` format are skipped.
- It can be interrupted at any point and re-run without duplicating work.
- To migrate only a specific tenant: `--tenant-id <uuid>`

### 5. Validate completion

```bash
SETTINGS_PATH=settings/config.toml myc-cli migrate-dek --dry-run
```

Should report **0 v1 fields remaining** across all tenants.

---

## KEK rotation (optional, post-migration)

After completing the migration to `v2`, the KEK can be rotated without touching
encrypted data records:

```bash
# Increment kek_version in config and make the new key available
# Then run:
SETTINGS_PATH=settings/config.toml myc-cli rotate-kek \
  --from-version 1 \
  --to-version 2
```

This re-encrypts only the `encrypted_dek` of each tenant with the new KEK. The
data records (`user.mfa`, `tenant.meta`, `webhook.secret`) **are not
modified**.

After a successful rotation, the v1 KEK can be discarded.

> **Side-effect — connection strings are invalidated.** `token_secret` is also
> used as the HMAC key for connection-string signatures
> (`UserAccountScope::sign_token`). Rotating the KEK therefore invalidates
> every signature issued under the old secret. There is no re-signing path —
> treat all active connection strings as revoked and plan the rotation
> accordingly. See the [Encryption
> Inventory](./20-encryption-inventory.md#token_secret-is-multi-purpose--rotation-has-side-effects)
> for the full list of `token_secret` consumers.

---

## Rollback

If you need to roll back before the migration is complete:

1. Roll back the deployment to the previous gateway version.
2. Any `v2` data already written is **unreadable** by the old version (which
   does not know the `v2` format).

> **Therefore:** do not interrupt a migration mid-way in production. Use
> `--dry-run` to validate first, and run in a maintenance window if in doubt.

If the migration is complete and you need to roll back the SQL schema:

```sql
ALTER TABLE tenant DROP COLUMN IF EXISTS encrypted_dek;
ALTER TABLE tenant DROP COLUMN IF EXISTS kek_version;
```

This is only safe if no `v2` data was written. If `v2` writes have already
occurred, rolling back the schema will cause loss of access to those records.

---

## Frequently asked questions

**Do I need downtime to migrate?**
No. The new version reads both `v1` and `v2`. Deploy first, then run
`migrate-dek` with the service running.

**Can I keep `v1` data indefinitely?**
Yes, as long as `token_secret` does not change. If the global key is rotated,
`v1` data becomes unreadable. Migrating to `v2` protects against this.

**What about Argon2 hashes (passwords)?**
Argon2 hashes in `identity_provider.password_hash` are one-way — there is no
plaintext to re-encrypt. They are unaffected by this migration and continue to
work normally.

**What happens to new tenants created after the deploy?**
Tenants created after the deploy receive a DEK automatically on first use. No
manual action is required.

---

## KEK Rotation (`myc-cli kek rotate-kek`)

`rotate-kek` rewraps every tenant's Data Encryption Key (DEK) under a new Key
Encryption Key (KEK) without touching any user-data ciphertext. Because the
DEKs themselves are unchanged, existing `v2:`-prefixed ciphertexts stay
readable after rotation.

### Precondition

Run `migrate-dek --dry-run` and confirm zero remaining `v1` rows. Any `v1`
ciphertext encountered after a KEK rotation is unrecoverable: it is pinned to
the old `token_secret` and not protected by a DEK at all.

### Step-by-step

1. **Keep the old `token_secret` reachable.** Export it under the
   `MYC_OLD_TOKEN_SECRET` environment variable before invoking the CLI. It is
   used to unwrap each tenant's stored DEK.
2. **Set the new `token_secret`** in `settings/config.toml` (or the
   backing Vault secret). This is the KEK that every rewrapped DEK will be
   bound to.
3. **Dry-run first:**
   ```bash
   MYC_OLD_TOKEN_SECRET=<old-uuid> \
     myc-cli kek rotate-kek --from-version 1 --to-version 2 --dry-run
   ```
   Inspect the counters. `Migrated` should equal the number of tenants on
   `kek_version == 1` with an `encrypted_dek` set; `Skipped` should be zero
   unless some rows are intentionally on a different generation.
4. **Run live:**
   ```bash
   MYC_OLD_TOKEN_SECRET=<old-uuid> \
     myc-cli kek rotate-kek --from-version 1 --to-version 2
   ```
5. **Verify** that the gateway starts cleanly under the new `token_secret`
   and that authenticated traffic still reaches tenant-scoped features
   (Telegram bot tokens, TOTP verification, webhook secrets).
6. **Discard** the old `token_secret` from Vault / env only after the live
   run is confirmed.

### Connection strings stay valid

`rotate-kek` does **not** touch the HMAC key used to sign user-facing
connection strings — see the dedicated HMAC Key Rotation guide (Etapa 3)
for that procedure. Issued connection strings remain usable across a KEK
rotation as long as `hmac_secret` (or the Etapa-1 fallback to the previous
`token_secret` value) is kept available.

### Idempotence and rollback

Re-running `rotate-kek` with the same `--from-version` / `--to-version`
reports the already-rotated rows as `Already done` and is safe to run
repeatedly.

The rewrap itself is **irreversible per row** — once a tenant's
`encrypted_dek` is persisted under the new KEK, unwrapping requires the new
`token_secret`. Keep both old and new secrets resolvable (Vault versioning,
dual env vars, or a short overlap in config) until you have confirmed the
rotation end-to-end.

---

See [Encryption Inventory](./20-encryption-inventory.md) for the complete field
classification table.
