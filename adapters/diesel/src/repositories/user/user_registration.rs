use crate::{
    models::{
        config::DbConfig,
        identity_provider::IdentityProvider as IdentityProviderModel,
        user::User as UserModel,
    },
    schema::{
        identity_provider as identity_provider_model, user as user_model,
    },
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
    pub db_config: Arc<dyn DbConfig>,
}

#[async_trait]
impl UserRegistration for UserRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        user: User,
    ) -> Result<GetOrCreateResponseKind<User>, MappedErrors> {
        let provider = user.provider().ok_or_else(|| {
            creation_err("Provider is required to create a user")
                .with_code(NativeErrorCodes::MYC00002)
        })?;

        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if user exists
        let existing_user = user_model::table
            .filter(user_model::email.eq(user.email.email()))
            .select(UserModel::as_select())
            .first::<UserModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check user: {}", e))
            })?;

        if let Some(record) = existing_user {
            return Ok(GetOrCreateResponseKind::NotCreated(
                User::new(
                    Some(record.id),
                    record.username,
                    Email::from_string(record.email)?,
                    Some(record.first_name),
                    Some(record.last_name),
                    record.is_active,
                    record.created.and_local_timezone(Local).unwrap(),
                    record
                        .updated
                        .map(|dt| dt.and_local_timezone(Local).unwrap()),
                    record.account_id.map(Parent::Id),
                    None,
                )
                .with_principal(record.is_principal),
                "User already exists".to_string(),
            ));
        }

        // Create new user in transaction
        let result = conn.transaction(|conn| {
            // Create user
            let new_user = UserModel {
                id: Uuid::new_v4(),
                username: user.username.clone(),
                email: user.email.email(),
                first_name: user.first_name.clone().unwrap_or_default(),
                last_name: user.last_name.clone().unwrap_or_default(),
                account_id: None,
                is_active: user.is_active,
                is_principal: user.is_principal(),
                created: Local::now().naive_utc(),
                updated: None,
                mfa: None,
            };

            let created_user = diesel::insert_into(user_model::table)
                .values(&new_user)
                .returning(UserModel::as_returning())
                .get_result::<UserModel>(conn)?;

            // Create identity provider
            let provider_params = match provider {
                Provider::External(name) => IdentityProviderModel {
                    user_id: created_user.id,
                    name: Some(name),
                    password_hash: None,
                },
                Provider::Internal(pass) => IdentityProviderModel {
                    user_id: created_user.id,
                    name: None,
                    password_hash: Some(pass.hash),
                },
            };

            diesel::insert_into(identity_provider_model::table)
                .values(&provider_params)
                .execute(conn)?;

            Ok::<UserModel, diesel::result::Error>(created_user)
        });

        match result {
            Ok(record) => Ok(GetOrCreateResponseKind::Created(
                User::new(
                    Some(record.id),
                    record.username,
                    Email::from_string(record.email)?,
                    Some(record.first_name),
                    Some(record.last_name),
                    record.is_active,
                    record.created.and_local_timezone(Local).unwrap(),
                    record
                        .updated
                        .map(|dt| dt.and_local_timezone(Local).unwrap()),
                    record.account_id.map(Parent::Id),
                    None,
                )
                .with_principal(record.is_principal),
            )),
            Err(e) => creation_err(format!(
                "Unexpected error detected on create user: {}",
                e
            ))
            .as_error(),
        }
    }
}
