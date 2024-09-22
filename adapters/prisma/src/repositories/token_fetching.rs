use async_trait::async_trait;
use myc_core::domain::{
    dtos::token::{EmailConfirmationTokenMeta, PasswordChangeTokenMeta},
    entities::TokenFetching,
};
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Component;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TokenFetching)]
pub struct TokenFetchingSqlDbRepository {}

#[async_trait]
impl TokenFetching for TokenFetchingSqlDbRepository {
    async fn get_and_invalidate_email_confirmation_token(
        &self,
        _: EmailConfirmationTokenMeta,
    ) -> Result<FetchResponseKind<Uuid, String>, MappedErrors> {
        unimplemented!(
            "TokenFetching::get_and_invalidate_email_confirmation_token not implemented"
        )
    }

    async fn get_and_invalidate_password_change_token(
        &self,
        _: PasswordChangeTokenMeta,
    ) -> Result<FetchResponseKind<Uuid, String>, MappedErrors> {
        unimplemented!(
            "TokenFetching::get_and_invalidate_password_change_token not implemented"
        )
    }
}
