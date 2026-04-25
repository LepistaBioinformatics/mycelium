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

The built-in (native) error codes are seeded into the database on first run using the CLI:

```bash
myc-cli native-errors init
```

This command is run once during installation. It populates the database with all error codes
that the core domain layer uses internally. Without this step, error responses that carry domain
codes will have no human-readable message attached.

See [CLI Reference](./18-cli.md) for full usage.

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

## Error code API reference

| JSON-RPC method | Description |
|---|---|
| `systemManager.errorCodes.create` | Register a new error code |
| `systemManager.errorCodes.list` | List all error codes (native + custom) |
| `systemManager.errorCodes.get` | Get a single error code by code string |
| `systemManager.errorCodes.updateMessageAndDetails` | Update an existing code's message and details |
| `systemManager.errorCodes.delete` | Remove a custom error code |

Native error codes (prefixed `MYC`) cannot be deleted — only their message and details can be
updated if you want to localize them.
