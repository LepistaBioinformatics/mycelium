# Error Codes

Mycelium has a built-in error code registry. Error codes give structured names to domain-specific
errors so that clients can handle them programmatically rather than parsing error message strings.

---

## What error codes are

A Mycelium error code is a short, opaque identifier (e.g. `MYC00023`) paired with a human-readable
message and optional detail text. When a use-case returns a domain error with a code, Mycelium
includes that code in the HTTP response body:

```json
{
  "message": "Account not found.",
  "code": "MYC00023",
  "details": "No account with this ID exists in the requested tenant."
}
```

Clients that need to distinguish between, say, "account not found" and "account archived" can
`switch` on `code` rather than parsing the `message` string.

---

## Native error codes

The built-in (native) error codes are defined in the gateway itself, so they need no
installation step: every listing and lookup serves them straight from the code, ahead of the
custom codes stored in the database. Their message, details and `isInternal` flag always match
the running version.

The CLI command `myc-cli native-errors init`, which used to copy them into the database, is no
longer needed. Native rows it stored earlier are ignored by listings and lookups, so they can
neither duplicate nor contradict the built-in definitions. See [CLI Reference](./18-cli.md).

Profile signing adds three internal codes (see
[Downstream Profile Signature](./27-downstream-profile-signature.md)):

| Code | Meaning |
|---|---|
| `MYC00041` | A profile signing key could not be derived from `core.accountLifeCycle.hmacSecrets` (secret unresolved or self-test failed); the gateway refuses to start |
| `MYC00042` | The profile signature could not be issued for a request; the request is not forwarded |
| `MYC00043` | Invalid verification setting in a Rust downstream service (`GatewayProfileData`): an invalid `MYCELIUM_JWKS_URL`, tolerance or insecure-HTTP switch, a missing audience while `MYCELIUM_JWKS_URL` is set, or an unsafe published key set; requests fail closed |

---

## Custom error codes

You can define additional error codes for your own downstream services. This lets you publish
a shared error vocabulary between the gateway and the services behind it.

Custom codes are managed through the `systemManager.errorCodes.*` JSON-RPC methods or the
equivalent REST routes. Requires the **system-manager** role.

### Create a custom error code

```json
{
  "jsonrpc": "2.0",
  "method": "systemManager.errorCodes.create",
  "params": {
    "code": "SVC00001",
    "message": "User has exceeded their rate limit.",
    "details": "The account has made more requests than allowed in the current window."
  },
  "id": 1
}
```

### List all error codes

```json
{
  "jsonrpc": "2.0",
  "method": "systemManager.errorCodes.list",
  "params": {},
  "id": 1
}
```

---

## Public error code lookup

People who receive an error code (e.g. `MYC00023`) can look it up without signing in. The
public endpoint returns only the codes registered with `isInternal = false`; internal codes are
never listed.

```bash
curl "https://gateway.example.com/error-codes?prefix=MYC&code=23"
```

| Query parameter | Description |
|---|---|
| `prefix` | Filter by prefix (e.g. `MYC`) |
| `code` | Filter by error number (e.g. `23` for `MYC00023`) |
| `pageSize` | Page size; capped at 100 |
| `skip` | Number of records to skip |

The response is `200` with a paginated body (`count`, `skip`, `size`, `records`), `204` when no
code matches, and `500` on a repository failure.

---

## Error code API reference

| JSON-RPC method | Description |
|---|---|
| `systemManager.errorCodes.create` | Register a new error code |
| `systemManager.errorCodes.list` | List all error codes (native + custom) |
| `systemManager.errorCodes.get` | Get a single error code by code string |
| `systemManager.errorCodes.updateMessageAndDetails` | Update an existing code's message and details |
| `systemManager.errorCodes.delete` | Remove a custom error code |

Native error codes (prefixed `MYC`) are read-only: updating or deleting them is refused
(`MYC00018`), and custom codes cannot be registered under the `MYC` prefix.
