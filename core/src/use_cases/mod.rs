/// Gateway use cases
///
/// This module contains the use cases for the gateway functionalities like load
/// and validate route configurations and match incoming requests to the
/// appropriate route.
///
pub mod gateway;

/// Federation use cases
///
/// This module contains the use cases for the federation functionalities like
/// manage accounts, tenants, roles, permissions, webhooks, and other.
///
/// Current roles are:
/// - Account Manager
/// - Beginners
/// - Gateway Manager
/// - Subscriptions Manager
/// - System Manager
/// - Tenant Manager
/// - Tenant Owner
/// - User Manager
///
pub mod role_scoped;

/// Service use cases
///
/// This module contains the use cases for the service functionalities like
/// automated creation of accounts, guest to default accounts and other.
///
pub mod service;

/// Super Users use cases
///
/// Use cases related to staff and managers of the system. It includes the
/// management of tenants, system initialization and super users management.
///
pub mod super_users;

/// Support use cases
///
/// This module contains the use cases for the support crate-related
/// functionalities.
///
pub(crate) mod support;
pub use support::dispatch_webhooks;
