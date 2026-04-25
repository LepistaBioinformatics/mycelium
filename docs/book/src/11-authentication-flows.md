# Authentication Flows

This page covers the three ways users authenticate with Mycelium and how to create
service-to-service tokens.

---

## Magic link (email login)

Magic link is the default authentication method. No passwords required — the user enters
their email and receives a one-time link.

### How it works

```
User enters email
    ↓
POST /_adm/beginners/users/magic-link/request
    ↓
Mycelium sends a login email with a one-time link
    ↓
User clicks the link → GET /_adm/beginners/users/magic-link/display/{token}
    ↓
POST /_adm/beginners/users/magic-link/verify  (with the token)
    ↓
Response: { "token": "jwt...", "type": "Bearer" }
```

### Enabling it

Internal authentication must be enabled in `config.toml`:

```toml
[auth]
internal = "enabled"
jwtSecret = "your-secret"
jwtExpiresIn = 86400
```

SMTP must also be configured so Mycelium can send the email. See [Configuration](./04-configuration.md#smtp-and-queue--email).

### Using the JWT

After login, include the JWT in every request:

```http
Authorization: Bearer <your-jwt-token>
```

The JWT is valid for `jwtExpiresIn` seconds (default: 24 hours).

---

## Two-factor authentication (2FA / TOTP)

Users can add a second factor using any TOTP authenticator app (Google Authenticator,
Authy, 1Password, etc.).

### Enabling 2FA (user journey)

**Step 1 — Start activation:**

```http
POST /_adm/beginners/users/totp/enable
Authorization: Bearer <jwt>
```

Response:
```json
{
  "totpUrl": "otpauth://totp/MyApp:user@example.com?secret=BASE32SECRET&issuer=MyApp"
}
```

The user scans the `totpUrl` in their authenticator app (or adds the secret manually).

**Step 2 — Confirm activation:**

```http
POST /_adm/beginners/users/totp/validate-app
Authorization: Bearer <jwt>
Content-Type: application/json

{ "token": "123456" }
```

The `token` is the 6-digit code shown in the authenticator app. This confirms that the app
is correctly configured and activates 2FA on the account.

### Logging in with 2FA

When 2FA is enabled, the magic link login response includes `totp_required: true`. The client
must then call:

```http
POST /_adm/beginners/users/totp/check-token
Authorization: Bearer <jwt>
Content-Type: application/json

{ "token": "123456" }
```

Only after a successful TOTP check does the session have full access.

### Disabling 2FA

```http
POST /_adm/beginners/users/totp/disable
Authorization: Bearer <jwt>
Content-Type: application/json

{ "token": "123456" }
```

Requires a valid TOTP token to confirm the user's intent.

---

## Connection strings (service tokens)

A connection string is a long-lived API token tied to a specific account, tenant, and role.
Use them for:

- **Machine-to-machine calls** — scripts, cron jobs, or services that don't have a user session.
- **Telegram Mini Apps** — the gateway issues a connection string when a user logs in via Telegram
  (see [Alternative Identity Providers](./10-alternative-idps.md)).
- **Long-running sessions** — when the standard JWT expiry is too short.

### Creating a connection string

```http
POST /_adm/beginners/tokens
Authorization: Bearer <jwt>
Content-Type: application/json

{
  "tenantId": "a3f1e2d0-1234-4abc-8def-000000000001",
  "accountId": "b5e2f3a1-5678-4def-9abc-000000000002",
  "role": "manager",
  "expiresAt": "2027-01-01T00:00:00Z"
}
```

Response:
```json
{
  "connectionString": "acc=<uuid>;tid=<uuid>;r=manager;edt=2027-01-01T00:00:00Z;sig=<hmac>",
  "expiresAt": "2027-01-01T00:00:00Z"
}
```

### Listing your connection strings

```http
GET /_adm/beginners/tokens
Authorization: Bearer <jwt>
```

### Using a connection string

Instead of `Authorization: Bearer`, send the connection string in its own header:

```http
x-mycelium-connection-string: acc=<uuid>;tid=<uuid>;r=manager;edt=...;sig=...
```

The gateway checks `x-mycelium-connection-string` first. If absent, it falls back to
`Authorization: Bearer`. Do not mix them — a connection string sent as `Authorization: Bearer`
will fail JWT validation.

---

## OAuth2 / external providers

If you configure external OAuth2 providers (Google, Microsoft, Auth0), users authenticate
directly with the provider and present the provider's JWT to Mycelium. Mycelium validates
the token's signature using the provider's JWKS endpoint.

See [Configuration → External Authentication](./04-configuration.md#external-oauth2-providers)
for setup instructions.

---

## Fetching your own profile

Any authenticated user can fetch their full profile:

```http
GET /_adm/beginners/profile
Authorization: Bearer <jwt>
```

This is the same profile that Mycelium injects as `x-mycelium-profile` into downstream
requests. Useful for debugging or displaying account/tenant information in a frontend.
