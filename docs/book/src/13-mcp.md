# MCP — AI Agent Integration

Mycelium exposes a **Model Context Protocol (MCP)** endpoint that lets AI assistants (Claude,
GPT, and any MCP-compatible agent) call your downstream APIs as tools — without needing direct
access to your services.

---

## What is MCP?

MCP is an open protocol for connecting AI assistants to external tools and data. With Mycelium's
MCP support, an AI agent can:

- **Discover** which operations your downstream services expose.
- **Call** those operations on behalf of an authenticated user.
- **Receive** the responses and incorporate them into a response to the user.

The AI never bypasses Mycelium's authentication or authorization. Every tool call is subject to
the same security checks as a direct API request from a client.

---

## Prerequisites

For MCP to discover operations, your downstream services must:

1. Expose an **OpenAPI spec** at a known path.
2. Be registered in Mycelium's config with `discoverable = true` and `openapiPath` set.

```toml
[[my-service]]
host = "api.internal:4000"
protocol = "http"
discoverable = true
openapiPath = "/api/openapi.json"
description = "Customer data API"
capabilities = ["customer-search", "order-history"]

[[my-service.path]]
group = "protected"
path = "/api/*"
methods = ["ALL"]
```

---

## The MCP endpoint

```
POST /mcp
```

This single endpoint handles all MCP JSON-RPC requests. It supports:

| MCP method | What it does |
|---|---|
| `initialize` | Returns Mycelium's MCP server info and capabilities |
| `tools/list` | Returns all operations discovered from `discoverable` services |
| `tools/call` | Executes a specific operation through the gateway |

---

## How tool calls work

When an AI calls `tools/call`, Mycelium:

1. Resolves the operation to the registered downstream service.
2. Validates the caller's token (forwarded from the original MCP request header).
3. Forwards the call through the gateway — including profile injection and all security checks.
4. Returns the downstream response as a tool result.

The AI passes its `Authorization: Bearer <jwt>` or `x-mycelium-connection-string` header in
the MCP request, and Mycelium forwards it when calling the downstream service.

---

## Connecting an AI assistant to Mycelium

Point the AI agent's MCP server URL to your Mycelium instance:

```
http://your-gateway:8080/mcp
```

In Claude Desktop's MCP configuration:

```json
{
  "mcpServers": {
    "mycelium": {
      "url": "http://your-gateway:8080/mcp"
    }
  }
}
```

The assistant will authenticate as the user whose token is configured, and will only be able
to call operations that user is authorized to access.

---

## Tool naming

Mycelium builds tool names deterministically from the service name and OpenAPI operation path:
`{service_name}__{http_method}__{path_slug}`.

For example, an operation `GET /api/customers/{id}` on service `customer-api` becomes the
tool name `customer-api__get__api_customers_id`.

---

## Notes

- MCP support requires `discoverable = true` and a valid `openapiPath` on at least one service.
- Operations on non-discoverable services are never exposed to the MCP endpoint.
- The MCP endpoint itself does not require authentication to connect, but individual `tools/call`
  requests are forwarded with the caller's token — so unauthorized calls will be rejected with
  `401`/`403` by the downstream route's security group.
