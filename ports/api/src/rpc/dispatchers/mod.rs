pub(crate) mod account_manager;
pub(crate) mod audit;
pub(crate) mod beginners;
#[cfg(feature = "federation")]
pub(crate) mod beginners_federation;
#[cfg(feature = "federation")]
pub(crate) mod federation;
#[cfg(feature = "federation")]
mod federation_translations;
pub(crate) mod gateway_manager;
pub(crate) mod guest_manager;
pub(crate) mod managers;
#[cfg(feature = "federation")]
mod resolve_own_grants;
#[cfg(feature = "federation")]
mod revoke_identity_link;
pub(crate) mod service;
pub(crate) mod staff;
pub(crate) mod subscriptions_manager;
pub(crate) mod system_manager;
pub(crate) mod tenant_manager;
pub(crate) mod tenant_owner;
pub(crate) mod users_manager;

pub(crate) use account_manager::dispatch_account_manager;
pub(crate) use audit::dispatch_audit;
pub(crate) use beginners::dispatch_beginners;
#[cfg(feature = "federation")]
pub(crate) use federation::dispatch_federation;
pub(crate) use gateway_manager::dispatch_gateway_manager;
pub(crate) use guest_manager::dispatch_guest_manager;
pub(crate) use managers::dispatch_managers;
pub(crate) use service::dispatch_service;
pub(crate) use staff::dispatch_staff;
pub(crate) use subscriptions_manager::dispatch_subscriptions_manager;
pub(crate) use system_manager::dispatch_system_manager;
pub(crate) use tenant_manager::dispatch_tenant_manager;
pub(crate) use tenant_owner::dispatch_tenant_owner;
pub(crate) use users_manager::dispatch_users_manager;
