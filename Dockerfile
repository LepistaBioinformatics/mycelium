# ? ----------------------------------------------------------------------------
# ? Build stage/
# ? ----------------------------------------------------------------------------

FROM rust:latest AS builder

WORKDIR /rust

# ? The copy operations are performed in sepparate steps to allow caching layers
# ? over building operations
RUN cargo install mycelium-api

# ? ----------------------------------------------------------------------------
# ? Production stage
# ? ----------------------------------------------------------------------------

FROM rust:latest

# RUN apt-get update && apt-get install -y libpq-dev

COPY --from=builder /usr/local/cargo/bin/myc-api /usr/local/bin/myc-api

ARG SERVICE_PORT=8080
ENV SERVICE_PORT=${SERVICE_PORT}

EXPOSE ${SERVICE_PORT}

ENTRYPOINT ["myc-api"]
