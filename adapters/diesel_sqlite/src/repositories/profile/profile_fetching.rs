use crate::{
    config::SqliteDbPoolProvider,
    models::{account::Account as AccountModel, user::User as UserModel},
    schema::{account, user},
    types::uuid_from_text,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::dtos::account_type::AccountType;
use myc_core::domain::{
    dtos::{
        account::VerboseStatus,
        email::Email,
        native_error_codes::NativeErrorCodes,
        profile::{Owner, Profile},
    },
    entities::ProfileFetching,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use tracing::error;

#[derive(Component)]
#[shaku(interface = ProfileFetching)]
pub struct ProfileFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl ProfileFetching for ProfileFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_profile_from_email", skip_all)]
    async fn get_from_email(
        &self,
        email: Email,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let result = user::table
            .inner_join(
                account::table.on(account::id.nullable().eq(user::account_id)),
            )
            .filter(
                user::email
                    .eq(email.email())
                    .and(account::is_deleted.eq(false)),
            )
            .select((AccountModel::as_select(), UserModel::as_select()))
            .first::<(AccountModel, UserModel)>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch profile: {}", e))
            })?;

        match result {
            None => Ok(FetchResponseKind::NotFound(Some(email.email()))),
            Some((account_row, owner)) => {
                let account_type: AccountType = serde_json::from_str(
                    &account_row.account_type,
                )
                .map_err(|err| {
                    error!("Error on discovery account type: {err}");
                    fetching_err("Unexpected error on discovery account type.")
                })?;

                let (is_subscription, is_manager, is_staff) = match account_type
                {
                    AccountType::Subscription { .. }
                    | AccountType::RoleAssociated { .. } => {
                        (true, false, false)
                    }
                    AccountType::Manager => (false, true, false),
                    AccountType::Staff => (false, true, true),
                    _ => (false, false, false),
                };

                let is_active = owner.is_principal;
                let owner = Owner {
                    id: uuid_from_text(&owner.id).unwrap(),
                    email: Email::from_string(owner.email)?.email(),
                    first_name: Some(owner.first_name),
                    last_name: Some(owner.last_name),
                    username: Some(owner.username),
                    is_principal: owner.is_principal,
                };

                Ok(FetchResponseKind::Found(Profile::new(
                    vec![owner],
                    uuid_from_text(&account_row.id).unwrap(),
                    is_subscription,
                    is_manager,
                    is_staff,
                    is_active,
                    account_row.is_active,
                    account_row.is_checked,
                    account_row.is_archived,
                    account_row.is_deleted,
                    Some(VerboseStatus::from_flags(
                        account_row.is_active,
                        account_row.is_checked,
                        account_row.is_archived,
                        account_row.is_deleted,
                    )),
                    None,
                    None,
                )))
            }
        }
    }

    async fn get_from_token(
        &self,
        _token: String,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
        unimplemented!("Not implemented yet: Fetch profile from token")
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_support::setup_temp_db,
        types::{naive_timestamp_to_text, uuid_to_text},
    };
    use chrono::Utc;
    use uuid::Uuid;

    #[tokio::test]
    async fn get_profile_from_email_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let fetching = ProfileFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let account_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let now = naive_timestamp_to_text(&Utc::now().naive_utc());

        {
            let conn = &mut db.provider.get_pool().get().unwrap();
            diesel::insert_into(account::table)
                .values((
                    account::id.eq(uuid_to_text(&account_id)),
                    account::name.eq("Acme"),
                    account::slug.eq("acme"),
                    account::account_type.eq("\"user\""),
                    account::created.eq(&now),
                ))
                .execute(conn)
                .unwrap();

            diesel::insert_into(user::table)
                .values((
                    user::id.eq(uuid_to_text(&user_id)),
                    user::username.eq("owner"),
                    user::email.eq("owner@acme.test"),
                    user::first_name.eq("Own"),
                    user::last_name.eq("Er"),
                    user::is_active.eq(true),
                    user::created.eq(&now),
                    user::account_id.eq(uuid_to_text(&account_id)),
                    user::is_principal.eq(true),
                ))
                .execute(conn)
                .unwrap();
        }

        let found = match fetching
            .get_from_email(Email::from_string("owner@acme.test".into())?)
            .await?
        {
            FetchResponseKind::Found(profile) => profile,
            FetchResponseKind::NotFound(_) => {
                panic!("expected the profile to be found")
            }
        };
        assert_eq!(found.acc_id, account_id);
        assert!(found.owner_is_active);
        assert!(found.account_is_active);

        let not_found = fetching
            .get_from_email(Email::from_string("nobody@acme.test".into())?)
            .await?;
        assert!(matches!(not_found, FetchResponseKind::NotFound(_)));

        Ok(())
    }
}
