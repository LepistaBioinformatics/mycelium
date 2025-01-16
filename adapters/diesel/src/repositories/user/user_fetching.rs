use crate::{
    models::{
        config::DbPoolProvider,
        identity_provider::IdentityProvider as IdentityProviderModel,
        user::User as UserModel,
    },
    schema::{
        identity_provider as identity_provider_model, user as user_model,
    },
};

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        email::Email,
        native_error_codes::NativeErrorCodes,
        user::{MultiFactorAuthentication, PasswordHash, Provider, User},
    },
    entities::UserFetching,
};
use mycelium_base::{
    dtos::Parent,
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use serde_json::from_value;
use shaku::Component;
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = UserFetching)]
pub struct UserFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl UserFetching for UserFetchingSqlDbRepository {
    async fn get_user_by_email(
        &self,
        email: Email,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let result = user_model::table
            .filter(user_model::email.eq(email.email()))
            .inner_join(identity_provider_model::table)
            .select((
                UserModel::as_select(),
                IdentityProviderModel::as_select(),
            ))
            .first::<(UserModel, IdentityProviderModel)>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch user: {}", e))
            })?;

        match result {
            None => Ok(FetchResponseKind::NotFound(None)),
            Some((user_record, provider_record)) => {
                let provider = if let Some(password_hash) =
                    provider_record.password_hash
                {
                    Provider::Internal(PasswordHash::new_from_hash(
                        password_hash,
                    ))
                } else if let Some(name) = provider_record.name {
                    Provider::External(name)
                } else {
                    return fetching_err(
                        "User has invalid provider configuration",
                    )
                    .as_error();
                };

                let mut user = User::new(
                    Some(user_record.id),
                    user_record.username,
                    Email::from_string(user_record.email)?,
                    Some(user_record.first_name),
                    Some(user_record.last_name),
                    user_record.is_active,
                    user_record.created.and_local_timezone(Local).unwrap(),
                    user_record
                        .updated
                        .map(|dt| dt.and_local_timezone(Local).unwrap()),
                    user_record.account_id.map(Parent::Id),
                    Some(provider),
                )
                .with_principal(user_record.is_principal);

                if let Some(mfa) = user_record.mfa {
                    match from_value::<MultiFactorAuthentication>(mfa) {
                        Ok(mut mfa) => {
                            mfa.redact_secrets();
                            user = user.with_mfa(mfa);
                        }
                        Err(err) => {
                            error!("Failed to parse MFA data: {}", err);
                            return fetching_err("Failed to parse MFA data")
                                .as_error();
                        }
                    }
                }

                Ok(FetchResponseKind::Found(user))
            }
        }
    }

    async fn get_user_by_id(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let result = user_model::table
            .find(id)
            .inner_join(identity_provider_model::table)
            .select((
                UserModel::as_select(),
                IdentityProviderModel::as_select(),
            ))
            .first::<(UserModel, IdentityProviderModel)>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch user: {}", e))
            })?;

        match result {
            None => Ok(FetchResponseKind::NotFound(None)),
            Some((user_record, provider_record)) => {
                let provider = if let Some(password_hash) =
                    provider_record.password_hash
                {
                    Provider::Internal(PasswordHash::new_from_hash(
                        password_hash,
                    ))
                } else if let Some(name) = provider_record.name {
                    Provider::External(name)
                } else {
                    return fetching_err(
                        "User has invalid provider configuration",
                    )
                    .as_error();
                };

                let mut user = User::new(
                    Some(user_record.id),
                    user_record.username,
                    Email::from_string(user_record.email)?,
                    Some(user_record.first_name),
                    Some(user_record.last_name),
                    user_record.is_active,
                    user_record.created.and_local_timezone(Local).unwrap(),
                    user_record
                        .updated
                        .map(|dt| dt.and_local_timezone(Local).unwrap()),
                    user_record.account_id.map(Parent::Id),
                    Some(provider),
                )
                .with_principal(user_record.is_principal);

                if let Some(mfa) = user_record.mfa {
                    match from_value::<MultiFactorAuthentication>(mfa) {
                        Ok(mut mfa) => {
                            mfa.redact_secrets();
                            user = user.with_mfa(mfa);
                        }
                        Err(err) => {
                            error!("Failed to parse MFA data: {}", err);
                            return fetching_err("Failed to parse MFA data")
                                .as_error();
                        }
                    }
                }

                Ok(FetchResponseKind::Found(user))
            }
        }
    }

    async fn get_not_redacted_user_by_email(
        &self,
        email: Email,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let result = user_model::table
            .filter(user_model::email.eq(email.email()))
            .inner_join(identity_provider_model::table)
            .select((
                UserModel::as_select(),
                IdentityProviderModel::as_select(),
            ))
            .first::<(UserModel, IdentityProviderModel)>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch user: {}", e))
            })?;

        match result {
            None => Ok(FetchResponseKind::NotFound(None)),
            Some((user_record, provider_record)) => {
                let provider = if let Some(password_hash) =
                    provider_record.password_hash
                {
                    Provider::Internal(PasswordHash::new_from_hash(
                        password_hash,
                    ))
                } else if let Some(name) = provider_record.name {
                    Provider::External(name)
                } else {
                    return fetching_err(
                        "User has invalid provider configuration",
                    )
                    .as_error();
                };

                let mut user = User::new(
                    Some(user_record.id),
                    user_record.username,
                    Email::from_string(user_record.email)?,
                    Some(user_record.first_name),
                    Some(user_record.last_name),
                    user_record.is_active,
                    user_record.created.and_local_timezone(Local).unwrap(),
                    user_record
                        .updated
                        .map(|dt| dt.and_local_timezone(Local).unwrap()),
                    user_record.account_id.map(Parent::Id),
                    Some(provider),
                )
                .with_principal(user_record.is_principal);

                if let Some(mfa) = user_record.mfa {
                    let mfa = match from_value(mfa) {
                        Ok(mfa) => mfa,
                        Err(err) => {
                            error!("Failed to parse MFA data: {}", err);
                            return fetching_err("Failed to parse MFA data")
                                .as_error();
                        }
                    };
                    user = user.with_mfa(mfa);
                }

                Ok(FetchResponseKind::Found(user))
            }
        }
    }
}
