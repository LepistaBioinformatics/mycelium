# Mycelium Downstream Service

This is a simple service that is used to test the Mycelium API gateway with a
downstream service.

## Running the service

```bash
cargo run
```

## Routes

Some endpoints perform filtering of the Mycelium profile according to input
parameters such as roles and permissions. Other endpoints simply mirror the
input data in the response.

### GET /health
- **Input:** None
- **Output:** Status code 200 with "success" message when the service is healthy

### GET /public  
- **Input:** None
- **Output:** Status code 200 with "success" message

### GET /protected
- **Input:** Mycelium profile in the request header
- **Output:** Returns the same profile without filtering

### GET /protected/roles/{role}
- **Input:** 
    - Mycelium profile in the header
    - Role in the path parameter
- **Output:** Profile filtered by role with related accounts

### GET /protected/roles/{role}/with-permission
- **Input:**
    - Mycelium profile in the header
    - Role in the path parameter
    - Permission (read/write) as query parameter
- **Output:** Profile filtered by role and permission, with related accounts and
  access level

### GET /protected/role/{role}/with-scope
- **Input:**
    - Service token with scope in the x-mycelium-scope header
    - Role in the path parameter
- **Output:** Not implemented

### GET /protected/expects-headers
- **Input:** Arbitrary headers
- **Output:** Mirrors the received headers as key-value pairs

### POST /webhooks/account-created
- **Input:** Account creation payload (id, name, created)
- **Output:** Mirrors the received payload

### PUT /webhooks/account-updated
- **Input:** Account update payload (id, name, created)
- **Output:** Mirrors the received payload

### DELETE /webhooks/account-deleted
- **Input:** Account deletion payload (id, name, created)
- **Output:** Mirrors the received payload

### GET /secrets/authorization-header
- **Input:** Token in the Authorization header
- **Output:** Value of the received Authorization header

### GET /secrets/query-parameter-token
- **Input:** Token as query parameter
- **Output:** Value of the received token
