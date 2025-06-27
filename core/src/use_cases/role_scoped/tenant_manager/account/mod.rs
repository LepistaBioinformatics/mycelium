// Only tenant managers and owners should create new subscription accounts to a
// given tenant. Then, the accounts with the above cited roles should be able to
// perform the following functions:
//
// - Delete subscription accounts.
// - Create subscription manager accounts.
//

mod create_subscription_manager_account;
mod delete_subscription_account;

pub use create_subscription_manager_account::*;
pub use delete_subscription_account::*;
