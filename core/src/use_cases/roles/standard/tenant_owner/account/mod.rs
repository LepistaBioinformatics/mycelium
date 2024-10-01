// Only account owners should create management accounts. Management accounts
// should perform the following functions:
//
// - Create management accounts;
// - Update management accounts;
// - Delete management accounts.
//

mod create_management_account;
mod delete_management_account;
mod update_management_account;

pub use create_management_account::*;
pub use delete_management_account::*;
pub use update_management_account::*;
