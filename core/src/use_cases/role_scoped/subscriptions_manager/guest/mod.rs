// All actions listed below should ve performed by:
//
// - Subscription Manager
// - Tenant Manager
// - Tenant Owner
//

mod guest_user_to_subscription_account;
mod list_guest_on_subscription_account;
mod list_licensed_accounts_of_email;
mod register_deny_flag;
mod register_permit_flag;
mod revoke_deny_flat;
mod revoke_permit_flat;
mod revoke_user_guest_to_subscription_account;

pub use guest_user_to_subscription_account::*;
pub use list_guest_on_subscription_account::*;
pub use list_licensed_accounts_of_email::*;
pub use register_deny_flag::*;
pub use register_permit_flag::*;
pub use revoke_deny_flat::*;
pub use revoke_permit_flat::*;
pub use revoke_user_guest_to_subscription_account::*;
