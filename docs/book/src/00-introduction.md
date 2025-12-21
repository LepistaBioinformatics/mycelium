# Introduction

Welcome to **Mycelium API Gateway**, the ultimate solution for secure, flexible, and multi-tenant API management! Whether you're building a robust platform or enhancing your downstream APIs, our gateway is designed to provide you with everything you need for seamless authentication, authorization, and security.

## What is Mycelium?

Mycelium is an **open and free API Gateway**, designed to operate in modern, multi-tenant, and API-oriented environments. The project prioritizes architectural clarity, security, and extensibility, maintaining an explicit separation between technical concerns and organizational aspects of the project.

Mycelium acts as the entry layer for downstream services, being responsible for authentication, identity normalization, routing, and security policy enforcement. The gateway does not impose business logic, but provides **authorization primitives** that allow each service to evaluate permissions in an explicit, secure, and contextual manner.

## Key Features

### Authentication & Authorization

- **OAuth2**: Support for any OAuth2 identity provider with a few lines of configuration.
- **Two-Factor Authentication (2FA)**: Built-in support for TOTP to ensure an extra layer of security when users opt to use the internal authentication system.
- **Federated Identity Support**: Integrate with external identity providers while maintaining full control over roles and permissions.
- **Contextual Authorization (FBAC)**: Fine-grained, Feature-based Access Control with contextual evaluation at both gateway (RBAC for edge control) and downstream services (contextual FBAC for resource-level decisions).

### Multi-Tenant Architecture

- **Tenant Management**: Create and manage tenants with subscription-based accounts.
- **Role Assignment**: Invite users to join tenants and assign them specific roles to streamline collaboration.

### Secure Secrets Management

- **Vault Integration**: Leverage HashiCorp Vault for secure storage of secrets.
- **Flexible Configurations**: Use secrets stored in Vault, environment variables, or define them in YAML.
- **Dynamic Secret Injection**: Automate secure secret delivery to downstream APIs.

### API Routing & Service Discovery

- **Service Discovery**: Discover downstream APIs and their capabilities, allowing dynamic integration and routing based on available services.
- **Full Control of Downstream APIs**: Downstream APIs can control whether their routes should be discovered or not, maintaining granular visibility control.
- **Health Checks**: Downstream APIs can define health checks to indicate when they are ready to receive requests. Health status is automatically updated based on the health checks and informed during discovery.
- **Smart API Routing**: Easily configure API routes with support for secure token-based authentication.
- **Webhook Support**: Define webhooks with secrets for secure callbacks and notifications.

### TOML-Driven Configuration

- **Simple and Intuitive**: Manage all configurations (tenants, roles, permissions, routes, and security) with easy-to-read TOML files.
- **Environment Flexibility**: Combine TOML definitions with environment variables for maximum flexibility.

### Security-First Design

- **Layered Authorization**: Gateway applies declarative controls (RBAC) while downstream services use contextual FBAC for fine-grained decisions.
- **Profile Injection**: Automatically inject identity context and capabilities to downstream APIs via Profile objects.
- **Token Management**: Store and securely pass tokens in request headers.
- **Compliance Ready**: Designed with modern security practices to meet enterprise compliance requirements.

## Why Choose Mycelium API Gateway?

1. **Community-Driven and Open Source**: Leverage a growing community while benefiting from an open-source model.
2. **Scalable and Modular**: Designed to grow with your needs, from startups to enterprise-scale applications.
3. **Developer-Friendly**: TOML-based configurations, secure secret management, and contextual authorization model make it easy to get started.
4. **Modern Authorization**: Combines declarative RBAC at the gateway with fine-grained contextual FBAC in downstream services for maximum flexibility and security.

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
