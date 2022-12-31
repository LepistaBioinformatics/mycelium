use crate::domain::{dtos::token::TokenDTO, entities::TokenRegistration};

use chrono::{Duration, Local};
use clean_base::{
    entities::default_response::CreateResponseKind, utils::errors::MappedErrors,
};

/// Register a new token.
///
/// This function should be useful to send a new token to the data repository.
pub async fn register_token(
    own_service: String,
    token_registration_repo: Box<&dyn TokenRegistration>,
) -> Result<CreateResponseKind<TokenDTO>, MappedErrors> {
    token_registration_repo
        .create(
            TokenDTO::new_undated_token(
                own_service,
                Some(Local::now() + Duration::seconds(5)),
            )
            .await,
        )
        .await
}
