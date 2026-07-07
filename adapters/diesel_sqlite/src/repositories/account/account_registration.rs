use super::{created_at_from_text, map_account_model_to_dto};
use crate::{
    config::SqliteDbPoolProvider,
    models::{account::Account as AccountModel, user::User as UserModel},
    repositories::internal_error::InternalError,
    schema::{account, manager_account_on_tenant, user},
    types::{naive_timestamp_to_text, uuid_from_text, uuid_to_text},
};

use async_trait::async_trait;
use chrono::Local;
use diesel::{
    prelude::*,
    result::{DatabaseErrorKind, Error},
};
use myc_core::domain::{
    dtos::{
        account::{Account, AccountMetaKey},
        account_type::AccountType,
        email::Email,
        native_error_codes::NativeErrorCodes,
        user::User,
    },
    entities::AccountRegistration,
};
use mycelium_base::utils::errors::fetching_err;
use mycelium_base::{
    dtos::Children,
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountRegistration)]
pub struct AccountRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl AccountRegistration for AccountRegistrationSqlDbRepository {
    #[tracing::instrument(name = "create_subscription_account", skip_all)]
    async fn create_subscription_account(
        &self,
        account: Account,
        tenant_id: Uuid,
    ) -> Result<CreateResponseKind<Account>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let new_account = self
            .create_account_model(
                account.clone(),
                Some(tenant_id),
                AccountType::Subscription { tenant_id },
            )
            .map_err(|e| {
                creation_err(format!("Failed to create account: {}", e))
            })?;

        // Create account
        diesel::insert_into(account::table)
            .values(&new_account)
            .execute(conn)
            .map_err(|e| match e {
                Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                    creation_err("Account already exists")
                        .with_exp_true()
                        .with_code(NativeErrorCodes::MYC00018)
                }
                _ => {
                    tracing::error!("Failed to create account: {}", e);

                    creation_err("Failed to create account")
                }
            })?;

        let record = account::table
            .find(&new_account.id)
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .map(map_account_model_to_dto)
            .map_err(|e| {
                creation_err(format!("Failed to check existing account: {}", e))
            })?;

        Ok(CreateResponseKind::Created(record))
    }

    #[tracing::instrument(
        name = "get_or_create_tenant_management_account",
        skip_all
    )]
    async fn get_or_create_tenant_management_account(
        &self,
        account: Account,
        tenant_id: Uuid,
    ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let account_type = AccountType::TenantManager { tenant_id };
        let tenant_id_text = uuid_to_text(&tenant_id);

        let account_type_json =
            serde_json::to_string(&account_type).map_err(|e| {
                creation_err(format!("Failed to serialize account type: {e}"))
            })?;

        // Check if account already exists
        let existing_account = account::table
            .filter(account::slug.eq(&account.slug))
            .filter(account::account_type.eq(&account_type_json))
            .filter(account::tenant_id.eq(Some(&tenant_id_text)))
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing account: {}", e))
            })?;

        if let Some(account) = existing_account {
            return Ok(GetOrCreateResponseKind::NotCreated(
                map_account_model_to_dto(account),
                "Account already exists".to_string(),
            ));
        }

        // Create new account
        let new_account = self
            .create_account_model(
                account.clone(),
                Some(tenant_id),
                account_type,
            )
            .map_err(|e| {
                creation_err(format!("Failed to create account: {}", e))
            })?;

        let manager_link_id = uuid_to_text(&Uuid::new_v4());

        let transaction_result: Result<AccountModel, InternalError> = conn
            .transaction(|conn| {
                diesel::insert_into(account::table)
                    .values(&new_account)
                    .execute(conn)?;

                diesel::insert_into(manager_account_on_tenant::table)
                    .values((
                        manager_account_on_tenant::id.eq(manager_link_id),
                        manager_account_on_tenant::tenant_id.eq(tenant_id_text),
                        manager_account_on_tenant::account_id
                            .eq(&new_account.id),
                    ))
                    .execute(conn)?;

                account::table
                    .find(&new_account.id)
                    .select(AccountModel::as_select())
                    .first(conn)
                    .map_err(InternalError::from)
            });

        match transaction_result {
            Ok(created_account) => Ok(GetOrCreateResponseKind::Created(
                map_account_model_to_dto(created_account),
            )),
            Err(InternalError::Database(e)) => {
                creation_err(format!("Database error: {}", e)).as_error()
            }
            _ => creation_err("Failed to create tenant management account")
                .as_error(),
        }
    }

    #[tracing::instrument(name = "get_or_create_user_account", skip_all)]
    async fn get_or_create_user_account(
        &self,
        account: Account,
        user_exists: bool,
        omit_user_creation: bool,
    ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Get emails from owners
        let emails = match account.owners.to_owned() {
            Children::Ids(_) => vec![],
            Children::Records(res) => res
                .into_iter()
                .map(|user| user.email.email())
                .collect::<Vec<String>>(),
        };

        // Check if account exists
        let existing_account = user::table
            .inner_join(account::table)
            .filter(
                user::email
                    .eq_any(emails)
                    .or(account::slug.eq(&account.name.clone())),
            )
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing account: {}", e))
            })?;

        if let Some(account) = existing_account {
            return Ok(GetOrCreateResponseKind::NotCreated(
                map_account_model_to_dto(account),
                "Account already exists".to_string(),
            ));
        }

        let new_account = self
            .create_account_model(account.clone(), None, account.account_type)
            .map_err(|e| {
                creation_err(format!("Failed to create account: {}", e))
            })?;

        tracing::trace!("new_account: {:?}", new_account.id);

        if omit_user_creation {
            // Create only the account
            diesel::insert_into(account::table)
                .values(&new_account)
                .execute(conn)
                .map_err(|e| {
                    creation_err(format!("Failed to create tag: {}", e))
                })?;

            let created_account = account::table
                .find(&new_account.id)
                .select(AccountModel::as_select())
                .first::<AccountModel>(conn)
                .map(map_account_model_to_dto)
                .map_err(|e| {
                    creation_err(format!(
                        "Failed to check existing account: {}",
                        e
                    ))
                })?;

            return Ok(GetOrCreateResponseKind::Created(created_account));
        }

        // Create account and user
        let owner = match account.owners {
            Children::Records(mut users) => match users.pop() {
                Some(owner) => owner,
                None => return creation_err("No owner provided").as_error(),
            },
            _ => return creation_err("Invalid owner data").as_error(),
        };

        let new_user_id = uuid_to_text(&Uuid::new_v4());

        let transaction_result: Result<AccountModel, InternalError> = conn
            .transaction(|conn| {
                diesel::insert_into(account::table)
                    .values(&new_account)
                    .execute(conn)?;

                if user_exists && owner.id.is_some() {
                    diesel::update(user::table)
                        .filter(user::id.eq(uuid_to_text(&owner.id.unwrap())))
                        .set((
                            user::account_id.eq(Some(new_account.id.clone())),
                            user::is_active.eq(owner.is_active),
                        ))
                        .execute(conn)?;
                } else {
                    let new_user = UserModel {
                        id: new_user_id.clone(),
                        username: owner.username.clone(),
                        email: owner.email.email(),
                        first_name: owner
                            .first_name
                            .clone()
                            .unwrap_or_default(),
                        last_name: owner.last_name.clone().unwrap_or_default(),
                        account_id: None,
                        is_active: owner.is_active,
                        is_principal: owner.is_principal(),
                        created: naive_timestamp_to_text(
                            &Local::now().naive_utc(),
                        ),
                        updated: None,
                        mfa: None,
                    };

                    diesel::insert_into(user::table)
                        .values(new_user)
                        .execute(conn)?;
                }

                account::table
                    .find(&new_account.id)
                    .select(AccountModel::as_select())
                    .first(conn)
                    .map_err(InternalError::from)
            });

        match transaction_result {
            Ok(created_account) => {
                let mut account =
                    map_account_model_to_dto(created_account.clone());

                let owners = UserModel::belonging_to(&created_account)
                    .select(UserModel::as_select())
                    .load::<UserModel>(conn)
                    .map_err(|e| {
                        fetching_err(format!("Failed to fetch users: {e}"))
                    })?
                    .into_iter()
                    .map(|o| {
                        User::new_public_redacted(
                            uuid_from_text(&o.id).unwrap(),
                            Email::from_string(o.email).unwrap(),
                            o.username,
                            created_at_from_text(&o.created),
                            o.is_active,
                            o.is_principal,
                        )
                    })
                    .collect::<Vec<User>>();

                account.owners = Children::Records(owners);

                Ok(GetOrCreateResponseKind::Created(account))
            }
            Err(InternalError::Database(e)) => {
                creation_err(format!("Database error: {}", e)).as_error()
            }
            Err(InternalError::Unknown) => {
                creation_err("No owner provided").as_error()
            }
        }
    }

    #[tracing::instrument(
        name = "get_or_create_role_related_account",
        skip_all
    )]
    async fn get_or_create_role_related_account(
        &self,
        account: Account,
    ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let (tenant_id, role_name, read_role_id, write_role_id) =
            match account.account_type.clone() {
                AccountType::RoleAssociated {
                    tenant_id,
                    role_name,
                    read_role_id,
                    write_role_id,
                } => (tenant_id, role_name, read_role_id, write_role_id),
                _ => {
                    return creation_err(
                        "Could not create account. Invalid account type.",
                    )
                    .as_error()
                }
            };

        let concrete_account_type = AccountType::RoleAssociated {
            tenant_id,
            role_name,
            read_role_id,
            write_role_id,
        };

        let account_type_json = serde_json::to_string(&concrete_account_type)
            .map_err(|e| {
            creation_err(format!("Failed to serialize account type: {e}"))
        })?;

        // Check if account already exists
        let existing_account = account::table
            .filter(account::tenant_id.eq(Some(uuid_to_text(&tenant_id))))
            .filter(account::account_type.eq(&account_type_json))
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing account: {e}"))
            })?;

        if let Some(account) = existing_account {
            return Ok(GetOrCreateResponseKind::NotCreated(
                map_account_model_to_dto(account),
                "Account already exists".to_string(),
            ));
        }

        // Create new account
        let new_account = self
            .create_account_model(
                account.clone(),
                Some(tenant_id),
                concrete_account_type,
            )
            .map(|mut account| {
                account.tenant_id = Some(uuid_to_text(&tenant_id));
                account
            })
            .map_err(|e| {
                creation_err(format!("Failed to create account: {}", e))
            })?;

        match diesel::insert_into(account::table)
            .values(&new_account)
            .execute(conn)
        {
            Ok(_) => {
                let result = account::table
                    .find(&new_account.id)
                    .select(AccountModel::as_select())
                    .first::<AccountModel>(conn)
                    .map_err(|e| {
                        creation_err(format!(
                            "Failed to check existing account: {}",
                            e
                        ))
                    })?;

                let account = map_account_model_to_dto(result);
                Ok(GetOrCreateResponseKind::Created(account))
            }
            Err(e) => match e {
                Error::DatabaseError(
                    DatabaseErrorKind::ForeignKeyViolation,
                    _,
                ) => Ok(GetOrCreateResponseKind::NotCreated(
                    map_account_model_to_dto(new_account),
                    "Account already exists".to_string(),
                )),
                _ => creation_err(format!("Failed to create account: {}", e))
                    .as_error(),
            },
        }
    }

    #[tracing::instrument(
        name = "get_or_create_actor_related_account",
        skip_all
    )]
    async fn get_or_create_actor_related_account(
        &self,
        account: Account,
    ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let actor = match account.account_type.clone() {
            AccountType::ActorAssociated { actor } => actor,
            _ => {
                return creation_err(
                    "Could not create account. Invalid account type.",
                )
                .as_error()
            }
        };

        let concrete_account_type = AccountType::ActorAssociated { actor };

        let account_type_json = serde_json::to_string(&concrete_account_type)
            .map_err(|e| {
            creation_err(format!("Failed to serialize account type: {e}"))
        })?;

        // Check if account already exists
        let existing_account = account::table
            .filter(account::slug.eq(&account.slug))
            .filter(account::account_type.eq(&account_type_json))
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing account: {}", e))
            })?;

        if let Some(account) = existing_account {
            return Ok(GetOrCreateResponseKind::NotCreated(
                map_account_model_to_dto(account),
                "Account already exists".to_string(),
            ));
        }

        // Create new account
        let new_account = self
            .create_account_model(account.clone(), None, concrete_account_type)
            .map_err(|e| {
                creation_err(format!("Failed to create account: {}", e))
            })?;

        let transaction_result: Result<AccountModel, InternalError> = conn
            .transaction(|conn| {
                diesel::insert_into(account::table)
                    .values(&new_account)
                    .execute(conn)?;

                account::table
                    .find(&new_account.id)
                    .select(AccountModel::as_select())
                    .first(conn)
                    .map_err(InternalError::from)
            });

        match transaction_result {
            Ok(created_account) => Ok(GetOrCreateResponseKind::Created(
                map_account_model_to_dto(created_account),
            )),
            Err(InternalError::Database(e)) => {
                creation_err(format!("Database error: {}", e)).as_error()
            }
            _ => creation_err("Failed to create actor related account")
                .as_error(),
        }
    }

    #[tracing::instrument(name = "register_account_meta", skip_all)]
    async fn register_account_meta(
        &self,
        account_id: Uuid,
        key: AccountMetaKey,
        value: String,
    ) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let account_id_text = uuid_to_text(&account_id);

        let account = account::table
            .find(&account_id_text)
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing account: {}", e))
            })?;

        let Some(account) = account else {
            return creation_err("Account not found").as_error();
        };

        let mut meta_map: HashMap<String, String> = account
            .meta
            .map(|m| serde_json::from_str(&m).unwrap())
            .unwrap_or_default();

        meta_map.insert(format!("{key}", key = key), value);

        let meta_text = serde_json::to_string(&meta_map)
            .expect("meta map is always serializable");

        diesel::update(account::table)
            .filter(account::id.eq(&account_id_text))
            .set(account::meta.eq(Some(meta_text)))
            .execute(conn)
            .map_err(|e| {
                creation_err(format!("Failed to update tenant meta: {}", e))
            })?;

        Ok(CreateResponseKind::Created(meta_map))
    }
}

impl AccountRegistrationSqlDbRepository {
    fn create_account_model(
        &self,
        account: Account,
        tenant_id: Option<Uuid>,
        account_type: AccountType,
    ) -> Result<AccountModel, MappedErrors> {
        Ok(AccountModel {
            id: uuid_to_text(&Uuid::new_v4()),
            name: account.name,
            slug: account.slug,
            meta: None,
            tenant_id: tenant_id.map(|id| uuid_to_text(&id)),
            account_type: serde_json::to_string(&account_type).map_err(
                |e| {
                    creation_err(format!(
                        "Failed to serialize account type: {e}"
                    ))
                },
            )?,
            is_active: account.is_active,
            is_checked: account.is_checked,
            is_archived: account.is_archived,
            is_default: account.is_system_account,
            is_deleted: account.is_deleted,
            created: naive_timestamp_to_text(&Local::now().naive_utc()),
            created_by: account
                .created_by
                .map(|m| serde_json::to_string(&m).unwrap()),
            updated: None,
            updated_by: account
                .updated_by
                .map(|m| serde_json::to_string(&m).unwrap()),
        })
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
            account::{
                AccountDeletionSqlDbRepository, AccountFetchingSqlDbRepository,
                AccountUpdatingSqlDbRepository,
            },
            account_tag::AccountTagRegistrationSqlDbRepository,
        },
        test_support::setup_temp_db,
    };
    use myc_core::domain::{
        dtos::related_accounts::RelatedAccounts,
        entities::{
            AccountDeletion, AccountFetching, AccountTagRegistration,
            AccountUpdating,
        },
    };
    use mycelium_base::entities::{
        DeletionResponseKind, FetchResponseKind, UpdatingResponseKind,
    };

    #[tokio::test]
    async fn account_lifecycle_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let tenant_id = Uuid::new_v4();

        // `account.tenant_id` has a FK to `tenant(id)` (mirrors postgres'
        // `fk_account_tenant`); seed a minimal tenant row directly since the
        // tenant repositories are not implemented yet (SM-T8).
        {
            use crate::schema::tenant;

            let conn = &mut db.provider.get_pool().get().unwrap();
            diesel::insert_into(tenant::table)
                .values((
                    tenant::id.eq(uuid_to_text(&tenant_id)),
                    tenant::name.eq("Acme Tenant"),
                    tenant::created
                        .eq(naive_timestamp_to_text(&Local::now().naive_utc())),
                ))
                .execute(conn)
                .unwrap();
        }

        let registration = AccountRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let fetching = AccountFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let updating = AccountUpdatingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let deletion = AccountDeletionSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let tag_registration = AccountTagRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };

        // Create
        let account =
            Account::new_subscription_account("Acme".into(), tenant_id, None);

        let created = match registration
            .create_subscription_account(account, tenant_id)
            .await?
        {
            CreateResponseKind::Created(account) => account,
            CreateResponseKind::NotCreated(..) => {
                panic!("expected the account to be created")
            }
        };

        let account_id = created.id.expect("created account must have an id");
        assert_eq!(created.name, "Acme");
        assert!(!created.is_deleted);

        // Fetch
        let found = match fetching
            .get(account_id, RelatedAccounts::HasStaffPrivileges)
            .await?
        {
            FetchResponseKind::Found(account) => account,
            FetchResponseKind::NotFound(_) => {
                panic!("expected the account to be found")
            }
        };
        assert_eq!(found.name, "Acme");

        // Update
        let updated = match updating
            .update_own_account_name(account_id, "Acme Updated".into())
            .await?
        {
            UpdatingResponseKind::Updated(account) => account,
            UpdatingResponseKind::NotUpdated(..) => {
                panic!("expected the account to be updated")
            }
        };
        assert_eq!(updated.name, "Acme Updated");
        assert!(updated.updated_at.is_some());

        // Tag
        let tag_outcome = tag_registration
            .get_or_create(account_id, "vip".into(), HashMap::new())
            .await?;
        assert!(matches!(
            tag_outcome,
            GetOrCreateResponseKind::Created(ref tag) if tag.value == "vip"
        ));

        let found_with_tag = match fetching
            .get(account_id, RelatedAccounts::HasStaffPrivileges)
            .await?
        {
            FetchResponseKind::Found(account) => account,
            FetchResponseKind::NotFound(_) => {
                panic!("expected the account to be found")
            }
        };
        let tags = found_with_tag.tags.expect("expected tags to be present");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].value, "vip");

        // Soft delete
        let deleted = deletion
            .soft_delete_account(
                account_id,
                AccountType::Subscription { tenant_id },
                RelatedAccounts::HasStaffPrivileges,
            )
            .await?;
        assert!(matches!(deleted, DeletionResponseKind::Deleted));

        let after_delete = match fetching
            .get(account_id, RelatedAccounts::HasStaffPrivileges)
            .await?
        {
            FetchResponseKind::Found(account) => account,
            FetchResponseKind::NotFound(_) => {
                panic!("soft delete must not remove the row")
            }
        };
        assert!(after_delete.is_deleted);
        assert!(!after_delete.is_active);
        assert_eq!(after_delete.name, format!("{account_id}-deleted"));

        Ok(())
    }
}
