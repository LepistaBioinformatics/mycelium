# Response Callbacks

After Mycelium forwards a request to a downstream service and receives a response, it can
optionally run **callbacks** — side effects triggered by the response. Use callbacks to log
metrics, notify third-party services, or trigger async workflows without modifying the
downstream service.

Callbacks run **after** the response has been returned to the caller. They never block or
modify the response.

---

## How it works

```
Client → Mycelium → Downstream service
                         ↓ response
Mycelium returns response to client
       ↓ (in parallel or fire-and-forget)
Callbacks execute
```

---

## Defining a callback

Callbacks are defined in `config.toml` under `[api.callbacks]`, then referenced by name on
individual routes.

### HTTP callback — POST the response context to a URL

```toml
[api.callbacks]

[[callback]]
name = "audit-log"
type = "http"
url = "https://audit.internal/events"
method = "POST"          # default
timeout_ms = 3000
retry_count = 3
retry_interval_ms = 1000
```

### Python callback — run a script

```toml
[[callback]]
name = "metrics-push"
type = "python"
script_path = "/opt/mycelium/callbacks/push_metrics.py"
python_path = "/usr/bin/python3.12"
timeout_ms = 5000
```

### JavaScript callback — run a Node.js script

```toml
[[callback]]
name = "slack-notify"
type = "javascript"
script_path = "/opt/mycelium/callbacks/notify_slack.js"
node_path = "/usr/bin/node"
timeout_ms = 3000
```

---

## Attaching a callback to a route

Reference the callback by name in the route's `callbacks` field:

```toml
[api.services]

[[my-service]]
host = "localhost:3000"
protocol = "http"

[[my-service.path]]
group = "protected"
path = "/api/*"
methods = ["POST", "PUT"]
callbacks = ["audit-log", "metrics-push"]
```

Multiple callbacks can be attached to the same route.

---

## Filtering which responses trigger the callback

By default, a callback runs for every response. Use filters to narrow this:

```toml
[[callback]]
name = "error-alert"
type = "http"
url = "https://alerts.internal/errors"

# Only trigger on 5xx responses
[callback.triggering_status_codes]
oneOf = [500, 502, 503, 504]

# Only trigger on POST and DELETE
[callback.triggering_methods]
oneOf = ["POST", "DELETE"]

# Only trigger if the response has a specific header
[callback.triggering_headers.oneOf]
"X-Error-Code" = "PAYMENT_FAILED"
```

Filter types:
- **`oneOf`** — at least one value must match
- **`allOf`** — all values must match
- **`noneOf`** — none of the values may match

---

## Execution mode

Control how callbacks run globally:

```toml
[api]
callbackExecutionMode = "fireAndForget"  # default
```

| Mode | Behavior |
|---|---|
| `fireAndForget` | Callbacks run in background tasks; gateway does not wait for them |
| `parallel` | All callbacks run concurrently; gateway waits for all to finish |
| `sequential` | Callbacks run one after another; gateway waits |

Use `fireAndForget` (default) when callback latency should not affect response time. Use
`sequential` when order matters (e.g., log before notify).

---

## What the callback receives

Each callback receives a **context object** with information about the completed request:

| Field | Description |
|---|---|
| `status_code` | HTTP status code returned by the downstream service |
| `response_headers` | Response headers |
| `duration_ms` | Time from gateway forwarding to downstream response |
| `upstream_path` | The path the client called |
| `downstream_url` | The URL Mycelium forwarded to |
| `method` | HTTP method |
| `timestamp` | ISO 8601 timestamp |
| `request_id` | Value of `x-mycelium-request-id` if present |
| `client_ip` | Caller's IP address |
| `user_info` | Authenticated user info (email, account ID) — present when route is `authenticated` or higher |
| `security_group` | The security group that was applied |

For **HTTP callbacks**, this context is sent as a JSON POST body.
For **Python / JavaScript callbacks**, the context is passed as a JSON-serialized argument.

---

## Reference — callback fields

| Field | Type | Description |
|---|---|---|
| `name` | string | Unique name — used to reference the callback from routes |
| `type` | `http` / `python` / `javascript` | Callback engine |
| `timeout_ms` | integer | Max execution time in ms (default: 5000). Ignored in `fireAndForget` mode |
| `retry_count` | integer | How many times to retry on failure (default: 3) |
| `retry_interval_ms` | integer | Wait between retries in ms (default: 1000) |
| `url` | string | HTTP only — target URL |
| `method` | string | HTTP only — `POST`, `PUT`, `PATCH`, or `DELETE` (default: `POST`) |
| `script_path` | path | Python / JavaScript only — path to script file |
| `python_path` | path | Python only — interpreter path (default: system `python3`) |
| `node_path` | path | JavaScript only — Node.js path (default: system `node`) |
