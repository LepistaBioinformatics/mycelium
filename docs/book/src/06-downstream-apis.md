# Downstream APIs Configuration

Downstream services are configured directly in the main TOML configuration file. This guide explains how to configure routes, security groups, and authentication for your backend services.

## Configuration Location

All service and route configurations are defined in your main `config.toml` file under the `[api.services]` section:

```bash
SETTINGS_PATH=settings/config.toml myc-api
```

## Service Configuration

Services are defined using TOML's array of tables syntax (`[[service-name]]`). Each service includes configuration options and associated routes.

### Required Fields

**Service Name**: The table name (e.g., `[[my-service]]`)
- Used to identify the service in the gateway URL path
- Should use kebab-case (e.g., `my-service`)

**`host`**: The service host
- Should include the port number
- Do not include the protocol
- Example: `"localhost:3000"` or `"api.example.com:443"`

**`protocol`**: The connection protocol
- Options: `"http"` or `"https"`

**Routes** (`[[service-name.path]]`): Array of route definitions
- Each route is defined as `[[service-name.path]]`
- See the [Routes Section](#routes) for details

### Optional Fields

**`discoverable`**: Enable AI-aware service discovery
- Set to `true` to make the service discoverable by LLM agents
- Default: `false`

**`id`**: Unique identifier (UUID v4 recommended)
**`description`**: Human-readable service description
**`openapiPath`**: Path to OpenAPI specification
**`healthCheckPath`**: Health check endpoint path
**`capabilities`**: Array of capability strings
**`serviceType`**: Type of service (e.g., `"rest-api"`)
**`isContextApi`**: Whether this is a context API
**`proxyAddress`**: Proxy server address (if using a proxy)

### Basic Service Example

```toml
[api.services]

[[my-service]]
host = "localhost:3000"
protocol = "http"

[[my-service.path]]
group = "public"
path = "/api/users/*"
methods = ["GET", "POST", "PUT", "DELETE"]
```

### Service with Multiple Hosts (Load Balancing)

```toml
[[api-service]]
hosts = ["api-01.example.com:8080", "api-02.example.com:8080", "api-03.example.com:8080"]
protocol = "https"

[[api-service.path]]
group = "authenticated"
path = "/api/*"
methods = ["ALL"]
```

## Secrets Configuration

Secrets are used to authenticate Mycelium with downstream services.

### Query Parameter Secret

```toml
[[legacy-api]]
host = "legacy.example.com:8080"
protocol = "https"

[[legacy-api.secret]]
name = "legacy-token"
queryParameter = { name = "token", token = "my-secret-token" }

[[legacy-api.path]]
group = "public"
path = "/legacy/*"
methods = ["GET"]
secretName = "legacy-token"
```

### Authorization Header Secret

```toml
[[protected-api]]
host = "api.example.com:443"
protocol = "https"

[[protected-api.secret]]
name = "auth-header"
authorizationHeader = { name = "Authorization", prefix = "Bearer ", token = "my-bearer-token" }

[[protected-api.path]]
group = "public"
path = "/protected/*"
methods = ["ALL"]
secretName = "auth-header"
```

### Secrets from Environment Variables

```bash
# Set environment variable with inline TOML format
export MY_SECRET='{ queryParameter = { name = "token", token = "my-secret-01" } }'
```

```toml
[[my-service]]
host = "localhost:3000"
protocol = "http"

[[my-service.secret]]
name = "secret-query-token"
env = "MY_SECRET"
```

### Secrets from Vault

```toml
[[my-service]]
host = "localhost:3000"
protocol = "http"

[[my-service.secret]]
name = "secret-auth-header"
vault = { path = "myc/services/my-service", key = "authHeader" }
```

## AI-Aware Service Discovery

When `discoverable = true`, additional configuration options enhance AI discoverability:

```toml
[[auth-service]]
id = "550e8400-e29b-41d4-a716-446655440000"
host = "auth.example.com:443"
protocol = "https"
discoverable = true
description = "Authentication and authorization service"
openapiPath = "/api/openapi.json"
healthCheckPath = "/health"
capabilities = ["authenticate", "OAuth2", "2FA", "JWT"]
serviceType = "rest-api"
isContextApi = true

[[auth-service.path]]
group = "public"
path = "/auth/*"
methods = ["POST"]
```

## Routes

Routes define how requests are matched and secured. Each route is defined as `[[service-name.path]]`.

### Required Fields

**`group`**: Security group for the route
- Determines authentication and authorization requirements
- See [Security Groups](#security-groups) for options

**`path`**: URL path pattern
- Supports wildcards: `/users/*`
- Exact matches: `/users/profile`

**`methods`**: Array of HTTP methods
- Standard: `["GET"]`, `["POST", "PUT"]`, etc.
- Special: `["ALL"]` matches all methods

### Optional Fields

**`secretName`**: Reference to a secret defined at service level
**`acceptInsecureRouting`**: Allow self-signed certificates (default: `false`)

## Security Groups

Mycelium supports seven security groups for route protection:

### 1. Public Routes

No authentication required.

```toml
[[health-service]]
host = "localhost:8080"
protocol = "http"

[[health-service.path]]
group = "public"
path = "/health"
methods = ["GET"]
```

### 2. Authenticated Routes

Requires valid JWT token. User email injected in `x-mycelium-email` header.

```toml
[[user-service]]
host = "users.example.com:443"
protocol = "https"

[[user-service.path]]
group = "authenticated"
path = "/user/profile"
methods = ["GET", "PUT"]
```

### 3. Protected Routes

Requires authentication and valid profile. Full profile injected in `x-mycelium-profile` header.

```toml
[[dashboard-service]]
host = "dashboard.example.com:443"
protocol = "https"

[[dashboard-service.path]]
group = "protected"
path = "/dashboard/*"
methods = ["GET"]
```

### 4. Protected by Roles

Requires authentication and specific role(s).

```toml
[[admin-service]]
host = "admin.example.com:443"
protocol = "https"

[[admin-service.path]]
group = { protectedByRoles = [{ slug = "admin" }, { slug = "super-admin" }] }
path = "/admin/*"
methods = ["ALL"]
```

### 5. Protected by Roles with Permissions

Requires authentication and specific role-permission combinations.

```toml
[[content-service]]
host = "content.example.com:443"
protocol = "https"

[[content-service.path]]
group = { protectedByRoles = [{ slug = "editor", permission = "write" }, { slug = "admin", permission = "write" }] }
path = "/content/edit/*"
methods = ["POST", "PUT", "DELETE"]

[[content-service.path]]
group = { protectedByRoles = [{ slug = "viewer", permission = "read" }, { slug = "editor", permission = "read" }] }
path = "/content/view/*"
methods = ["GET"]
```

### 6. Protected by Service Token with Role

Requires valid service token with specific role(s).

```toml
[[service-api]]
host = "service.internal:8080"
protocol = "http"

[[service-api.path]]
group = { protectedByServiceTokenWithRole = { roles = ["service-admin"] } }
path = "/service-api/*"
methods = ["ALL"]
```

Service token format:
```
tid=c8282c6d-ce0b-4fed-a4e8-8e8a70f5b789;rid=8d7b119b-a12b-4ff1-9db0-5b6d05794282;r=newbie;edt=2025-01-11T21:51:01-03:00;sig=asd132f141e1...
```

### 7. Protected by Service Token with Permissioned Roles

Requires valid service token with specific role-permission combinations.

```toml
[[service-data]]
host = "data.internal:8080"
protocol = "http"

[[service-data.path]]
group = { protectedByServiceTokenWithPermissionedRoles = { permissionedRoles = [["admin", "write"], ["viewer", "read"]] } }
path = "/service-data/*"
methods = ["GET", "POST"]
```

## Complete Example

Here's a complete configuration with multiple services:

```toml
[api.services]

# Health check endpoint
[[health-check]]
host = "localhost:8080"
protocol = "http"

[[health-check.path]]
group = "public"
path = "/health"
methods = ["GET"]

# User service with authentication
[[user-service]]
host = "users.example.com:443"
protocol = "https"

[[user-service.path]]
group = "authenticated"
path = "/users/me"
methods = ["GET", "PUT"]

[[user-service.path]]
group = "protected"
path = "/users/preferences"
methods = ["GET", "POST"]

# Admin service with role-based access
[[admin-service]]
host = "admin.example.com:443"
protocol = "https"

[[admin-service.path]]
group = { protectedByRoles = [{ slug = "admin" }, { slug = "super-admin" }] }
path = "/admin/*"
methods = ["ALL"]

# Content service with permissioned roles
[[content-service]]
host = "content.example.com:443"
protocol = "https"
discoverable = true
description = "Content management service"
healthCheckPath = "/health"
capabilities = ["content-management", "versioning"]

[[content-service.path]]
group = { protectedByRoles = [{ slug = "editor", permission = "write" }] }
path = "/content/edit/*"
methods = ["POST", "PUT", "DELETE"]

[[content-service.path]]
group = { protectedByRoles = [{ slug = "viewer", permission = "read" }] }
path = "/content/view/*"
methods = ["GET"]

# Legacy API with authentication secret
[[legacy-api]]
host = "legacy.internal:8080"
protocol = "http"

[[legacy-api.secret]]
name = "legacy-token"
queryParameter = { name = "api_key", token = { env = "LEGACY_API_KEY" } }

[[legacy-api.path]]
group = "public"
path = "/legacy/*"
methods = ["GET"]
secretName = "legacy-token"
acceptInsecureRouting = true
```

## Header Injection

Mycelium automatically injects headers to downstream services based on the security group:

| Security Group | Injected Headers |
|----------------|------------------|
| `authenticated` | `x-mycelium-email` |
| `protected` | `x-mycelium-profile` |
| `protectedByRoles` | `x-mycelium-profile` |

The `x-mycelium-profile` header contains a JSON object with user information, roles, and permissions.

## Testing Routes

Test your route configuration using curl:

```bash
# Test public route
curl http://localhost:8080/health

# Test authenticated route (requires JWT)
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  http://localhost:8080/users/me

# Test with service token
curl -H "Authorization: ServiceToken tid=...;rid=...;r=admin;edt=...;sig=..." \
  http://localhost:8080/service-api/data
```

## Troubleshooting

**Route not matching:**
- Verify path pattern includes wildcards if needed (`/*`)
- Check method is in the `methods` array
- Ensure service name is correct in URL path

**Authentication failing:**
- Verify JWT token is valid and not expired
- Check security group matches authentication method
- Ensure roles/permissions are correctly assigned

**Downstream service not reachable:**
- Verify host and port are correct
- Check protocol (http vs https) matches
- Test direct connection to downstream service
- Review `acceptInsecureRouting` for self-signed certs

**Configuration syntax errors:**
- Validate TOML syntax using an online validator
- Check array of tables syntax: `[[service-name]]`
- Ensure inline tables use correct syntax: `{ key = "value" }`

## Next Steps

- [Authorization Model](./01-authorization.md) - Understanding security in depth
- [Configuration Guide](./04-configuration.md) - Main configuration options
- [Running Tests](./07-running-tests.md) - Test your setup

For more help, visit the [GitHub Issues](https://github.com/LepistaBioinformatics/mycelium/issues) page.
