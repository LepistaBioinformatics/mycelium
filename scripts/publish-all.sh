#!/usr/bin/bash

ARGS=("$@")

# Base packages
cargo publish -p mycelium-base $ARGS
cargo publish -p mycelium-config $ARGS
cargo publish -p myc-core $ARGS

# Adapters
cargo publish -p mycelium-diesel $ARGS
cargo publish -p mycelium-memory-db $ARGS
cargo publish -p mycelium-service $ARGS
cargo publish -p mycelium-notifier $ARGS

# Ports and related
cargo publish -p mycelium-http-tools $ARGS
cargo publish -p mycelium-cli $ARGS
cargo publish -p mycelium-api $ARGS
