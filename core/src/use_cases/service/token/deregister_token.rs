use crate::domain::{
    dtos::{role::RoleDTO, token::TokenDTO},
    entities::TokenDeregistration,
};

use clean_base::{
    entities::default_response::DeletionResponseKind,
    utils::errors::MappedErrors,
};

/// De-register token.
///
/// Remove a token from database. The requesting service argument should be used
/// to check if the requesting service that are trying to deregister the token
/// was the same which registered such token.
pub async fn deregister_token(
    token: TokenDTO,
    requesting_service: String,
    token_deregistration_repo: Box<&dyn TokenDeregistration>,
) -> Result<DeletionResponseKind<RoleDTO>, MappedErrors> {
    token_deregistration_repo
        .get_then_delete(token, requesting_service)
        .await
}
