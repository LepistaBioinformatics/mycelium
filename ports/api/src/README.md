# MYCELIUM API: The main service port of the Mycelium project

## Development Usage

### Configure environment

```bash

# Configure API basic control parameters
export ALLOWED_ORIGINS="http://localhost:8080,http://localhost:3000"
export SERVICE_WORKERS=1
export SERVICE_PORT=8080

# Configure logging
export RUST_LOG=actix_web=debug,clean_base=debug,myc_api=debug
export RUST_BACKTRACE=full

# Configure routes
export SOURCE_FILE_PATH=test/mock/routes.yaml

# Configure Google integration variables
## Redirect and origins
export GOOGLE_CLIENT_ORIGIN="http://localhost:3000"
export GOOGLE_OAUTH_REDIRECT_URL="http://localhost:3000"
## Token times
export GOOGLE_TOKEN_EXPIRED_IN="60m"
export GOOGLE_TOKEN_MAX_AGE="60"
## Secrets
export GOOGLE_JWT_SECRET="..."
export GOOGLE_OAUTH_CLIENT_ID="..."
export GOOGLE_OAUTH_CLIENT_SECRET="..."

```

### Run the application in development mode

```bash
cargo run \
    --package mycelium-api \
    --bin myc-api
```
