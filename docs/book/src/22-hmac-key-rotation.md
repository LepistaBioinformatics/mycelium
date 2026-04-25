# HMAC Key Rotation

`UserAccountScope` connection strings are signed with an HMAC key. Starting
from the deploy that introduces this document, the gateway manages signing
keys as a **versioned set** and embeds the key version inside every token
(the `kvr` bean). Verification picks the key by version, so multiple keys
can coexist during rotation.

> **!!!!!! DEPLOYMENT WARNING !!!!!!**
>
> Deploying the HMAC key-rotation release **invalidates every connection
> string issued before the deploy**. Pre-existing tokens carry no `kvr`
> bean; verification fails with `MYC00030` (MissingKeyVersion) and users
> must re-authenticate. Plan a maintenance window.
>
> The gateway **refuses to start** when `hmacPrimaryVersion` is absent
> from `hmacSecrets` or when the set is empty. This is enforced at config
> load.

---

## Why versioned keys

Prior to this release, the HMAC signing key was tied to the global
`token_secret`. Rotating `token_secret` for any reason (KEK rotation,
compliance refresh) invalidated every connection string in circulation
because there was no way to verify tokens under the previous key while
the new one was active.

A versioned key set decouples rotation from invalidation:

- New tokens are signed with the **primary** version.
- Verification looks up the key by the `kvr` bean, so tokens signed with
  a still-listed previous version stay valid until the operator removes
  the entry.
- Tampering with the `kvr` bean does not help an attacker: the HMAC is
  computed over the `kvr` value (anti-downgrade), so changing it after
  issuance yields `MYC00032` (SignatureMismatch).

---

## Configuration reference

```toml
[core.accountLifeCycle]
# …other fields…
tokenSecret = { vault = { path = "myc/core/accountLifeCycle", key = "tokenSecret" } }

# Version of the HMAC key used to sign new connection strings.
# MUST match a `version` listed in `hmacSecrets` below.
hmacPrimaryVersion = 2

[[core.accountLifeCycle.hmacSecrets]]
version = 1
secret = { vault = { path = "myc/core/accountLifeCycle", key = "hmacSecretV1" } }

[[core.accountLifeCycle.hmacSecrets]]
version = 2
secret = { vault = { path = "myc/core/accountLifeCycle", key = "hmacSecretV2" } }
```

Validation rules (checked at startup, gateway refuses to boot on
failure):

- `hmacSecrets` must be non-empty.
- No two entries may share the same `version`.
- `hmacPrimaryVersion` must equal one of the `version` values.

Key material is resolved through `SecretResolver`, so Vault, env vars
and literal values are all supported — follow the same operational
practices you already use for `tokenSecret`.

---

## Rotation procedure

The goal is to introduce a new signing key, let the fleet pick it up,
switch signing to it, let old tokens age out, then retire the old key.

1. **Provision the new secret** (e.g. as `hmacSecretV2` in Vault).
2. **Add a new `[[hmacSecrets]]` entry** to every config replica. Keep
   `hmacPrimaryVersion` pointing at the **old** version for now; the
   gateway will start verifying new tokens carrying either version. Roll
   out the config.
3. **Bump `hmacPrimaryVersion`** to the new version once every replica
   has the new entry loaded. From this point, freshly-issued tokens
   carry `kvr=<new>`.
4. **Wait for the connection-string TTL** (`tokenExpiration`) so that
   every token issued under the old version has naturally expired.
5. **Remove the old `[[hmacSecrets]]` entry** and the corresponding
   secret. Tokens still citing that version will now fail with
   `MYC00031` (UnknownKeyVersion); this is the desired retirement
   outcome.

Reverse the procedure (un-bump primary, then remove the new entry) if
you need to roll back before step 4.

---

## Anti-downgrade

The `kvr` bean is part of the HMAC input — not an out-of-band hint. An
attacker who intercepts a token signed under `kvr=2` cannot rewrite the
bean to `kvr=1` and have verification succeed, because the recomputed
HMAC over the rewritten payload no longer matches the stored `SIG`.
Verification returns `MYC00032` in that case, not a silent fallback.

Similarly, stripping the `kvr` bean yields `MYC00030` rather than a
guess at the default version.

---

## Native error codes

| Code | Meaning | Typical cause |
|------|---------|---------------|
| `MYC00030` | Connection string is missing the HMAC key version (`kvr` bean). | Pre-rotation token, or tampering. |
| `MYC00031` | Connection string references an unknown HMAC key version. | Retired key, misconfigured replica, or tampering. |
| `MYC00032` | Connection string signature mismatch. | Tampering (including `kvr` downgrade) or wrong key provisioned for the version. |

All three are surfaced verbatim in `Unauthorized` responses from the
connection-string middleware, and structured-logged with
`connection_string_verification_failed=true` for operator audit.

---

See also:

- [Envelope Encryption Migration Guide](./21-envelope-encryption-migration.md)
  for KEK rotation (different key, different procedure).
- [Encryption Inventory](./20-encryption-inventory.md) for the full list
  of `token_secret` consumers.
