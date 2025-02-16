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

enum UpdatePasswordResponse {
    UserNotFound,
    PasswordUpdated,
    SamePassword,
    UnableToValidatePassword,
}

use serde_json::to_value;
use UpdatePasswordResponse::*;

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        email::Email,
        native_error_codes::NativeErrorCodes,
        user::{MultiFactorAuthentication, PasswordHash, User},
    },
    entities::UserUpdating,
};
use mycelium_base::utils::errors::MappedErrors;
use mycelium_base::{
    dtos::Parent, entities::UpdatingResponseKind, utils::errors::updating_err,
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = UserUpdating)]
pub struct UserUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl UserUpdating for UserUpdatingSqlDbRepository {
    #[tracing::instrument(name = "update_user", skip_all)]
    async fn update(
        &self,
        user: User,
    ) -> Result<UpdatingResponseKind<User>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let user_id = user.id.ok_or_else(|| {
            updating_err("Unable to update user. Invalid record ID")
        })?;

        let updated = diesel::update(user_model::table.find(user_id))
            .set((
                user_model::username.eq(user.username),
                user_model::first_name.eq(user.first_name.unwrap()),
                user_model::last_name.eq(user.last_name.unwrap()),
                user_model::is_active.eq(user.is_active),
                user_model::updated.eq(Some(Local::now().naive_utc())),
            ))
            .returning(UserModel::as_returning())
            .get_result::<UserModel>(conn)
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    updating_err(format!("Invalid primary key: {:?}", user_id))
                } else {
                    updating_err(format!("Failed to update user: {}", e))
                }
            })?;

        Ok(UpdatingResponseKind::Updated(
            User::new(
                Some(updated.id),
                updated.username,
                Email::from_string(updated.email)?,
                Some(updated.first_name),
                Some(updated.last_name),
                updated.is_active,
                updated.created.and_local_timezone(Local).unwrap(),
                updated
                    .updated
                    .map(|dt| dt.and_local_timezone(Local).unwrap()),
                updated.account_id.map(|id| Parent::Id(id)),
                None,
            )
            .with_principal(updated.is_principal),
        ))
    }

    #[tracing::instrument(name = "update_password", skip_all)]
    async fn update_password(
        &self,
        user_id: Uuid,
        new_password: PasswordHash,
    ) -> Result<
        UpdatingResponseKind<(Option<NativeErrorCodes>, bool)>,
        MappedErrors,
    > {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let result = conn
            .transaction::<UpdatePasswordResponse, diesel::result::Error, _>(
                |conn| {
                    // Get current password
                    let provider = identity_provider_model::table
                        .filter(identity_provider_model::user_id.eq(user_id))
                        .select(IdentityProviderModel::as_select())
                        .first::<IdentityProviderModel>(conn)
                        .optional()?;

                    let provider = match provider {
                        None => {
                            return Ok(UserNotFound);
                        }
                        Some(p) => p,
                    };

                    let old_password = PasswordHash::new_from_hash(
                        provider
                            .password_hash
                            .expect("Password hash not found"),
                    );

                    // Check if new password is same as old
                    if let Some(new_raw_pass) = new_password.get_raw_password()
                    {
                        if old_password
                            .check_password(new_raw_pass.as_bytes())
                            .is_ok()
                        {
                            return Ok(SamePassword);
                        }
                    } else {
                        return Ok(UnableToValidatePassword);
                    }

                    // Update password
                    diesel::update(identity_provider_model::table)
                        .filter(identity_provider_model::user_id.eq(user_id))
                        .set(
                            identity_provider_model::password_hash
                                .eq(Some(new_password.hash)),
                        )
                        .execute(conn)?;

                    Ok(PasswordUpdated)
                },
            );

        match result {
            Ok(msg) => match msg {
                PasswordUpdated => {
                    Ok(UpdatingResponseKind::Updated((None, true)))
                }
                UserNotFound => Ok(UpdatingResponseKind::NotUpdated(
                    (Some(NativeErrorCodes::MYC00009), false),
                    "Unable to find target user".to_string(),
                )),
                SamePassword => Ok(UpdatingResponseKind::NotUpdated(
                    (Some(NativeErrorCodes::MYC00011), false),
                    "New Password is the same as the old one".to_string(),
                )),
                UnableToValidatePassword => {
                    Ok(UpdatingResponseKind::NotUpdated(
                        (Some(NativeErrorCodes::MYC00012), false),
                        "Unable to validate password".to_string(),
                    ))
                }
            },
            Err(e) => if e == diesel::result::Error::NotFound {
                updating_err(format!("Invalid user type: {:?}", user_id))
            } else {
                updating_err(format!("Failed to update password: {}", e))
            }
            .as_error(),
        }
    }

    #[tracing::instrument(name = "update_mfa", skip_all)]
    async fn update_mfa(
        &self,
        user_id: Uuid,
        mfa: MultiFactorAuthentication,
    ) -> Result<UpdatingResponseKind<bool>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        match diesel::update(user_model::table.find(user_id))
            .set(user_model::mfa.eq(Some(to_value(&mfa).unwrap())))
            .execute(conn)
        {
            Ok(_) => Ok(UpdatingResponseKind::Updated(true)),
            Err(e) => if e == diesel::result::Error::NotFound {
                updating_err(format!("Invalid user type: {:?}", user_id))
            } else {
                updating_err(format!("Failed to update MFA: {}", e))
            }
            .as_error(),
        }
    }
}
