use super::map_user_row_to_dto;
use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::{
        identity_provider::IdentityProvider as IdentityProviderModel,
        user::User as UserModel,
    },
    schema::{identity_provider, user},
    types::uuid_to_text,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{email::Email, native_error_codes::NativeErrorCodes, user::User},
    entities::UserFetching,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = UserFetching)]
pub struct UserFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl UserFetching for UserFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_user_by_email", skip_all)]
    async fn get_user_by_email(
        &self,
        email: Email,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let result = self.find_by_email(conn, &email)?;

        let Some((user_record, provider_record)) = result else {
            return Ok(FetchResponseKind::NotFound(None));
        };

        Ok(FetchResponseKind::Found(map_user_row_to_dto(
            user_record,
            provider_record,
            true,
        )?))
    }

    #[tracing::instrument(name = "get_user_by_id", skip_all)]
    async fn get_user_by_id(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let result = user::table
            .find(uuid_to_text(&id))
            .inner_join(identity_provider::table)
            .select((
                UserModel::as_select(),
                IdentityProviderModel::as_select(),
            ))
            .first::<(UserModel, IdentityProviderModel)>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch user: {}", e))
            })?;

        let Some((user_record, provider_record)) = result else {
            return Ok(FetchResponseKind::NotFound(None));
        };

        Ok(FetchResponseKind::Found(map_user_row_to_dto(
            user_record,
            provider_record,
            true,
        )?))
    }

    #[tracing::instrument(name = "get_not_redacted_user_by_email", skip_all)]
    async fn get_not_redacted_user_by_email(
        &self,
        email: Email,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let result = self.find_by_email(conn, &email)?;

        let Some((user_record, provider_record)) = result else {
            return Ok(FetchResponseKind::NotFound(None));
        };

        Ok(FetchResponseKind::Found(map_user_row_to_dto(
            user_record,
            provider_record,
            false,
        )?))
    }
}

impl UserFetchingSqlDbRepository {
    fn find_by_email(
        &self,
        conn: &mut diesel::SqliteConnection,
        email: &Email,
    ) -> Result<Option<(UserModel, IdentityProviderModel)>, MappedErrors> {
        user::table
            .filter(user::email.eq(email.email()))
            .inner_join(identity_provider::table)
            .select((
                UserModel::as_select(),
                IdentityProviderModel::as_select(),
            ))
            .first::<(UserModel, IdentityProviderModel)>(conn)
            .optional()
            .map_err(|e| fetching_err(format!("Failed to fetch user: {}", e)))
    }
}
