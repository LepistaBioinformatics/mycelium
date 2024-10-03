// Only tenant managers and owners should create new subscription accounts to a
// given tenant. Then, the accounts with the above cited roles should be able to
// perform the following functions:
//
// - Delete subscription accounts.
// - Guest users to the management account;
// - Uninvite guest users from the management account;
//

mod delete_subscription_account;

pub use delete_subscription_account::*;
