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

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        email::Email,
        native_error_codes::NativeErrorCodes,
        user::{Provider, User},
    },
    entities::UserRegistration,
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
#[shaku(interface = UserRegistration)]
pub struct UserRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl UserRegistration for UserRegistrationSqlDbRepository {
    #[tracing::instrument(name = "get_or_create_user", skip_all)]
    async fn get_or_create(
        &self,
        user_dto: User,
    ) -> Result<GetOrCreateResponseKind<User>, MappedErrors> {
        let provider = user_dto.provider().ok_or_else(|| {
            creation_err("Provider is required to create a user")
                .with_code(NativeErrorCodes::MYC00002)
        })?;

        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if user exists
        let existing_user = user::table
            .filter(user::email.eq(user_dto.email.email()))
            .select(UserModel::as_select())
            .first::<UserModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check user: {}", e))
            })?;

        if let Some(record) = existing_user {
            tracing::info!("User already exists: {record:?}");

            return Ok(GetOrCreateResponseKind::NotCreated(
                dto_from_record(record)?,
                "User created if not exists".to_string(),
            ));
        }

        let new_user_id = uuid_to_text(&Uuid::new_v4());

        // Create new user in transaction
        let result = conn.transaction(|conn| {
            let new_user = UserModel {
                id: new_user_id.clone(),
                username: user_dto.username.clone(),
                email: user_dto.email.email(),
                first_name: user_dto.first_name.clone().unwrap_or_default(),
                last_name: user_dto.last_name.clone().unwrap_or_default(),
                account_id: None,
                is_active: user_dto.is_active,
                is_principal: user_dto.is_principal(),
                created: naive_timestamp_to_text(&Local::now().naive_utc()),
                updated: None,
                mfa: None,
            };

            let created_user = diesel::insert_into(user::table)
                .values(new_user)
                .returning(UserModel::as_returning())
                .get_result::<UserModel>(conn)?;

            // Create identity provider
            let provider_params = match provider {
                Provider::External(name) => IdentityProviderModel {
                    user_id: created_user.id.clone(),
                    name: Some(name),
                    password_hash: None,
                },
                Provider::Internal(pass) => IdentityProviderModel {
                    user_id: created_user.id.clone(),
                    name: None,
                    password_hash: Some(pass.hash),
                },
            };

            diesel::insert_into(identity_provider::table)
                .values(provider_params)
                .execute(conn)?;

            Ok::<UserModel, diesel::result::Error>(created_user)
        });

        match result {
            Ok(record) => {
                Ok(GetOrCreateResponseKind::Created(dto_from_record(record)?))
            }
            Err(e) => creation_err(format!(
                "Unexpected error detected on create user: {e}"
            ))
            .as_error(),
        }
    }
}

/// Builds the domain `User` from a bare user row (no provider lookup),
/// mirroring the postgres registration repo, which never populates
/// `provider` on either the "already exists" or "created" response.
fn dto_from_record(record: UserModel) -> Result<User, MappedErrors> {
    Ok(User::new(
        Some(uuid_from_text(&record.id).unwrap()),
        record.username,
        Email::from_string(record.email)?,
        Some(record.first_name),
        Some(record.last_name),
        record.is_active,
        created_at_from_text(&record.created),
        record.updated.map(|dt| created_at_from_text(&dt)),
        record
            .account_id
            .map(|id| Parent::Id(uuid_from_text(&id).unwrap())),
        None,
    )
    .with_principal(record.is_principal))
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        repositories::user::{
            UserDeletionSqlDbRepository, UserFetchingSqlDbRepository,
            UserUpdatingSqlDbRepository,
        },
        test_support::setup_temp_db,
    };
    use myc_core::domain::{
        dtos::user::{MultiFactorAuthentication, PasswordHash, Totp},
        entities::{UserDeletion, UserFetching, UserUpdating},
    };
    use mycelium_base::entities::{
        DeletionResponseKind, FetchResponseKind, UpdatingResponseKind,
    };

    fn new_user(email: &str, password: PasswordHash) -> User {
        User::new(
            None,
            "owner".into(),
            Email::from_string(email.into()).unwrap(),
            Some("Own".into()),
            Some("Er".into()),
            true,
            Local::now(),
            None,
            None,
            Some(Provider::Internal(password)),
        )
    }

    #[tokio::test]
    async fn user_lifecycle_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();

        let registration = UserRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let fetching = UserFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let updating = UserUpdatingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let deletion = UserDeletionSqlDbRepository {
            db_config: db.provider.clone(),
        };

        // Create
        let old_password = PasswordHash::hash_user_password(b"old-pass");
        let user_dto = new_user("owner@acme.test", old_password.clone());

        let created = match registration.get_or_create(user_dto).await? {
            GetOrCreateResponseKind::Created(user) => user,
            GetOrCreateResponseKind::NotCreated(..) => {
                panic!("expected the user to be created")
            }
        };
        let user_id = created.id.expect("created user must have an id");
        assert_eq!(created.username, "owner");

        // Re-registering the same email must be a no-op (NotCreated)
        let not_created = registration
            .get_or_create(new_user("owner@acme.test", old_password.clone()))
            .await?;
        assert!(matches!(
            not_created,
            GetOrCreateResponseKind::NotCreated(..)
        ));

        // Fetch by id
        let found = match fetching.get_user_by_id(user_id).await? {
            FetchResponseKind::Found(user) => user,
            FetchResponseKind::NotFound(_) => {
                panic!("expected the user to be found by id")
            }
        };
        assert_eq!(found.username, "owner");
        assert!(matches!(found.provider(), Some(Provider::Internal(_))));

        // Fetch by email (redacted)
        let found_by_email = match fetching
            .get_user_by_email(Email::from_string("owner@acme.test".into())?)
            .await?
        {
            FetchResponseKind::Found(user) => user,
            FetchResponseKind::NotFound(_) => {
                panic!("expected the user to be found by email")
            }
        };
        assert_eq!(found_by_email.id, Some(user_id));

        // Update
        let mut update_payload = found.clone();
        update_payload.username = "owner-renamed".into();
        let updated = match updating.update(update_payload).await? {
            UpdatingResponseKind::Updated(user) => user,
            UpdatingResponseKind::NotUpdated(..) => {
                panic!("expected the user to be updated")
            }
        };
        assert_eq!(updated.username, "owner-renamed");

        // Update password
        let new_password = PasswordHash::hash_user_password(b"new-pass")
            .with_raw_password("new-pass".to_string());
        let password_updated =
            updating.update_password(user_id, new_password).await?;
        assert!(matches!(
            password_updated,
            UpdatingResponseKind::Updated((None, true))
        ));

        // Update MFA
        let mfa_updated = updating
            .update_mfa(
                user_id,
                MultiFactorAuthentication {
                    totp: Totp::Disabled,
                },
            )
            .await?;
        assert!(matches!(mfa_updated, UpdatingResponseKind::Updated(true)));

        // Delete
        let deleted = deletion.delete(user_id).await?;
        assert!(matches!(deleted, DeletionResponseKind::Deleted));

        let after_delete = fetching.get_user_by_id(user_id).await?;
        assert!(matches!(after_delete, FetchResponseKind::NotFound(_)));

        Ok(())
    }
}
