#!/usr/bin/bash

RUST_LOG=debug SERVICE_PORT=8083 cargo run --package mycelium-api-test-svc --bin myc-api-test-svc
