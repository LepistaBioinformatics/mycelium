mod account_manager;
mod beginners;
mod discovery;
mod gateway_manager;
mod guest_manager;
mod managers;
mod service;
mod staff;
mod subscriptions_manager;
mod system_manager;
mod tenant_manager;
mod tenant_owner;
mod users_manager;

/// All methods in order: discovery, managers, accountManager, gatewayManager,
/// guestManager, subscriptionsManager, systemManager, tenantManager,
/// tenantOwner, userManager, service, staff, beginners.
pub fn all_methods() -> Vec<serde_json::Value> {
    let mut out = vec![discovery::method()];
    out.extend(managers::methods());
    out.extend(account_manager::methods());
    out.extend(gateway_manager::methods());
    out.extend(guest_manager::methods());
    out.extend(subscriptions_manager::methods());
    out.extend(system_manager::methods());
    out.extend(tenant_manager::methods());
    out.extend(tenant_owner::methods());
    out.extend(users_manager::methods());
    out.extend(service::methods());
    out.extend(staff::methods());
    out.extend(beginners::methods());
    out
}
