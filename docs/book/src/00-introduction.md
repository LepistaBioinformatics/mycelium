# Introduction

Welcome to **Mycelium API Gateway**, the ultimate solution for secure, flexible, and multi-tenant API management! Whether you're building a robust platform or enhancing your downstream APIs, our gateway is designed to provide you with everything you need for seamless authentication, authorization, and security.

## What is Mycelium?

Mycelium is an **open and free API Gateway**, designed to operate in modern, multi-tenant, and API-oriented environments. The project prioritizes architectural clarity, security, and extensibility, maintaining an explicit separation between technical concerns and organizational aspects of the project.

Mycelium acts as the entry layer for downstream services, being responsible for authentication, identity normalization, routing, and security policy enforcement. The gateway does not impose business logic, but provides **authorization primitives** that allow each service to evaluate permissions in an explicit, secure, and contextual manner.

## Key Features

### AI-aware API Gateway

- **Service Discovery**: Discover downstream APIs and their capabilities. Mycelium API Gateway is designed to be AI-aware, meaning it can understand the capabilities of the downstream APIs and use that information to route requests appropriately.
- **Full control of downstream APIs**: Downstream APIs can control whether their routes should be discovered or not.
- **Health Checks**: Downstream APIs can define health checks to indicate when they are ready to receive requests. Health status is automatically updated based on the health checks and informed during discovery.

### Authentication & Authorization

- **OAuth2**: Support for any OAuth2 identity provider with a few lines of configuration.
- **Two-Factor Authentication (2FA)**: Built-in support for TOTP to ensure an extra layer of security when users opt to use the internal authentication system.
- **Federated Identity Support**: Integrate with external identity providers while maintaining full control over roles and permissions.
- **Role-Based Access Control (RBAC)**: Define granular roles for both the gateway and downstream APIs using simple YAML configurations.

### Multi-Tenant Architecture

- **Tenant Management**: Create and manage tenants with subscription-based accounts.
- **Role Assignment**: Invite users to join tenants and assign them specific roles to streamline collaboration.

### Secure Secrets Management

- **Vault Integration**: Leverage HashiCorp Vault for secure storage of secrets.
- **Flexible Configurations**: Use secrets stored in Vault, environment variables, or define them in YAML.
- **Dynamic Secret Injection**: Automate secure secret delivery to downstream APIs.

### API Routing & Webhooks

- **Smart API Routing**: Easily configure API routes with support for secure token-based authentication.
- **Webhook Support**: Define webhooks with secrets for secure callbacks and notifications.

### TOML-Driven Configuration

- **Simple and Intuitive**: Manage all configurations (tenants, roles, permissions, routes, and security) with easy-to-read TOML files.
- **Environment Flexibility**: Combine TOML definitions with environment variables for maximum flexibility.

### Security-First Design

- **Downstream Security**: Automatically pass role-based security credentials to downstream APIs.
- **Token Management**: Store and securely pass tokens in request headers.
- **Compliance Ready**: Designed with modern security practices to meet enterprise compliance requirements.

## Why Choose Mycelium API Gateway?

1. **Community-Driven and Open Source**: Leverage a growing community while benefiting from an open-source model.
2. **Scalable and Modular**: Designed to grow with your needs, from startups to enterprise-scale applications.
3. **Developer-Friendly**: TOML-based configurations, secure secret management, and role-based policies make it easy to get started.

## Conceptual Structure

```
Client
  ↓
API Gateway (auth, routing, edge RBAC)
  ↓
Downstream Services (contextual FBAC)
```

This separation ensures low coupling, high expressiveness, and security decisions close to the resource.

## Next Steps

- [Installation Guide](./02-installation.md) - Learn how to install Mycelium
- [Quick Start](./03-quick-start.md) - Get up and running in minutes
- [Configuration](./04-configuration.md) - Understand configuration options
- [Authorization Model](./01-authorization.md) - Deep dive into the authorization system

## License

Mycelium API Gateway is licensed under the Apache 2.0 License. Additional restrictions for commercial use apply under the Commons Clause. See the [LICENSE](https://github.com/LepistaBioinformatics/mycelium/blob/main/LICENSE) file for details.
