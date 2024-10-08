// All actions listed below should ve performed by:
//
// - Subscription Manager
// - Tenant Manager
// - Tenant Owner
//

mod delete_tag;
mod register_tag;
mod update_tag;

pub use delete_tag::*;
pub use register_tag::*;
pub use update_tag::*;
