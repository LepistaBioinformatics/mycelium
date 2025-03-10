# ? ----------------------------------------------------------------------------
# ? Build stage/
# ? ----------------------------------------------------------------------------

FROM rust:latest AS builder

WORKDIR /rust

# ? The copy operations are performed in separate steps to allow caching layers
# ? over building operations
RUN cargo install mycelium-api --force

# ? ----------------------------------------------------------------------------
# ? Production stage
# ? ----------------------------------------------------------------------------

FROM rust:latest

COPY --from=builder /usr/local/cargo/bin/myc-api /usr/local/bin/myc-api

COPY ports/api/src/api_docs/redoc.config.json /home/redoc.config.json
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
