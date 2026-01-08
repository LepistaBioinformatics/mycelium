use crate::{
    models::{
        config::DbPoolProvider, guest_user::GuestUser as GuestUserModel,
        guest_user_on_account::GuestUserOnAccount as GuestUserOnAccountModel,
    },
    schema::{
        guest_role as guest_role_model, guest_user as guest_user_model,
        guest_user_on_account,
    },
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        guest_role::Permission, guest_user_on_account::GuestUserOnAccount,
        native_error_codes::NativeErrorCodes,
    },
    entities::GuestUserOnAccountUpdating,
};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestUserOnAccountUpdating)]
pub struct GuestUserOnAccountUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl GuestUserOnAccountUpdating for GuestUserOnAccountUpdatingSqlDbRepository {
    #[tracing::instrument(name = "accept_invitation", skip_all)]
    async fn accept_invitation(
        &self,
        guest_role_name: String,
        account_id: Uuid,
        permission: Permission,
    ) -> Result<UpdatingResponseKind<(String, Uuid, Permission)>, MappedErrors>
    {
        let guest_role_name = guest_role_name.clone();
        let permission = permission.clone();

        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Find guest role by name
        let guest_role = guest_role_model::table
            .filter(guest_role_model::name.eq(&guest_role_name))
            .filter(guest_role_model::permission.eq(permission.to_i32()))
            .select(guest_role_model::id)
            .first::<Uuid>(conn)
            .optional()
            .map_err(|e| {
                updating_err(format!("Failed to fetch guest role: {}", e))
            })?;

        let guest_role_id = match guest_role {
            Some(id) => id,
            None => {
                return Ok(UpdatingResponseKind::NotUpdated(
                    (guest_role_name, account_id, permission),
                    "Guest role not found".to_string(),
                ))
            }
        };

        // Find guest user by account
        let guest_user = guest_user_model::table
            .inner_join(guest_user_on_account::table)
            .filter(guest_user_on_account::account_id.eq(account_id))
            .filter(guest_user_model::was_verified.eq(false))
            .select(GuestUserModel::as_select())
            .first::<GuestUserModel>(conn)
            .optional()
            .map_err(|e| {
                updating_err(format!("Failed to fetch guest user: {}", e))
            })?;

        match guest_user {
            Some(user) => {
                // Update guest user
                diesel::update(guest_user_model::table.find(user.id))
                    .set((
                        guest_user_model::guest_role_id.eq(guest_role_id),
                        guest_user_model::was_verified.eq(true),
                    ))
                    .get_result::<GuestUserModel>(conn)
                    .map_err(|e| {
                        updating_err(format!(
                            "Failed to update guest user: {}",
                            e
                        ))
                    })?;

                Ok(UpdatingResponseKind::Updated((
                    guest_role_name.clone(),
                    account_id,
                    permission.clone(),
                )))
            }
            None => Ok(UpdatingResponseKind::NotUpdated(
                (guest_role_name, account_id, permission),
                "No unverified guest user found for account".to_string(),
            )),
        }
    }

    #[tracing::instrument(name = "update", skip_all)]
    async fn update(
        &self,
        guest_user_on_account: GuestUserOnAccount,
    ) -> Result<UpdatingResponseKind<GuestUserOnAccount>, MappedErrors> {
        use chrono::Local;

        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Convert Vec<String> to Option<Vec<String>> for database
        let permit_flags = if guest_user_on_account.permit_flags.is_empty() {
            None
        } else {
            Some(guest_user_on_account.permit_flags.clone())
        };

        let deny_flags = if guest_user_on_account.deny_flags.is_empty() {
            None
        } else {
            Some(guest_user_on_account.deny_flags.clone())
        };

        // Update only permit_flags and deny_flags using composite primary key
        let updated = diesel::update(
            guest_user_on_account::table.filter(
                guest_user_on_account::guest_user_id
                    .eq(guest_user_on_account.guest_user_id)
                    .and(
                        guest_user_on_account::account_id
                            .eq(guest_user_on_account.account_id),
                    ),
            ),
        )
        .set((
            guest_user_on_account::permit_flags.eq(permit_flags),
            guest_user_on_account::deny_flags.eq(deny_flags),
        ))
        .get_result::<GuestUserOnAccountModel>(conn)
        .map_err(|e| {
            if e == diesel::result::Error::NotFound {
                updating_err(format!(
                    "Guest user on account not found: guest_user_id={}, account_id={}",
                    guest_user_on_account.guest_user_id,
                    guest_user_on_account.account_id
                ))
            } else {
                updating_err(format!("Failed to update guest user on account: {}", e))
            }
        })?;

        // Map model back to DTO
        let dto = GuestUserOnAccount {
            guest_user_id: updated.guest_user_id,
            account_id: updated.account_id,
            created: updated
                .created
                .and_local_timezone(Local)
                .unwrap()
                .with_timezone(&Local),
            permit_flags: updated.permit_flags.unwrap_or_default(),
            deny_flags: updated.deny_flags.unwrap_or_default(),
        };

        Ok(UpdatingResponseKind::Updated(dto))
    }
}
