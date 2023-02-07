#!/usr/bin/bash

RUST_LOG=debug SERVICE_PORT=8081 cargo run --package mycelium-api-test-svc --bin myc-api-test-svc &
RUST_LOG=debug SERVICE_PORT=8082 cargo run --package mycelium-api-test-svc --bin myc-api-test-svc &
