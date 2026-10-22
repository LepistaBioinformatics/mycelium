use crate::{
    config::SqliteDbPoolProvider,
    models::guest_user_on_account::GuestUserOnAccount as GuestUserOnAccountModel,
    schema::{guest_user, guest_user_on_account},
    types::{string_array_to_text, uuid_from_text, uuid_to_text},
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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
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
        let guest_user_id = guest_user::table
            .inner_join(guest_user_on_account::table)
            .filter(
                guest_user_on_account::account_id.eq(uuid_to_text(&account_id)),
            )
            .filter(guest_user::email.eq_any(&emails))
            .filter(guest_user::guest_role_id.eq(uuid_to_text(&guest_role_id)))
            .filter(guest_user::was_verified.eq(false))
            .select(guest_user::id)
            .first::<String>(conn)
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

        diesel::update(guest_user::table.find(&guest_user_id))
            .set(guest_user::was_verified.eq(true))
            .execute(conn)
            .map_err(|e| {
                updating_err(format!("Failed to update guest user: {}", e))
            })?;

        Ok(UpdatingResponseKind::Updated(uuid_from_text(
            &guest_user_id,
        )?))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::setup_temp_db;
    use diesel::sql_query;

    const NOW: &str = "2026-10-22T00:00:00+00:00";

    struct Fixture {
        account_id: Uuid,
        role_id: Uuid,
        other_role_id: Uuid,
    }

    /// Alice and Bob invited to the same account with the same role, Bob
    /// also holding a pending invitation with another role.
    fn seed(conn: &mut SqliteConnection) -> Fixture {
        let fixture = Fixture {
            account_id: Uuid::new_v4(),
            role_id: Uuid::new_v4(),
            other_role_id: Uuid::new_v4(),
        };

        sql_query(format!(
            "INSERT INTO account (id, name, created, slug) \
             VALUES ('{}', 'Account', '{NOW}', 'account')",
            fixture.account_id
        ))
        .execute(conn)
        .unwrap();

        for (role_id, permission) in
            [(fixture.role_id, 1), (fixture.other_role_id, 0)]
        {
            sql_query(format!(
                "INSERT INTO guest_role (id, name, permission, slug, created) \
                 VALUES ('{role_id}', 'rc-manager', {permission}, \
                 'rc-manager', '{NOW}')"
            ))
            .execute(conn)
            .unwrap();
        }

        for (email, role_id) in [
            ("alice@example.com", fixture.role_id),
            ("bob@example.com", fixture.role_id),
            ("bob@example.com", fixture.other_role_id),
        ] {
            let guest_id = Uuid::new_v4();
            sql_query(format!(
                "INSERT INTO guest_user (id, email, guest_role_id, created) \
                 VALUES ('{guest_id}', '{email}', '{role_id}', '{NOW}')"
            ))
            .execute(conn)
            .unwrap();
            sql_query(format!(
                "INSERT INTO guest_user_on_account \
                 (guest_user_id, account_id, created) \
                 VALUES ('{guest_id}', '{}', '{NOW}')",
                fixture.account_id
            ))
            .execute(conn)
            .unwrap();
        }

        fixture
    }

    fn verified(
        conn: &mut SqliteConnection,
        email: &str,
        role_id: Uuid,
    ) -> bool {
        guest_user::table
            .filter(guest_user::email.eq(email))
            .filter(guest_user::guest_role_id.eq(uuid_to_text(&role_id)))
            .select(guest_user::was_verified)
            .first::<bool>(conn)
            .unwrap()
    }

    #[tokio::test]
    async fn verifies_only_the_callers_guest_on_the_given_role(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let conn = &mut db.provider.get_pool().get().unwrap();
        let fixture = seed(conn);
        let repo = GuestUserOnAccountUpdatingSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let result = repo
            .accept_invitation(
                vec![Email::from_string("bob@example.com".into())?],
                fixture.account_id,
                fixture.role_id,
            )
            .await?;

        assert!(matches!(result, UpdatingResponseKind::Updated(_)));
        assert!(verified(conn, "bob@example.com", fixture.role_id));
        assert!(!verified(conn, "alice@example.com", fixture.role_id));
        assert!(!verified(conn, "bob@example.com", fixture.other_role_id));

        Ok(())
    }

    #[tokio::test]
    async fn does_not_verify_a_guest_of_another_email(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let conn = &mut db.provider.get_pool().get().unwrap();
        let fixture = seed(conn);
        let repo = GuestUserOnAccountUpdatingSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let result = repo
            .accept_invitation(
                vec![Email::from_string("carol@example.com".into())?],
                fixture.account_id,
                fixture.role_id,
            )
            .await?;

        assert!(matches!(result, UpdatingResponseKind::NotUpdated(..)));
        assert!(!verified(conn, "alice@example.com", fixture.role_id));
        assert!(!verified(conn, "bob@example.com", fixture.role_id));

        Ok(())
    }
}
