# ? ----------------------------------------------------------------------------
# ? Build stage/
# ? ----------------------------------------------------------------------------

FROM rust:latest AS builder

WORKDIR /rust

# ? The copy operations are performed in separate steps to allow caching layers
# ? over building operations

ARG VERSION="latest"
ENV VERSION=${VERSION}

# ? If the VERSION is latest, instal using cargo install
# ? Otherwise, install using the --version flag
RUN if [ "${VERSION}" = "latest" ]; then \
        echo "Installing mycelium-api using cargo install"; \
        cargo install mycelium-api --force; \
        echo "mycelium-api installed successfully"; \
        echo "Version: $(myc-api --version)"; \
    else \
        echo "Cloning mycelium-api repository and building from source"; \
        cargo install mycelium-api --version ${VERSION} --force; \
        echo "mycelium-api installed successfully"; \
        echo "Version: $(myc-api --version)"; \
    fi

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
