use super::map_model_to_dto;
use crate::{
    config::SqliteDbPoolProvider,
    models::guest_user::GuestUser as GuestUserModel,
    schema::{guest_user, guest_user_on_account},
    types::{naive_timestamp_to_text, timestamp_to_text, uuid_to_text},
};

use async_trait::async_trait;
use chrono::{Local, Utc};
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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl GuestUserRegistration for GuestUserRegistrationSqlDbRepository {
    #[tracing::instrument(name = "get_or_create_guest_user", skip_all)]
    async fn get_or_create(
        &self,
        guest_user_dto: GuestUser,
        account_id: Uuid,
    ) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let guest_role_id = guest_user_dto
            .guest_role_id()
            .map_err(|e| creation_err(format!("Invalid guest role: {e}")))?;

        // Check if guest user exists
        let existing = guest_user::table
            .filter(guest_user::email.eq(guest_user_dto.email.email()).and(
                guest_user::guest_role_id.eq(uuid_to_text(&guest_role_id)),
            ))
            .select(GuestUserModel::as_select())
            .first::<GuestUserModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!(
                    "Failed to check existing guest user: {}",
                    e
                ))
            })?;

        let guest_user_record = if let Some(record) = existing {
            record
        } else {
            // Create new guest user
            let guest_role_id = match guest_user_dto.guest_role {
                Parent::Id(id) => id,
                _ => {
                    return creation_err(
                        "Guest role ID is required".to_string(),
                    )
                    .as_error()
                }
            };

            let new_user = GuestUserModel {
                id: uuid_to_text(&Uuid::new_v4()),
                email: guest_user_dto.email.to_string(),
                guest_role_id: uuid_to_text(&guest_role_id),
                created: timestamp_to_text(&Local::now().with_timezone(&Utc)),
                updated: None,
                was_verified: false,
            };

            diesel::insert_into(guest_user::table)
                .values(&new_user)
                .returning(GuestUserModel::as_returning())
                .get_result::<GuestUserModel>(conn)
                .map_err(|e| {
                    creation_err(format!("Failed to create guest user: {}", e))
                })?
        };

        // Create guest user on account relationship. Unlike postgres
        // (`created TIMESTAMPTZ DEFAULT now()`), the SQLite table has no
        // server-side default, so `created` is set explicitly.
        diesel::insert_into(guest_user_on_account::table)
            .values((
                guest_user_on_account::guest_user_id.eq(&guest_user_record.id),
                guest_user_on_account::account_id.eq(uuid_to_text(&account_id)),
                guest_user_on_account::created
                    .eq(naive_timestamp_to_text(&Utc::now().naive_utc())),
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
            guest_user_record,
            None,
        )))
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        repositories::{
            guest_role::GuestRoleRegistrationSqlDbRepository,
            guest_user::{
                GuestUserDeletionSqlDbRepository,
                GuestUserFetchingSqlDbRepository,
                GuestUserOnAccountFetchingSqlDbRepository,
                GuestUserOnAccountUpdatingSqlDbRepository,
            },
        },
        schema::account,
        test_support::setup_temp_db,
    };
    use myc_core::domain::{
        dtos::{
            email::Email,
            guest_role::{GuestRole, Permission},
            guest_user_on_account::GuestUserOnAccount,
        },
        entities::{
            GuestRoleRegistration, GuestUserDeletion, GuestUserFetching,
            GuestUserOnAccountFetching, GuestUserOnAccountUpdating,
        },
    };
    use mycelium_base::entities::{
        DeletionResponseKind, FetchManyResponseKind, UpdatingResponseKind,
    };

    #[tokio::test]
    async fn guest_user_lifecycle_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let account_id = Uuid::new_v4();

        // Seed a minimal account row (guest_user_on_account.account_id has a
        // FK to account(id); tenant_id is nullable so no tenant is needed).
        {
            let conn = &mut db.provider.get_pool().get().unwrap();
            diesel::insert_into(account::table)
                .values((
                    account::id.eq(uuid_to_text(&account_id)),
                    account::name.eq("Acme"),
                    account::slug.eq("acme"),
                    account::created
                        .eq(naive_timestamp_to_text(&Utc::now().naive_utc())),
                ))
                .execute(conn)
                .unwrap();
        }

        let role_registration = GuestRoleRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let registration = GuestUserRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let fetching = GuestUserFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let deletion = GuestUserDeletionSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let on_account_fetching = GuestUserOnAccountFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let on_account_updating = GuestUserOnAccountUpdatingSqlDbRepository {
            db_config: db.provider.clone(),
        };

        // Create a guest role
        let role_dto = GuestRole::new(
            None,
            "Collaborator".into(),
            None,
            Permission::Write,
            None,
            false,
        );
        let role = match role_registration.get_or_create(role_dto).await? {
            GetOrCreateResponseKind::Created(role) => role,
            GetOrCreateResponseKind::NotCreated(..) => {
                panic!("expected the role to be created")
            }
        };
        let role_id = role.id.expect("created role must have an id");

        // Create guest user
        let email = Email::from_string("guest@acme.test".into())?;
        let guest_user_dto =
            GuestUser::new_unverified(email, Parent::Id(role_id), None);
        let created = match registration
            .get_or_create(guest_user_dto, account_id)
            .await?
        {
            GetOrCreateResponseKind::Created(user) => user,
            GetOrCreateResponseKind::NotCreated(..) => {
                panic!("expected the guest user to be created")
            }
        };
        let guest_user_id =
            created.id.expect("created guest user must have an id");

        // List
        let listed = match fetching.list(account_id, None, None).await? {
            FetchManyResponseKind::FoundPaginated { records, .. } => records,
            _ => panic!("expected paginated guest users"),
        };
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, Some(guest_user_id));

        // Update permit/deny flags
        let flags_updated = match on_account_updating
            .update(GuestUserOnAccount {
                guest_user_id,
                account_id,
                created: Local::now(),
                permit_flags: vec!["read".into(), "write".into()],
                deny_flags: vec!["delete".into()],
            })
            .await?
        {
            UpdatingResponseKind::Updated(updated) => updated,
            UpdatingResponseKind::NotUpdated(..) => {
                panic!("expected the flags to be updated")
            }
        };
        assert_eq!(flags_updated.permit_flags, vec!["read", "write"]);
        assert_eq!(flags_updated.deny_flags, vec!["delete"]);

        // list_by_guest_role_id reflects the updated flags
        let by_role = match on_account_fetching
            .list_by_guest_role_id(role_id, account_id)
            .await?
        {
            FetchManyResponseKind::Found(records) => records,
            _ => panic!("expected found records"),
        };
        assert_eq!(by_role.len(), 1);
        assert_eq!(by_role[0].permit_flags, vec!["read", "write"]);

        // Delete
        let deleted = deletion
            .delete(role_id, account_id, "guest@acme.test".into())
            .await?;
        assert!(matches!(deleted, DeletionResponseKind::Deleted));

        // `list()` has no empty-result guard (matches postgres exactly): it
        // always returns `FoundPaginated`, with an empty `records` vec and
        // `count: 0` once the only guest_user_on_account row is gone.
        let after_delete = fetching.list(account_id, None, None).await?;
        match after_delete {
            FetchManyResponseKind::FoundPaginated {
                count, records, ..
            } => {
                assert_eq!(count, 0);
                assert!(records.is_empty());
            }
            _ => panic!("expected an (empty) paginated response"),
        }

        Ok(())
    }
}
