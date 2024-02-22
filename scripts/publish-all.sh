#!/usr/bin/bash

# Base packages
cargo publish -p mycelium-base
cargo publish -p mycelium-config
cargo publish -p myc-core

# Adapters
cargo publish -p mycelium-memory-db
cargo publish -p mycelium-service
cargo publish -p mycelium-smtp

# Ports and related
cargo publish -p mycelium-http-tools
