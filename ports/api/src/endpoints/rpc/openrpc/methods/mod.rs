mod account_manager;
mod beginners;
mod discovery;
mod gateway_manager;
mod guest_manager;
mod managers;

/// All methods in order: discovery, managers, accountManager, gatewayManager, guestManager, beginners.
pub fn all_methods() -> Vec<serde_json::Value> {
    let mut out = vec![discovery::method()];
    out.extend(managers::methods());
    out.extend(account_manager::methods());
    out.extend(gateway_manager::methods());
    out.extend(guest_manager::methods());
    out.extend(beginners::methods());
    out
}
