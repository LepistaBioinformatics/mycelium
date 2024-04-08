#!/usr/bin/bash

ARGS=("$@")

# Base packages
cargo publish -p mycelium-base $ARGS
cargo publish -p mycelium-config $ARGS
cargo publish -p myc-core $ARGS

# Adapters
cargo publish -p mycelium-memory-db $ARGS
cargo publish -p mycelium-service $ARGS
cargo publish -p mycelium-smtp $ARGS

# Ports and related
cargo publish -p mycelium-http-tools $ARGS
