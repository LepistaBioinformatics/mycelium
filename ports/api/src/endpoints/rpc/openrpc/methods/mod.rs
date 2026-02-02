//! OpenRPC method descriptors by scope: discovery, managers, beginners.

mod beginners;
mod discovery;
mod managers;

/// All methods in order: discovery, then managers, then beginners.
pub fn all_methods() -> Vec<serde_json::Value> {
    let mut out = vec![discovery::method()];
    out.extend(managers::methods());
    out.extend(beginners::methods());
    out
}
