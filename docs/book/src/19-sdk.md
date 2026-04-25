# SDK Integration Guide

Mycelium injects a compressed, encoded identity context into every downstream request via
the `x-mycelium-profile` header. This header carries the authenticated user's full profile:
account ID, tenant memberships, roles, and access scopes.

The **Python SDK** (`mycelium-http-tools`) decodes this header and provides a fluent filtering
API for making access decisions inside your service.

---

## Installation

```bash
pip install mycelium-http-tools
```

PyPI package: [`mycelium-http-tools`](https://pypi.org/project/mycelium-http-tools/)

Source: [github.com/LepistaBioinformatics/mycelium-sdk-py](https://github.com/LepistaBioinformatics/mycelium-sdk-py)

---

## What the header contains

When a request passes through a `protected` or `protectedByRoles` route, the gateway:

1. Resolves the caller's identity (from JWT or connection string).
2. Constructs a `Profile` object: account ID, email, tenant memberships, roles, access scopes.
3. ZSTD-compresses the JSON-serialized profile.
4. Base64-encodes the result.
5. Injects it as `x-mycelium-profile` in the forwarded request.

Your service never sees unauthenticated requests on protected routes. The SDK decodes the
header back into a typed `Profile` object.

---

## Using the SDK with FastAPI

The SDK ships a FastAPI middleware and dependency injectors:

```python
from fastapi import FastAPI, Depends
from myc_http_tools.fastapi import get_profile, MyceliumProfileMiddleware
from myc_http_tools.models.profile import Profile

app = FastAPI()
app.add_middleware(MyceliumProfileMiddleware)

@app.get("/dashboard")
async def dashboard(profile: Profile = Depends(get_profile)):
    # profile is already decoded and validated
    return {"account_id": str(profile.acc_id)}
```

---

## Making access decisions

The `Profile` object provides a fluent filtering chain. Each step narrows — never expands —
the set of permissions:

```python
from myc_http_tools.models.profile import Profile
from myc_http_tools.exceptions import InsufficientPrivilegesError

def get_tenant_account(profile: Profile, tenant_id: UUID, account_id: UUID):
    try:
        account = (
            profile
            .on_tenant(tenant_id)       # focus on this tenant
            .on_account(account_id)     # focus on this account
            .with_write_access()        # must have write permission
            .with_roles(["manager"])    # must have manager role
            .get_related_account_or_error()
        )
        return account
    except InsufficientPrivilegesError:
        raise HTTPException(status_code=403)
```

If any step in the chain finds no match (wrong tenant, no write access, missing role), it raises
`InsufficientPrivilegesError`. Your handler catches it and returns 403.

---

## Available filter methods

| Method | Effect |
|---|---|
| `.on_tenant(tenant_id)` | Filter to a specific tenant membership |
| `.on_account(account_id)` | Filter to a specific account within the tenant |
| `.with_read_access()` | Require at least read permission |
| `.with_write_access()` | Require write permission |
| `.with_roles(["role1", "role2"])` | Require at least one of the listed roles |
| `.get_related_account_or_error()` | Return the matched account or raise |

---

## Header constants

The SDK exports the same header key constants as the gateway. Use them instead of raw strings
to prevent mismatches if header names change:

```python
from myc_http_tools.settings import (
    DEFAULT_PROFILE_KEY,        # "x-mycelium-profile"
    DEFAULT_EMAIL_KEY,          # "x-mycelium-email"
    DEFAULT_SCOPE_KEY,          # "x-mycelium-scope"
    DEFAULT_MYCELIUM_ROLE_KEY,  # "x-mycelium-role"
    DEFAULT_REQUEST_ID_KEY,     # "x-mycelium-request-id"
    DEFAULT_CONNECTION_STRING_KEY,  # "x-mycelium-connection-string"
    DEFAULT_TENANT_ID_KEY,      # "x-mycelium-tenant-id"
)
```

---

## Manual decoding (any language)

If you are not using Python, decode `x-mycelium-profile` manually:

```
Base64-decode → ZSTD-decompress → JSON-parse → access Profile fields
```

Example in shell (for debugging):

```bash
echo "<header-value>" | base64 -d | zstd -d | python3 -m json.tool
```

The resulting JSON has the same structure as the Rust `Profile` struct:
`acc_id`, `email`, `tenants` (array of tenant memberships with roles and accounts).

---

## Keeping the SDK in sync with the gateway

The `Profile` Pydantic model in the SDK mirrors the Rust `Profile` struct in the gateway.
When the gateway changes the `Profile` struct (new fields, renamed fields), the SDK must be
updated to match. If your service receives a profile that does not match the SDK's model,
deserialization will fail with a validation error.

Check the [SDK changelog](https://github.com/LepistaBioinformatics/mycelium-sdk-py/releases)
when upgrading the gateway.
