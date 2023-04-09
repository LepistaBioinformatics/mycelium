use crate::domain::{
    dtos::account::{AccountType, AccountTypeEnum},
    entities::AccountTypeRegistration,
};
use clean_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};

/// Get or create default accounts.
///
/// This use-case should only be executed by another use-cases, thus, could not
/// be exposed through system ports.
pub(crate) async fn get_or_create_default_account_types(
    account_type: AccountTypeEnum,
    name: Option<String>,
    description: Option<String>,
    account_type_registration: Box<&dyn AccountTypeRegistration>,
) -> Result<GetOrCreateResponseKind<AccountType>, MappedErrors> {
    match account_type {
        AccountTypeEnum::Standard => {
            account_type_registration
                .get_or_create(AccountType {
                    id: None,
                    name: name.unwrap_or(AccountTypeEnum::Standard.to_string()),
                    description: description.unwrap_or(
                        "These accounts should request delegating access"
                            .to_string(),
                    ),
                    is_subscription: false,
                    is_manager: false,
                    is_staff: false,
                })
                .await
        }
        AccountTypeEnum::Manager => account_type_registration
            .get_or_create(AccountType {
                id: None,
                name: name.unwrap_or(AccountTypeEnum::Manager.to_string()),
                description: description.unwrap_or(
                    "These accounts should perform system management action"
                        .to_string(),
                ),
                is_subscription: false,
                is_manager: true,
                is_staff: false,
            })
            .await,
        AccountTypeEnum::Staff => account_type_registration
            .get_or_create(AccountType {
                id: None,
                name: name.unwrap_or(AccountTypeEnum::Staff.to_string()),
                description: description.unwrap_or(
                    "These accounts should perform system maintenance action"
                        .to_string(),
                ),
                is_subscription: false,
                is_manager: true,
                is_staff: true,
            })
            .await,
        AccountTypeEnum::Subscription => {
            account_type_registration
                .get_or_create(AccountType {
                    id: None,
                    name: name
                        .unwrap_or(AccountTypeEnum::Subscription.to_string()),
                    description: description.unwrap_or(
                        "These accounts represent legal entities.".to_string(),
                    ),
                    is_subscription: true,
                    is_manager: false,
                    is_staff: false,
                })
                .await
        }
    }
}
