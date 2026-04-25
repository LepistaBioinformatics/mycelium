# Mycelium Code Concerns & Technical Debt

**Analyzed: 2026-04-03**

## Security Concerns

### 1. Unwrap() Calls in Production Code
**Severity**: HIGH
**Evidence**: 
- `/mnt/external/thirdparty-projects/mycelium/core/src/settings.rs:31`
  - `panic!("Error on load tera templates: {}", err)` in settings initialization
- `/mnt/external/thirdparty-projects/mycelium/adapters/notifier/src/repositories/remote_message_sending.rs:42`
  - `.unwrap()` on email parsing: `(... email_str).parse().unwrap()`
- `/mnt/external/thirdparty-projects/mycelium/adapters/notifier/src/repositories/remote_message_sending.rs:44`
  - `.unwrap()` on recipient: `message.to_owned().to.email().parse().unwrap()`
- `/mnt/external/thirdparty-projects/mycelium/core/src/domain/dtos/error_code.rs:173, 190, 207, 235`
  - Multiple `.unwrap()` calls on string parsing

**Risk**: Panic crashes in production; invalid email addresses crash the notifier

**Fix**:
```rust
// Instead of:
(... email).parse().unwrap()

// Use:
match (... email).parse() {
    Ok(e) => e,
    Err(e) => return creation_err(format!("Invalid email format: {}", e))
}
```

### 2. Incomplete X-Forwarded-For Header Handling
**Severity**: MEDIUM
**Evidence**: `/mnt/external/thirdparty-projects/mycelium/ports/api/src/router/mod.rs:53-55`
```rust
/// TODO: This forwarded implementation is incomplete as it only handles the
/// TODO: unofficial X-Forwarded-For header but not the official Forwarded
/// TODO: one.
```

**Risk**: Client IP detection unreliable; can be spoofed via X-Forwarded-For; RFC 7239 compliance missing

**Fix**: Implement proper `Forwarded` header parsing per RFC 7239; validate X-Forwarded-For only from trusted proxies

### 3. Missing mTLS Certificate Authentication
**Severity**: MEDIUM
**Evidence**: `/mnt/external/thirdparty-projects/mycelium/core/src/domain/dtos/http_secret.rs:66`
```rust
// TODO: Implement client certificate authentication
```

**Risk**: Gateway cannot validate downstream service certificates; susceptible to MITM attacks

**Fix**: Implement mTLS validation in `inject_downstream_secret.rs`; load and validate client certificates

### 4. JWT Secret Not Validated Before Use
**Severity**: MEDIUM
**Evidence**: `/mnt/external/thirdparty-projects/mycelium/lib/http_tools/src/functions/encode_jwt.rs` (line 87)
```rust
let secret = match auth_config.jwt_secret.async_get_or_error().await {
    Ok(secret) => secret,
    Err(err) => return Err(err),
};
```

**Risk**: No validation that secret meets minimum length requirements (should be >= 256 bits for RS256)

**Fix**: Add validation:
```rust
if secret.len() < 32 {
    return creation_err("JWT secret too short (minimum 256 bits)");
}
```

### 5. No SQL Injection Prevention Verification
**Severity**: MEDIUM (mitigated by Diesel)
**Evidence**: `/mnt/external/thirdparty-projects/mycelium/adapters/diesel/sql/up.sql` uses raw SQL

**Risk**: While Diesel provides parameterization by default, custom raw SQL in migrations could be vulnerable

**Fix**: Verify all raw SQL in `sql/up.sql` uses parameterized queries; document SQL safety

## Tech Debt & Code Quality

### 6. High Frequency of unwrap_or_default() Calls
**Severity**: LOW
**Evidence**: Observed in multiple locations:
- `core/src/domain/dtos/security_group.rs:35` — `.unwrap_or_default()`
- `core/src/domain/dtos/service.rs:90` — `.choose(&mut rand::thread_rng()).unwrap()`
- `core/src/domain/dtos/service.rs:262` — `.unwrap_or_default()`

**Risk**: May mask actual errors; unclear intent when default is chosen

**Fix**: 
```rust
// Clearer intent:
match route.service.hosts.choose(&mut rand::thread_rng()) {
    Some(host) => host.clone(),
    None => return Err(execution_err("No hosts available")),
}
```

### 7. Incomplete Guest Role Endpoints
**Severity**: LOW
**Evidence**: `/mnt/external/thirdparty-projects/mycelium/ports/api/src/rest/manager/guest_role_endpoints.rs:27`
```rust
// TODO
```

**Risk**: Feature not implemented; may block guest user functionality

**Fix**: Complete implementation or remove endpoints from API documentation

### 8. Multiple TODOs in Critical Paths
**Severity**: LOW
**Evidence**:
- X-Forwarded-For handling (line 53-55, router/mod.rs)
- Client certificate auth (http_secret.rs:66)
- Guest role endpoints (guest_role_endpoints.rs:27)

**Risk**: Incomplete features may lead to security gaps or functional issues

**Fix**: Track TODOs in GitHub issues; prioritize and implement

## Performance & Scalability Concerns

### 9. Synchronous Config Loading in Async Context
**Severity**: MEDIUM
**Evidence**: `/mnt/external/thirdparty-projects/mycelium/lib/config/src/settings.rs`
```rust
lazy_static! {
    pub(crate) static ref VAULT_CONFIG: Mutex<Option<OptionalConfig<VaultConfig>>> = Mutex::new(None);
}

pub async fn init_vault_config_from_file(...) {
    // Initializes static; may block
    VAULT_CONFIG.lock().unwrap().replace(config);
}
```

**Risk**: Lock contention on first access; all async tasks wait on single Mutex

**Fix**: Use `async_once` crate or `tokio::sync::OnceCell` for proper async initialization

### 10. MemDb Full Scan for Route Matching
**Severity**: MEDIUM
**Evidence**: `/mnt/external/thirdparty-projects/mycelium/ports/api/src/router/match_downstream_route_from_request.rs`
(inferred from pattern)

**Risk**: In-memory store; O(n) route matching; performance degrades with service count

**Fix**: 
- Use HashMap keyed by service name + path pattern
- Implement trie for path matching
- Cache route lookups with TTL

### 11. No Connection Pool Limits Visible
**Severity**: MEDIUM
**Evidence**: PostgreSQL connection pool configuration not found in inspected files

**Risk**: Unbounded connections could exhaust database server

**Fix**: Verify pool size configuration in `DieselDbPoolProvider`; add connection pool limits

## Testing Gaps

### 12. API Endpoint Handlers Not Tested
**Severity**: HIGH
**Evidence**: `/mnt/external/thirdparty-projects/mycelium/ports/api/src/rest/` — no inline tests found

**Risk**: REST/RPC/MCP endpoint bugs not caught before production

**Fix**: Add integration tests for all endpoint handlers using `actix-rt::test`

### 13. Router Logic Not Unit Tested
**Severity**: HIGH
**Evidence**: `/mnt/external/thirdparty-projects/mycelium/ports/api/src/router/mod.rs` — no tests

**Risk**: Security-critical routing logic not validated; edge cases (header injection, path traversal) not checked

**Fix**: Add unit tests for:
- Source IP validation
- Method permission checks
- Secret injection
- Header filtering
- URL building

### 14. Database Repository Integration Tests Missing
**Severity**: MEDIUM
**Evidence**: `adapters/diesel/src/repositories/` — no inline tests; no integration test helper for DB

**Risk**: CRUD operations not validated; migrations may break without detection

**Fix**: Add testcontainers-based PostgreSQL integration tests

## Dependencies & Versions

### 15. Beta Version in Production
**Severity**: LOW
**Evidence**: `Cargo.toml`: `version = "8.3.1-beta.5"`

**Risk**: Not GA; breaking changes possible; May affect downstream users

**Fix**: Release GA version (8.3.0 or 8.4.0); update documentation

### 16. Multiple RUSTSEC Workarounds
**Severity**: MEDIUM
**Evidence**: `/mnt/external/thirdparty-projects/mycelium/Cargo.toml:125-140`
```rust
# RUSTSEC-2026-0007: force bytes >= 1.11.1
# RUSTSEC-2026-0009: force time >= 0.3.47
bytes = "1.11.1"
time = "0.3.47"
```

**Risk**: Transitive dependency vulnerabilities; need to monitor for updates

**Fix**:
- Add `cargo audit` to CI
- Set up Dependabot alerts
- Schedule monthly security reviews

### 17. Unmaintained rustls-pemfile Avoided
**Severity**: MEDIUM (mitigated)
**Evidence**: `/mnt/external/thirdparty-projects/mycelium/Cargo.toml:71-75`
```rust
# Use native-tls only to avoid unmaintained rustls-pemfile (RUSTSEC-2025-0134).
reqwest = { version = "0.11", default-features = false, features = [
    "json",
    "native-tls",
] }
```

**Risk**: Good mitigation; but native-tls has larger attack surface on some platforms

**Fix**: Monitor rustls-pemfile revival; consider switching if maintenance resumes

## Data Validation

### 18. Email Parsing Without Validation
**Severity**: MEDIUM
**Evidence**: `adapters/notifier/src/repositories/remote_message_sending.rs:42-48`
```rust
let email = LettreMessage::builder()
    .from((... email).parse().unwrap())  // Panics on invalid
    .to(message.to_owned().to.email().parse().unwrap())  // Panics
    .subject(message.to_owned().subject)
    .header(ContentType::TEXT_HTML)
    .body(message.to_owned().body)
    .unwrap();
```

**Risk**: Invalid email addresses crash notifier; no validation in DTO layer

**Fix**: Add email validation in `Email` DTO constructor; return error type

### 19. JSONB Serialization Without Schema Validation
**Severity**: MEDIUM
**Evidence**: `adapters/diesel/sql/up.sql` — many JSONB columns (meta, status, trigger, etc.) with no constraints

**Risk**: Invalid JSON silently accepted; query errors at runtime; schema drift

**Fix**:
- Add JSON schema validation constraints
- Document JSONB schema per field
- Add schema migration tests

## Operational Concerns

### 20. Panic on Template Directory Missing
**Severity**: HIGH
**Evidence**: `/mnt/external/thirdparty-projects/mycelium/core/src/settings.rs:24-31`
```rust
pub(crate) fn load_settings() -> Settings {
    match TERA.lock() {
        Ok(tera) => tera,
        Err(err) => panic!("Error on load tera templates: {}", err),
    }
}
```

**Risk**: Runtime panic if templates/ directory missing or invalid; no graceful fallback

**Fix**:
```rust
pub fn load_settings() -> Result<Settings, MappedErrors> {
    TERA.lock()
        .map_err(|e| execution_err(format!("Template loading failed: {}", e)))
}
```

### 21. Lazy-Static Config No Reload Support
**Severity**: LOW
**Evidence**: `lib/config/src/settings.rs` — all config is lazy-static, immutable

**Risk**: Cannot hot-reload config; requires restart to apply changes

**Fix**: Implement config reload endpoint for non-critical settings (log level, etc.)

### 22. No Rate Limiting on Public Endpoints
**Severity**: MEDIUM
**Evidence**: Gateway routing has no rate limiting observed

**Risk**: DDoS vulnerable; can exhaust downstream services

**Fix**: Add actix-web rate limiting middleware; configure per-IP/per-route limits

## Recommendations Summary

| Category | Count | Priority |
|----------|-------|----------|
| Security | 5 | HIGH (fix unwrap), MEDIUM (mTLS, header validation) |
| Testing | 3 | HIGH (API tests), MEDIUM (repo tests) |
| Tech Debt | 5 | LOW-MEDIUM (TODOs, unwrap_or_default, config) |
| Performance | 4 | MEDIUM (pool limits, config, route matching) |
| Data Validation | 2 | MEDIUM (email, JSONB schema) |
| Operations | 3 | HIGH (panic on startup), MEDIUM (rate limiting) |

### Immediate Actions (Next Sprint)
1. Remove all `.unwrap()` from email handling
2. Add rate limiting middleware
3. Add API endpoint integration tests
4. Add router logic unit tests

### Next Quarter
1. Implement proper X-Forwarded-For / RFC 7239 Forwarded header handling
2. Implement mTLS certificate validation
3. Add database integration tests with testcontainers
4. Implement configuration hot-reload

