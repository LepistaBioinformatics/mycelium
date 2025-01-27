use super::shared::map_model_to_dto;
use crate::{
    models::{config::DbPoolProvider, guest_user::GuestUser as GuestUserModel},
    schema::{guest_user as guest_user_model, guest_user_on_account},
};

use async_trait::async_trait;
use chrono::Local;
use diesel::{
    prelude::*,
    result::{DatabaseErrorKind, Error},
};
use myc_core::domain::{
    dtos::{guest_user::GuestUser, native_error_codes::NativeErrorCodes},
    entities::GuestUserRegistration,
};
use mycelium_base::{
    dtos::Parent,
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestUserRegistration)]
pub struct GuestUserRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl GuestUserRegistration for GuestUserRegistrationSqlDbRepository {
    #[tracing::instrument(name = "get_or_create_guest_user", skip_all)]
    async fn get_or_create(
        &self,
        guest_user: GuestUser,
        account_id: Uuid,
    ) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if guest user exists
        let existing = guest_user_model::table
            .filter(guest_user_model::email.eq(guest_user.email.email()))
            .select(GuestUserModel::as_select())
            .first::<GuestUserModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!(
                    "Failed to check existing guest user: {}",
                    e
                ))
            })?;

        let guest_user = if let Some(record) = existing {
            record
        } else {
            // Create new guest user
            let new_user = GuestUserModel {
                id: Uuid::new_v4().to_string(),
                email: guest_user.email.to_string(),
                guest_role_id: match guest_user.guest_role {
                    Parent::Id(id) => id.to_string(),
                    _ => {
                        return creation_err(
                            "Guest role ID is required".to_string(),
                        )
                        .as_error()
                    }
                },
                created: Local::now(),
                updated: None,
                was_verified: false,
            };

            diesel::insert_into(guest_user_model::table)
                .values(&new_user)
                .get_result::<GuestUserModel>(conn)
                .map_err(|e| {
                    creation_err(format!("Failed to create guest user: {}", e))
                })?
        };

        // Create guest user on account relationship
        diesel::insert_into(guest_user_on_account::table)
            .values((
                guest_user_on_account::guest_user_id.eq(guest_user.id.clone()),
                guest_user_on_account::account_id.eq(account_id.to_string()),
            ))
            .execute(conn)
            .map_err(|e| match e {
                Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                    creation_err("Guest user already exists".to_string())
                        .with_code(NativeErrorCodes::MYC00017)
                        .with_exp_true()
                }
                _ => creation_err(format!(
                    "Failed to create guest user relationship: {}",
                    e
                )),
            })?;

        Ok(GetOrCreateResponseKind::Created(map_model_to_dto(
            guest_user,
        )))
    }
}
