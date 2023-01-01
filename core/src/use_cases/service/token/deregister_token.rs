use crate::domain::{dtos::token::Token, entities::TokenDeregistration};

use clean_base::{
    entities::default_response::FetchResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// De-register token.
///
/// Remove a token from database. The requesting service argument should be used
/// to check if the requesting service that are trying to deregister the token
/// was the same which registered such token.
pub async fn deregister_token(
    token: Uuid,
    requesting_service: String,
    token_deregistration_repo: Box<&dyn TokenDeregistration>,
) -> Result<FetchResponseKind<Token, Uuid>, MappedErrors> {
    token_deregistration_repo
        .get_then_delete(Token {
            token,
            own_service: requesting_service,
        })
        .await
}
