use crate::{
    config::SqliteDbPoolProvider,
    models::{
        identity_provider::IdentityProvider as IdentityProviderModel,
        user::User as UserModel,
    },
    repositories::account::created_at_from_text,
    schema::{identity_provider, user},
    types::{naive_timestamp_to_text, uuid_from_text, uuid_to_text},
};

enum UpdatePasswordResponse {
    UserNotFound,
    PasswordUpdated,
    SamePassword,
    UnableToValidatePassword,
}

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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl UserUpdating for UserUpdatingSqlDbRepository {
    #[tracing::instrument(name = "update_user", skip_all)]
    async fn update(
        &self,
        user_dto: User,
    ) -> Result<UpdatingResponseKind<User>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let user_id = user_dto.id.ok_or_else(|| {
            updating_err("Unable to update user. Invalid record ID")
        })?;

        let is_principal = user_dto.is_principal();
        let updated = diesel::update(user::table.find(uuid_to_text(&user_id)))
            .set((
                user::username.eq(user_dto.username),
                user::first_name.eq(user_dto.first_name.unwrap()),
                user::last_name.eq(user_dto.last_name.unwrap()),
                user::is_active.eq(user_dto.is_active),
                user::is_principal.eq(is_principal),
                user::updated.eq(Some(naive_timestamp_to_text(
                    &Local::now().naive_utc(),
                ))),
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
                Some(uuid_from_text(&updated.id).unwrap()),
                updated.username,
                Email::from_string(updated.email)?,
                Some(updated.first_name),
                Some(updated.last_name),
                updated.is_active,
                created_at_from_text(&updated.created),
                updated.updated.map(|dt| created_at_from_text(&dt)),
                updated
                    .account_id
                    .map(|id| Parent::Id(uuid_from_text(&id).unwrap())),
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

        let user_id_text = uuid_to_text(&user_id);

        let result = conn
            .transaction::<UpdatePasswordResponse, diesel::result::Error, _>(
                |conn| {
                    // Get current password
                    let provider = identity_provider::table
                        .filter(identity_provider::user_id.eq(&user_id_text))
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
                    diesel::update(identity_provider::table)
                        .filter(identity_provider::user_id.eq(&user_id_text))
                        .set(
                            identity_provider::password_hash
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

        let mfa_text =
            serde_json::to_string(&mfa).expect("MFA is always serializable");

        match diesel::update(user::table.find(uuid_to_text(&user_id)))
            .set(user::mfa.eq(Some(mfa_text)))
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
