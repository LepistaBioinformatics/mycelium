use crate::domain::dtos::account::AccountType;

/// Try to create or fetch a default account.
///
/// This method are called when a new user start into the system. This method
/// creates a new account flagged as default based on the given account type.
/// Different account types should be connected with different default accounts.
///
/// Default accounts given specific accesses to the user. For example, a default
/// user should be able to view example data. Staff user should be able to
/// create new users and so on.
pub async fn get_or_create_default_account(account_type: AccountType) {}
