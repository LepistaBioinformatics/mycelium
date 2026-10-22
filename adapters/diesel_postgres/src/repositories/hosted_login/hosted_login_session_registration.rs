use super::hosted_login_session_record::{parse_session, serialize_session};
use crate::{
    models::{config::DbPoolProvider, token::Token as TokenModel},
    schema::token as token_model,
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        hosted_login::HostedLoginSession, native_error_codes::NativeErrorCodes,
    },
    entities::HostedLoginSessionRegistration,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = HostedLoginSessionRegistration)]
pub struct HostedLoginSessionRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl HostedLoginSessionRegistration
    for HostedLoginSessionRegistrationSqlDbRepository
{
    #[tracing::instrument(name = "create_hosted_login_session", skip_all)]
    async fn create(
        &self,
        session: HostedLoginSession,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<HostedLoginSession>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let record = serialize_session(&session)
            .and_then(|meta| {
                diesel::insert_into(token_model::table)
                    .values((
                        token_model::meta.eq(meta),
                        token_model::expiration.eq(expires.naive_utc()),
                    ))
                    .returning(TokenModel::as_returning())
                    .get_result::<TokenModel>(conn)
            })
            .map_err(|e| {
                creation_err(format!(
                    "Unexpected error on create hosted login session: {}",
                    e
                ))
            })?;

        let session = parse_session(&record).map_err(|e| {
            creation_err(format!("Invalid hosted login session: {}", e))
        })?;

        Ok(CreateResponseKind::Created(session))
    }
}
