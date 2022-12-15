use crate::domain::{
    dtos::account::AccountTypeDTO,
    entities::manager::account_type_registration::AccountTypeRegistration,
};
use agrobase::{
    entities::default_response::GetOrCreateResponseKind,
    utils::errors::MappedErrors,
};
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Clone, Debug, PartialEq)]
pub enum AccountTypeEnum {
    Standard,
    Manager,
    Staff,
    Subscription,
}

impl Display for AccountTypeEnum {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            AccountTypeEnum::Standard => write!(f, "Standard"),
            AccountTypeEnum::Manager => write!(f, "Manager"),
            AccountTypeEnum::Staff => write!(f, "Staff"),
            AccountTypeEnum::Subscription => write!(f, "Subscription"),
        }
    }
}

/// Get or create default accounts.
pub async fn get_or_create_default_account_types(
    account_type: AccountTypeEnum,
    name: Option<String>,
    description: Option<String>,
    account_type_registration: Box<&dyn AccountTypeRegistration>,
) -> Result<GetOrCreateResponseKind<AccountTypeDTO>, MappedErrors> {
    match account_type {
        AccountTypeEnum::Standard => {
            account_type_registration
                .get_or_create(AccountTypeDTO {
                    id: None,
                    name: name.unwrap_or(AccountTypeEnum::Standard.to_string()),
                    description: description.unwrap_or(
                        "Such users should request delegating access."
                            .to_string(),
                    ),
                    is_subscription: false,
                    is_manager: false,
                    is_staff: false,
                })
                .await
        }
        AccountTypeEnum::Manager => {
            account_type_registration
                .get_or_create(AccountTypeDTO {
                    id: None,
                    name: name.unwrap_or(AccountTypeEnum::Manager.to_string()),
                    description: description.unwrap_or(
                        "Such accounts should perform management action on the system."
                            .to_string(),
                    ),
                    is_subscription: false,
                    is_manager: true,
                    is_staff: false,
                })
                .await
        }
        AccountTypeEnum::Staff => {
            account_type_registration
                .get_or_create(AccountTypeDTO {
                    id: None,
                    name: name.unwrap_or(AccountTypeEnum::Staff.to_string()),
                    description: description.unwrap_or(
                            "Such accounts should perform maintenance action on the system."
                            .to_string(),
                    ),
                    is_subscription: false,
                    is_manager: false,
                    is_staff: true,
                })
                .await
        }
        AccountTypeEnum::Subscription => {
            account_type_registration
                .get_or_create(AccountTypeDTO {
                    id: None,
                    name: name
                        .unwrap_or(AccountTypeEnum::Subscription.to_string()),
                    description: description.unwrap_or(
                        "Such accounts are created to represents Customer results centering accounts."
                            .to_string(),
                    ),
                    is_subscription: true,
                    is_manager: false,
                    is_staff: false,
                })
                .await
        }
    }
}
