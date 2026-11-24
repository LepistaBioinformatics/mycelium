use crate::{
    models::{
        config::DbPoolProvider,
        guest_user_on_account::GuestUserOnAccount as GuestUserOnAccountModel,
    },
    schema::{guest_user as guest_user_model, guest_user_on_account},
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        email::Email, guest_user_on_account::GuestUserOnAccount,
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
        guest_emails: Vec<Email>,
        account_id: Uuid,
        guest_role_id: Uuid,
    ) -> Result<UpdatingResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let emails: Vec<String> =
            guest_emails.iter().map(|email| email.email()).collect();

        //
        // The caller's own pending guest on this account and role. Filtering
        // by email keeps another guest of the same account from being
        // verified in its place.
        //
        let guest_user_id = guest_user_model::table
            .inner_join(guest_user_on_account::table)
            .filter(guest_user_on_account::account_id.eq(account_id))
            .filter(guest_user_model::email.eq_any(&emails))
            .filter(guest_user_model::guest_role_id.eq(guest_role_id))
            .filter(guest_user_model::was_verified.eq(false))
            .select(guest_user_model::id)
            .first::<Uuid>(conn)
            .optional()
            .map_err(|e| {
                updating_err(format!("Failed to fetch guest user: {}", e))
            })?;

        let Some(guest_user_id) = guest_user_id else {
            return Ok(UpdatingResponseKind::NotUpdated(
                account_id,
                "No unverified guest user found for account".to_string(),
            ));
        };

        diesel::update(guest_user_model::table.find(guest_user_id))
            .set(guest_user_model::was_verified.eq(true))
            .execute(conn)
            .map_err(|e| {
                updating_err(format!("Failed to update guest user: {}", e))
            })?;

        Ok(UpdatingResponseKind::Updated(guest_user_id))
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
