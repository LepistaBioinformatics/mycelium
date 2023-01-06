# MYCELIUM API: The main service port of the Mycelium project

## Development Usage

### Configure environment

```bash
export RUST_LOG=actix_web=debug,clean_base=debug,myc_api=debug
export RUST_BACKTRACE=full
export SERVICE_PORT=8081
export ALLOWED_ORIGINS='http://localhost:8081,http://localhost:8082'
export TOKENS_EXPIRATION_TIME=360000
export TOKENS_VALIDATION_PATH="http://localhost:8081/services/tokens/"
```

### Run the application in development mode

```bash
cargo run \
    --package mycelium-api \
    --bin myc-api
```
