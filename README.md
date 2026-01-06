# 🚀 Mycelium: The Ultimate API Gateway

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/mycelium-api.svg)](https://crates.io/crates/mycelium-api)
[![Downloads](https://img.shields.io/crates/d/mycelium-api.svg)](https://crates.io/crates/mycelium-api)
[![Docs.rs](https://docs.rs/mycelium-api/badge.svg)](https://docs.rs/mycelium-api)
[![CI](https://github.com/LepistaBioinformatics/mycelium/actions/workflows/ci.yml/badge.svg)](https://github.com/LepistaBioinformatics/mycelium/actions/workflows/ci.yml)
[![Security](https://github.com/LepistaBioinformatics/mycelium/actions/workflows/security.yml/badge.svg)](https://github.com/LepistaBioinformatics/mycelium/actions/workflows/security.yml)
[![Docker Pulls](https://img.shields.io/docker/pulls/sgelias/mycelium-api)](https://hub.docker.com/r/sgelias/mycelium-api)
[![License](https://img.shields.io/crates/l/mycelium-api.svg)](./LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/10181/badge?t=2)](https://www.bestpractices.dev/en/projects/10181)

<img
  alt="Mycelium Logo"
  src="docs/assets/logo-large.svg"
  width="200"
  style="margin: 20px; background-color: transparent; border-radius: 20px;"
/>

</div>

Mycelium is an **open and free API Gateway**, designed to operate in
modern, multi-tenant, and API-oriented environments. The project prioritizes
architectural clarity, security, and extensibility, maintaining an explicit
separation between technical concerns and organizational aspects of the project.

This repository documents the fundamentals of Mycelium, with special focus on its
**authorization model**, which combines declarative controls at the gateway with
contextual decisions close to the resource.

---

## 📖 Documentation

**[📚 View Complete Documentation →](https://lepistabioinformatics.github.io/mycelium/)**

Access the full documentation website with guides, tutorials, and API reference.

---

## Quick Links

📚 **[Complete Documentation](./docs/book/src/00-introduction.md)** - Full documentation guide

🚀 **[Installation Guide](./docs/book/src/02-installation.md)** - Get started with installation

⚡ **[Quick Start](./docs/book/src/03-quick-start.md)** - Up and running in minutes

⚙️ **[Configuration](./docs/book/src/04-configuration.md)** - Configure Mycelium

🔐 **[Authorization Model](./docs/book/src/01-authorization.md)** - Deep dive into security

---

## Overview

Mycelium acts as the entry layer for downstream services, being responsible for
authentication, identity normalization, routing, and security policy
enforcement. The gateway does not impose business logic, but provides
**authorization primitives** that allow each service to evaluate permissions in
an explicit, secure, and contextual manner.

The project is maintained as open source software, with its continuity based on
governance, community collaboration, and ecosystem funding and acceleration
initiatives — aspects that are **independent of internal technical decisions**.

---

## Key Features

* Modern and extensible API Gateway
* Native support for multi-tenant environments
* Authorization at multiple layers (gateway and downstream)
* Identity context injection via Profile
* Composable authorization primitives
* Architecture compatible with market standards

---

## Authorization Model

Mycelium's authorization model is one of its central pillars and is documented
in detail in the file:

👉 **[Authorization](./docs/book/src/01-authorization.md)**

In summary:

* The gateway applies **declarative controls per route** (coarse-grained)
* Downstream services apply **contextual authorizations** (fine-grained)
* The Profile acts as an active capability object, not just as an identity
  payload

---

## Conceptual Structure

<img
  alt="Conceptual Structure"
  src="docs/draw.io/authentication-authorization-model.drawio.png"
  width="100%"
  style="background-color: transparent; border-radius: 10px; align-items: center; display: block; margin: 0 auto;"
/>

```
Client
  ↓
API Gateway (auth, routing, edge RBAC)
  ↓
Downstream Services (contextual FBAC)
```

This separation ensures low coupling, high expressiveness, and security
decisions close to the resource.

---

## Project Governance and Sustainability

Mycelium is an open and free project. Its maintenance and evolution are handled
within the project's organizational scope, through:

* Community collaboration
* Institutional support
* Funding and acceleration initiatives

These aspects **do not influence or condition** the technical authorization
model, which remains neutral, explicit, and verifiable.

---

## Getting Started

### Prerequisites

Before installing Mycelium, ensure you have:

- **Rust** (version 1.70 or higher) - [Install Rust](https://rustup.rs/)
- **Postgres** (version 14 or higher) - Database for tenant and user management
- **Redis** (version 6 or higher) - Caching layer
- **HashiCorp Vault** (optional) - Recommended for production secret management
- **Docker** (optional) - For containerized deployment

For detailed system dependencies and installation instructions, see the [Installation Guide](./docs/book/src/02-installation.md).

### Installation

Install Mycelium using Cargo:

```bash
cargo install mycelium-api
```

Or using Docker:

```bash
docker pull sgelias/mycelium-api:latest
```

For complete installation instructions including database setup and Vault configuration, see the [Installation Guide](./docs/book/src/02-installation.md).

### Quick Start

1. **Initialize the database:**
   ```bash
   psql postgres://postgres:postgres@localhost:5432/postgres \
     -f postgres/sql/up.sql \
     -v db_password='your-password'
   ```

2. **Configure Mycelium:**
   ```bash
   cp settings/config.example.toml settings/config.toml
   # Edit config.toml with your settings
   ```

3. **Start Mycelium:**
   ```bash
   SETTINGS_PATH=settings/config.toml myc-api
   ```

4. **Verify it's running:**
   ```bash
   curl http://localhost:8080/health
   ```

For a complete quick start guide with minimal configuration, see [Quick Start Guide](./docs/book/src/03-quick-start.md).

### Running Tests

Execute the test suite:

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Run specific tests
cargo test auth
```

For detailed testing instructions including integration tests and benchmarks, see [Running Tests](./docs/book/src/07-running-tests.md).

---

## Documentation

**📚 [Online Documentation](https://lepistabioinformatics.github.io/mycelium/)** - Browse the complete documentation website

Alternatively, you can access documentation files directly in the `docs/book/src/` directory:

- **[Introduction](./docs/book/src/00-introduction.md)** - Overview and key features
- **[Installation](./docs/book/src/02-installation.md)** - Installation and setup
- **[Quick Start](./docs/book/src/03-quick-start.md)** - Get started quickly
- **[Configuration](./docs/book/src/04-configuration.md)** - Configuration options
- **[Deploy Locally](./docs/book/src/05-deploy-locally.md)** - Local deployment with Docker
- **[Authorization Model](./docs/book/src/01-authorization.md)** - Security and authorization
- **[Downstream APIs](./docs/book/src/06-downstream-apis.md)** - Configure routes and services
- **[Running Tests](./docs/book/src/07-running-tests.md)** - Testing guide

---

## Project Status

Mycelium is under active development and open to contributions. Architectural
discussions, improvement proposals, and conceptual reviews are welcome.

---

## License

See the [LICENSE](./LICENSE) file for details about the project's licensing.
