mod account_manager;
mod beginners;
mod discovery;
mod managers;

/// All methods in order: discovery, managers, accountManager, beginners.
pub fn all_methods() -> Vec<serde_json::Value> {
    let mut out = vec![discovery::method()];
    out.extend(managers::methods());
    out.extend(account_manager::methods());
    out.extend(beginners::methods());
    out
}
