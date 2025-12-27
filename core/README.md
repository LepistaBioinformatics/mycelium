# MyceliumCore (also referred as MycCore): The core part of the Mycelium project

Here you can find business logic and domain types that are shared across the
workspace.

See the [Global README](../README.md) for more information.

## Directory Structure

This crate is organized into the following main directories:

### `domain/`

Contains the core domain models, data transfer objects (DTOs), entities, and utilities that represent the business domain of Mycelium.

- **`actors/`**: Defines system actors and roles used for authorization and access control (e.g., `SystemActor` enum with roles like `Beginner`, `SubscriptionsManager`, `AccountManager`, etc.)

- **`dtos/`**: Data Transfer Objects (DTOs) used for API communication and data serialization. Includes the following DTOs:
  - **`account/`**: Account-related DTOs including `Account`, `AccountMetaKey`, `VerboseStatus`, and `FlagResponse`
  - **`account_type.rs`**: Account type definitions
  - **`email.rs`**: Email DTO for email validation and handling
  - **`error_code.rs`**: Error code DTOs
  - **`guest_role.rs`**: Guest role DTOs with permissions
  - **`guest_user.rs`**: Guest user DTOs
  - **`health_check_info.rs`**: Health check information DTOs
  - **`http.rs`**: HTTP method and related DTOs
  - **`http_secret.rs`**: HTTP secret DTOs for secure communication
  - **`message.rs`**: Message DTOs
  - **`native_error_codes.rs`**: Native error code definitions
  - **`profile/`**: Profile-related DTOs including:
    - `Profile`: Main profile DTO with owners, accounts, and permissions
    - `LicensedResource` and `LicensedResources`: Licensed resource DTOs
    - `Owner`: Owner DTO
    - `TenantOwnership` and `TenantsOwnership`: Tenant ownership DTOs
  - **`related_accounts.rs`**: Related accounts DTOs
  - **`route.rs`**: Route configuration DTOs
  - **`security_group.rs`**: Security group DTOs
  - **`service.rs`**: Service DTOs
  - **`tag.rs`**: Tag DTOs for categorization
  - **`tenant/`**: Tenant-related DTOs including `Tenant`, `TenantMetaKey`, and `TenantStatus`
  - **`token/`**: Token-related DTOs organized in sub-modules:
    - **`connection_string/`**: Connection string DTOs including `ConnectionStringBeans`, `PublicConnectionStringInfo`, and `UserAccountConnectionString`
    - **`meta/`**: Token metadata DTOs including `AccountRelatedMeta` and `UserRelatedMeta`
    - **`token/`**: Token DTOs including `EmailConfirmationToken` and `PasswordChangeToken`
    - `Token` and `MultiTypeMeta`: Main token DTOs
  - **`user.rs`**: User DTOs
  - **`webhook/`**: Webhook-related DTOs including `WebHook`, `WebHookTrigger`, and webhook response DTOs
  - **`written_by.rs`**: Audit trail DTO for tracking who created/updated resources

- **`entities/`**: Domain entities representing the core business objects. Each entity module contains operations for:
  - Registration (creation)
  - Fetching (reading)
  - Updating
  - Deletion
  - Other domain-specific operations

  Entities include: `account`, `user`, `tenant`, `token`, `guest_role`, `guest_user`, `webhook`, `service`, `message`, `profile`, and more.

- **`utils/`**: Utility functions and helpers used across the domain layer (e.g., UUID derivation, type conversions)

### `use_cases/`

Contains the application use cases organized by functionality and role. These implement the business logic and orchestrate domain operations.

- **`gateway/`**: Use cases for gateway functionalities, including route configuration loading, validation, and request routing/matching

- **`role_scoped/`**: Federation use cases organized by user roles:
  - **`beginner/`**: Operations for users without assigned roles (account management, user registration, token management, tenant info)
  - **`account_manager/`**: Operations for managing a single subscription account
  - **`subscriptions_manager/`**: Operations for managing subscription accounts, guest users, roles, and tags
  - **`tenant_manager/`**: Operations for managing tenant-level resources (accounts, guests, tags, tenant details)
  - **`tenant_owner/`**: Operations for tenant owners to manage tenant metadata, tags, and ownership
  - **`users_manager/`**: Operations for managing user accounts
  - **`guest_manager/`**: Operations for managing guest roles and permissions
  - **`gateway_manager/`**: Operations for managing gateway endpoints and configurations
  - **`system_manager/`**: Operations for managing system-wide resources (error messages, webhooks, etc.)

- **`service/`**: Service-related use cases for automated account creation, guest-to-default account conversion, and service discovery

- **`super_users/`**: Use cases for system administrators:
  - **`managers/`**: Operations for managing tenants, system accounts, and system roles
  - **`staff/`**: Operations for staff account management and privilege upgrades/downgrades

- **`support/`**: Support use cases for notifications, webhook dispatching, and event registration

### `models/`

Configuration models and data structures used across the application:

- **`config.rs`**: Core configuration models
- **`account_life_cycle_config.rs`**: Configuration for account lifecycle management
- **`webhook_config.rs`**: Configuration for webhook dispatching

### `settings.rs`

System-wide settings, constants, and template configuration:
- Default TOTP domain configuration
- Template engine (Tera) initialization
- System constants and environment variable handling
