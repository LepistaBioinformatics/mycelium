use crate::{
    models::{
        account::Account as AccountModel, config::DbPoolProvider,
        user::User as UserModel,
    },
    schema::{account, user},
};

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        account::{Account, AccountMeta, AccountMetaKey, VerboseStatus},
        account_type::AccountType,
        native_error_codes::NativeErrorCodes,
    },
    entities::AccountRegistration,
};
use mycelium_base::{
    dtos::Children,
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{creation_err, MappedErrors},
};
use serde_json::{from_value, to_value};
use shaku::Component;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountRegistration)]
pub struct AccountRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl AccountRegistration for AccountRegistrationSqlDbRepository {
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
            Ok(created_account) => Ok(CreateResponseKind::Created(
                self.map_account_model_to_dto(created_account),
            )),
            Err(InternalError::Database(e)) => {
                creation_err(format!("Database error: {}", e)).as_error()
            }
            _ => {
                creation_err("Failed to create subscription account").as_error()
            }
        }
    }

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
            .filter(
                account::account_type
                    .eq(to_value(account_type.clone()).unwrap()),
            )
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

        if omit_user_creation {
            // Create only the account
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
                _ => creation_err("Failed to create account").as_error(),
            }
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
                                user::account_id.eq(Some(new_account.id)),
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
                            .values(&new_user)
                            .execute(conn)?;
                    }

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
                Err(InternalError::NoOwner) => {
                    creation_err("No owner provided").as_error()
                }
            }
        }
    }

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
            .filter(account::tenant_id.eq(Some(tenant_id)))
            .filter(
                account::account_type
                    .eq(to_value(&concrete_account_type).unwrap()),
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

        // Create new account
        let new_account = self
            .create_account_model(
                account.clone(),
                Some(tenant_id),
                concrete_account_type,
            )
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
            _ => {
                creation_err("Failed to create role related account").as_error()
            }
        }
    }

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
            .filter(
                account::account_type
                    .eq(to_value(&concrete_account_type).unwrap()),
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

    async fn register_account_meta(
        &self,
        account_id: Uuid,
        key: AccountMetaKey,
        value: String,
    ) -> Result<CreateResponseKind<AccountMeta>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let transaction_result: Result<AccountMeta, InternalError> = conn
            .transaction(|conn| {
                let mut meta_map = HashMap::new();
                meta_map.insert(key.to_string(), value);

                diesel::update(account::table)
                    .filter(account::id.eq(account_id))
                    .set(account::meta.eq(Some(to_value(&meta_map).unwrap())))
                    .execute(conn)?;

                Ok(AccountMeta::from_iter(meta_map.iter().map(|(k, v)| {
                    (AccountMetaKey::from_str(k).unwrap(), v.to_string())
                })))
            });

        match transaction_result {
            Ok(meta) => Ok(CreateResponseKind::Created(meta)),
            Err(InternalError::Database(e)) => {
                creation_err(format!("Database error: {}", e)).as_error()
            }
            _ => creation_err("Failed to update account meta").as_error(),
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
            created: Local::now().naive_utc(),
            updated: None,
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
            verbose_status: Some(VerboseStatus::from_flags(
                model.is_active,
                model.is_checked,
                model.is_archived,
            )),
            is_default: model.is_default,
            owners: Children::Records(vec![]),
            account_type: from_value(model.account_type).unwrap(),
            guest_users: None,
            created: model.created.and_local_timezone(Local).unwrap(),
            updated: model
                .updated
                .map(|dt| dt.and_local_timezone(Local).unwrap()),
            meta: None,
        }
    }
}

#[derive(Debug)]
enum InternalError {
    Database(diesel::result::Error),

    #[allow(unused)]
    NoOwner,
}

impl From<diesel::result::Error> for InternalError {
    fn from(err: diesel::result::Error) -> Self {
        InternalError::Database(err)
    }
}
