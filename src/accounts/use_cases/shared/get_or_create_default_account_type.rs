use crate::domain::{
    dtos::account::AccountTypeDTO,
    entities::{
        manager::account_type_registration::AccountTypeRegistration,
        shared::default_responses::GetOrCreateResponse,
    },
    utils::errors::MappedErrors,
};

/// This function are called when any user (admins users only) starts a new
/// account. Thus, a default account type are created if not exists.
pub async fn get_or_create_default_account_type(
    name: Option<String>,
    description: Option<String>,
    account_type_registration: Box<&dyn AccountTypeRegistration>,
) -> Result<GetOrCreateResponse<AccountTypeDTO>, MappedErrors> {
    account_type_registration
        .get_or_create(AccountTypeDTO {
            id: None,
            name: name.unwrap_or("Own Account Admin".to_string()),
            description: description.unwrap_or(
                "A default user of the application. Such user has full access to this own account."
                .to_string()
            ),
            is_manager: false,
            is_staff: false,
        })
        .await
}
