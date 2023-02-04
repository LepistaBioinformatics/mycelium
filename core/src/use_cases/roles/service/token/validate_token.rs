use crate::domain::{dtos::token::Token, entities::TokenDeregistration};

use clean_base::{
    entities::default_response::FetchResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Validate token.
///
/// Check if the token exists into database.
pub async fn validate_token(
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
