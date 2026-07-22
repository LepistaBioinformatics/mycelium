# ? ----------------------------------------------------------------------------
# ? Build stage/
# ? ----------------------------------------------------------------------------

FROM rust:latest AS builder

WORKDIR /rust

# ? The copy operations are performed in separate steps to allow caching layers
# ? over building operations

ARG VERSION="latest"
ENV VERSION=${VERSION}
RUN echo "Building mycelium-api version: ${VERSION}"

# ? Cargo features to build. Default reproduces the historical full-mode image
# ? byte-for-byte (default `postgres-backend` + `rhai`). Override to build the
# ? Redis-free Postgres-only mode:
# ?   docker build --build-arg CARGO_FEATURES=postgres-only,rhai .
# ? NOTE: `cargo install` from crates.io only works AFTER `mycelium-postgres-kv`
# ? and the `postgres-only` feature are published. The first postgres-only image
# ? (pre-publish) must be a source build instead:
# ?   cargo build --release --no-default-features --features postgres-only,rhai -p mycelium-api
ARG CARGO_FEATURES="postgres-backend,rhai"
ENV CARGO_FEATURES=${CARGO_FEATURES}

# ? If the VERSION is latest, instal using cargo install
# ? Otherwise, install using the --version flag
RUN if [ "${VERSION}" = "latest" ]; then \
        echo "Installing mycelium-api (features: ${CARGO_FEATURES})"; \
        cargo install mycelium-api --no-default-features --features "${CARGO_FEATURES}"; \
        echo "mycelium-api installed successfully"; \
        echo "Version: $(myc-api --version)"; \
    else \
        echo "Installing mycelium-api ${VERSION} (features: ${CARGO_FEATURES})"; \
        cargo install mycelium-api --no-default-features --features "${CARGO_FEATURES}" --version ${VERSION}; \
        echo "mycelium-api installed successfully"; \
        echo "Version: $(myc-api --version)"; \
    fi

# ? ----------------------------------------------------------------------------
# ? Production stage
# ? ----------------------------------------------------------------------------

FROM rust:latest

COPY --from=builder /usr/local/cargo/bin/myc-api /usr/local/bin/myc-api

COPY ports/api/src/openapi/redoc.config.json /home/redoc.config.json
COPY templates /home/templates

ENV UTOIPA_REDOC_CONFIG_FILE=/home/redoc.config.json

ARG TEMPLATES_DIR=/home/templates
ENV TEMPLATES_DIR=${TEMPLATES_DIR}

# Test if the templates directory exists
RUN if [ ! -d "${TEMPLATES_DIR}" ]; then \
    echo "Error: Templates directory not found at ${TEMPLATES_DIR}" && \
    exit 1; \
    fi

ARG SERVICE_PORT=8080
ENV SERVICE_PORT=${SERVICE_PORT}

EXPOSE ${SERVICE_PORT}

ENTRYPOINT ["myc-api"]
