use crate::models::internal_error::InternalError;
use crate::{
    models::{
        account::Account as AccountModel, config::DbPoolProvider,
        user::User as UserModel,
    },
    schema::{account, manager_account_on_tenant, user},
};

use async_trait::async_trait;
use chrono::Local;
use diesel::{
    dsl::sql,
    prelude::*,
    result::{DatabaseErrorKind, Error},
};
use myc_core::domain::dtos::account::Modifier;
use myc_core::domain::dtos::email::Email;
use myc_core::domain::dtos::user::User;
use myc_core::domain::{
    dtos::{
        account::{Account, AccountMetaKey, VerboseStatus},
        account_type::AccountType,
        native_error_codes::NativeErrorCodes,
    },
    entities::AccountRegistration,
};
use mycelium_base::utils::errors::fetching_err;
use mycelium_base::{
    dtos::Children,
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{creation_err, MappedErrors},
};
use serde_json::{from_value, json, to_value};
use shaku::Component;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountRegistration)]
pub struct AccountRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
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
            .find(new_account.id)
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .map(|account| self.map_account_model_to_dto(account))
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

        // Check if account already exists
        let existing_account = account::table
            .filter(account::slug.eq(&account.slug))
            .filter(sql::<diesel::sql_types::Bool>(&format!(
                "account_type::jsonb @> '{}'",
                match serde_json::to_string(&account_type) {
                    Ok(json) => json,
                    Err(e) => {
                        return creation_err(format!(
                            "Failed to serialize account type: {}",
                            e
                        ))
                        .as_error();
                    }
                }
            )))
            .filter(account::tenant_id.eq(Some(tenant_id)))
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing account: {}", e))
            })?;

        if let Some(account) = existing_account {
            return Ok(GetOrCreateResponseKind::NotCreated(
                self.map_account_model_to_dto(account),
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

        let transaction_result: Result<AccountModel, InternalError> = conn
            .transaction(|conn| {
                diesel::insert_into(account::table)
                    .values(&new_account)
                    .execute(conn)?;

                diesel::insert_into(manager_account_on_tenant::table)
                    .values((
                        manager_account_on_tenant::tenant_id.eq(tenant_id),
                        manager_account_on_tenant::account_id
                            .eq(new_account.id),
                    ))
                    .execute(conn)?;

                account::table
                    .find(new_account.id)
                    .select(AccountModel::as_select())
                    .first(conn)
                    .map_err(InternalError::from)
            });

        match transaction_result {
            Ok(created_account) => Ok(GetOrCreateResponseKind::Created(
                self.map_account_model_to_dto(created_account),
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
                self.map_account_model_to_dto(account),
                "Account already exists".to_string(),
            ));
        }

        let new_account = self
            .create_account_model(account.clone(), None, account.account_type)
            .map_err(|e| {
                creation_err(format!("Failed to create account: {}", e))
            })?;

        tracing::trace!("new_account: {:?}", new_account);

        if omit_user_creation {
            // Create only the account
            let created_account = diesel::insert_into(account::table)
                .values(&new_account)
                .returning(AccountModel::as_select())
                .get_result(conn)
                .map(|account| self.map_account_model_to_dto(account))
                .map_err(|e| {
                    creation_err(format!("Failed to create tag: {}", e))
                })?;

            Ok(GetOrCreateResponseKind::Created(created_account))
        } else {
            // Create account and user
            let owner = match account.owners {
                Children::Records(mut users) => match users.pop() {
                    Some(owner) => owner,
                    None => {
                        return creation_err("No owner provided").as_error()
                    }
                },
                _ => return creation_err("Invalid owner data").as_error(),
            };

            let transaction_result: Result<AccountModel, InternalError> = conn
                .transaction(|conn| {
                    diesel::insert_into(account::table)
                        .values(&new_account)
                        .execute(conn)?;

                    if user_exists && owner.id.is_some() {
                        diesel::update(user::table)
                            .filter(user::id.eq(owner.id.unwrap()))
                            .set((
                                user::account_id
                                    .eq(Some(new_account.id.clone())),
                                user::is_active.eq(owner.is_active),
                            ))
                            .execute(conn)?;
                    } else {
                        let new_user = UserModel {
                            id: Uuid::new_v4(),
                            username: owner.username.clone(),
                            email: owner.email.email(),
                            first_name: owner
                                .first_name
                                .clone()
                                .unwrap_or_default(),
                            last_name: owner
                                .last_name
                                .clone()
                                .unwrap_or_default(),
                            account_id: None,
                            is_active: owner.is_active,
                            is_principal: owner.is_principal(),
                            created: Local::now().naive_utc(),
                            updated: None,
                            mfa: None,
                        };

                        diesel::insert_into(user::table)
                            .values(new_user)
                            .execute(conn)?;
                    }

                    account::table
                        .find(new_account.id)
                        .select(AccountModel::as_select())
                        .first(conn)
                        .map_err(InternalError::from)
                });

            match transaction_result {
                Ok(created_account) => {
                    let mut account =
                        self.map_account_model_to_dto(created_account.clone());

                    let owners = UserModel::belonging_to(&created_account)
                        .select(UserModel::as_select())
                        .load::<UserModel>(conn)
                        .map_err(|e| {
                            fetching_err(format!("Failed to fetch users: {e}"))
                        })?
                        .into_iter()
                        .map(|o| {
                            User::new_public_redacted(
                                o.id,
                                Email::from_string(o.email).unwrap(),
                                o.username,
                                o.created.and_local_timezone(Local).unwrap(),
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

        let (tenant_id, role_name, role_id) = match account.account_type.clone()
        {
            AccountType::RoleAssociated {
                tenant_id,
                role_name,
                role_id,
            } => (tenant_id, role_name, role_id),
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
            role_id,
        };

        // Check if account already exists
        let existing_account = account::table
            .filter(account::tenant_id.eq(Some(tenant_id)).and(sql::<
                diesel::sql_types::Bool,
            >(
                &format!(
                    "account_type::jsonb @> '{}'",
                    match serde_json::to_string(&concrete_account_type) {
                        Ok(json) => json,
                        Err(e) => {
                            return creation_err(format!(
                                "Failed to serialize account type: {e}"
                            ))
                            .as_error();
                        }
                    }
                ),
            )))
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing account: {e}"))
            })?;

        if let Some(account) = existing_account {
            return Ok(GetOrCreateResponseKind::NotCreated(
                self.map_account_model_to_dto(account),
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
                account.tenant_id = Some(tenant_id);
                account
            })
            .map_err(|e| {
                creation_err(format!("Failed to create account: {}", e))
            })?;

        match diesel::insert_into(account::table)
            .values(&new_account)
            .returning(AccountModel::as_select())
            .get_result::<AccountModel>(conn)
        {
            Ok(result) => {
                let account = self.map_account_model_to_dto(result);
                Ok(GetOrCreateResponseKind::Created(account))
            }
            Err(e) => match e {
                Error::DatabaseError(
                    DatabaseErrorKind::ForeignKeyViolation,
                    _,
                ) => {
                    return Ok(GetOrCreateResponseKind::NotCreated(
                        self.map_account_model_to_dto(new_account),
                        "Account already exists".to_string(),
                    ));
                }
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

        // Check if account already exists
        let existing_account = account::table
            .filter(account::slug.eq(&account.slug))
            .filter(sql::<diesel::sql_types::Bool>(&format!(
                "account_type::jsonb @> '{}'",
                match serde_json::to_string(&concrete_account_type) {
                    Ok(json) => json,
                    Err(e) => {
                        return creation_err(format!(
                            "Failed to serialize account type: {}",
                            e
                        ))
                        .as_error();
                    }
                }
            )))
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing account: {}", e))
            })?;

        if let Some(account) = existing_account {
            return Ok(GetOrCreateResponseKind::NotCreated(
                self.map_account_model_to_dto(account),
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
                    .find(new_account.id)
                    .select(AccountModel::as_select())
                    .first(conn)
                    .map_err(InternalError::from)
            });

        match transaction_result {
            Ok(created_account) => Ok(GetOrCreateResponseKind::Created(
                self.map_account_model_to_dto(created_account),
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

        let account = account::table
            .find(account_id)
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing account: {}", e))
            })?;

        if let Some(account) = account {
            let mut meta_map: std::collections::HashMap<String, String> =
                account
                    .meta
                    .map(|m| serde_json::from_value(m).unwrap())
                    .unwrap_or_default();

            meta_map.insert(format!("{key}", key = key), value);

            diesel::update(account::table)
                .filter(account::id.eq(account_id))
                .set(account::meta.eq(Some(to_value(&meta_map).unwrap())))
                .execute(conn)
                .map_err(|e| {
                    creation_err(format!("Failed to update tenant meta: {}", e))
                })?;

            Ok(CreateResponseKind::Created(meta_map))
        } else {
            creation_err("Account not found").as_error()
        }
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
            id: Uuid::new_v4(),
            name: account.name,
            slug: account.slug,
            meta: None,
            tenant_id,
            account_type: to_value(account_type).unwrap(),
            is_active: account.is_active,
            is_checked: account.is_checked,
            is_archived: account.is_archived,
            is_default: account.is_default,
            is_deleted: account.is_deleted,
            created: Local::now().naive_utc(),
            created_by: account.created_by.map(|m| to_value(m).unwrap()),
            updated: None,
            updated_by: account.updated_by.map(|m| to_value(m).unwrap()),
        })
    }

    fn map_account_model_to_dto(&self, model: AccountModel) -> Account {
        Account {
            id: Some(model.id),
            name: model.name,
            slug: model.slug,
            tags: None,
            is_active: model.is_active,
            is_checked: model.is_checked,
            is_archived: model.is_archived,
            is_deleted: model.is_deleted,
            verbose_status: Some(VerboseStatus::from_flags(
                model.is_active,
                model.is_checked,
                model.is_archived,
                model.is_deleted,
            )),
            is_default: model.is_default,
            owners: Children::Records(vec![]),
            account_type: from_value(model.account_type).unwrap(),
            guest_users: None,
            created_at: model.created.and_local_timezone(Local).unwrap(),
            created_by: model.created_by.map(|m| from_value(m).unwrap()),
            updated_at: model
                .updated
                .map(|dt| dt.and_local_timezone(Local).unwrap()),
            updated_by: model
                .updated_by
                .map(|m| {
                    //
                    // Check if the Value is a empty object
                    //
                    if m == json!({}) {
                        None
                    } else {
                        let modifier: Modifier = from_value(m).unwrap();
                        Some(modifier)
                    }
                })
                .flatten(),
            meta: None,
        }
    }
}
