use crate::{
    config::SqliteDbPoolProvider,
    models::{
        guest_user::GuestUser as GuestUserModel,
        guest_user_on_account::GuestUserOnAccount as GuestUserOnAccountModel,
    },
    schema::{guest_role, guest_user, guest_user_on_account},
    types::{string_array_to_text, uuid_from_text, uuid_to_text},
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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
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
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Find guest role by name
        let guest_role_id = guest_role::table
            .filter(guest_role::name.eq(&guest_role_name))
            .filter(guest_role::permission.eq(permission.to_i32()))
            .select(guest_role::id)
            .first::<String>(conn)
            .optional()
            .map_err(|e| {
                updating_err(format!("Failed to fetch guest role: {}", e))
            })?;

        let Some(guest_role_id) = guest_role_id else {
            return Ok(UpdatingResponseKind::NotUpdated(
                (guest_role_name, account_id, permission),
                "Guest role not found".to_string(),
            ));
        };

        // Find guest user by account
        let guest_user_record = guest_user::table
            .inner_join(guest_user_on_account::table)
            .filter(
                guest_user_on_account::account_id.eq(uuid_to_text(&account_id)),
            )
            .filter(guest_user::was_verified.eq(false))
            .select(GuestUserModel::as_select())
            .first::<GuestUserModel>(conn)
            .optional()
            .map_err(|e| {
                updating_err(format!("Failed to fetch guest user: {}", e))
            })?;

        match guest_user_record {
            Some(user) => {
                // Update guest user
                diesel::update(guest_user::table.find(&user.id))
                    .set((
                        guest_user::guest_role_id.eq(&guest_role_id),
                        guest_user::was_verified.eq(true),
                    ))
                    .execute(conn)
                    .map_err(|e| {
                        updating_err(format!(
                            "Failed to update guest user: {}",
                            e
                        ))
                    })?;

                Ok(UpdatingResponseKind::Updated((
                    guest_role_name,
                    account_id,
                    permission,
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
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Convert Vec<String> to Option<String> for database
        let permit_flags = if guest_user_on_account.permit_flags.is_empty() {
            None
        } else {
            Some(
                string_array_to_text(&guest_user_on_account.permit_flags)
                    .map_err(|e| {
                        updating_err(format!(
                            "Failed to serialize permit flags: {e}"
                        ))
                    })?,
            )
        };

        let deny_flags = if guest_user_on_account.deny_flags.is_empty() {
            None
        } else {
            Some(
                string_array_to_text(&guest_user_on_account.deny_flags)
                    .map_err(|e| {
                        updating_err(format!(
                            "Failed to serialize deny flags: {e}"
                        ))
                    })?,
            )
        };

        let guest_user_id_text =
            uuid_to_text(&guest_user_on_account.guest_user_id);
        let account_id_text = uuid_to_text(&guest_user_on_account.account_id);

        // Update only permit_flags and deny_flags using composite primary key
        let updated = diesel::update(
            guest_user_on_account::table.filter(
                guest_user_on_account::guest_user_id
                    .eq(&guest_user_id_text)
                    .and(guest_user_on_account::account_id.eq(&account_id_text)),
            ),
        )
        .set((
            guest_user_on_account::permit_flags.eq(permit_flags),
            guest_user_on_account::deny_flags.eq(deny_flags),
        ))
        .returning(GuestUserOnAccountModel::as_returning())
        .get_result::<GuestUserOnAccountModel>(conn)
        .map_err(|e| {
            if e == diesel::result::Error::NotFound {
                updating_err(format!(
                    "Guest user on account not found: guest_user_id={}, account_id={}",
                    guest_user_on_account.guest_user_id,
                    guest_user_on_account.account_id
                ))
            } else {
                updating_err(format!(
                    "Failed to update guest user on account: {}",
                    e
                ))
            }
        })?;

        // Map model back to DTO
        let dto = GuestUserOnAccount {
            guest_user_id: uuid_from_text(&updated.guest_user_id).unwrap(),
            account_id: uuid_from_text(&updated.account_id).unwrap(),
            created: crate::types::naive_timestamp_from_text(&updated.created)
                .unwrap()
                .and_local_timezone(chrono::Local)
                .unwrap(),
            permit_flags: updated
                .permit_flags
                .map(|f| decode_string_array(&f))
                .unwrap_or_default(),
            deny_flags: updated
                .deny_flags
                .map(|f| decode_string_array(&f))
                .unwrap_or_default(),
        };

        Ok(UpdatingResponseKind::Updated(dto))
    }
}

fn decode_string_array(value: &str) -> Vec<String> {
    crate::types::string_array_from_text(value).unwrap()
}
