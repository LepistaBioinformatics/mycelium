use crate::{
    models::{config::DbConfig, token::Token as TokenModel},
    schema::token as token_model,
};
use diesel::prelude::*;

use async_trait::async_trait;
use chrono::{DateTime, Local};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        token::{
            AccountScopedConnectionString, EmailConfirmationTokenMeta,
            MultiTypeMeta, PasswordChangeTokenMeta, RoleScopedConnectionString,
            TenantScopedConnectionString, Token,
        },
    },
    entities::TokenRegistration,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use serde_json::{from_value, to_value};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = TokenRegistration)]
pub struct TokenRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbConfig>,
}

#[async_trait]
impl TokenRegistration for TokenRegistrationSqlDbRepository {
    async fn create_email_confirmation_token(
        &self,
        meta: EmailConfirmationTokenMeta,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut meta_clone = meta.clone();
        if let Err(err) = meta_clone.encrypted_token() {
            return creation_err(format!(
                "Unexpected error detected on token processing: {}",
                err
            ))
            .as_error();
        }

        let meta_value = match to_value(meta_clone) {
            Ok(value) => value,
            Err(_) => {
                return creation_err("Could not serialize the meta data")
                    .as_error()
            }
        };

        let token = diesel::insert_into(token_model::table)
            .values((
                token_model::meta.eq(meta_value),
                token_model::expiration.eq(expires.naive_utc()),
            ))
            .returning(TokenModel::as_returning())
            .get_result::<TokenModel>(conn)
            .map_err(|e| {
                creation_err(format!(
                    "Unexpected error detected on create record: {}",
                    e
                ))
            })?;

        let meta: EmailConfirmationTokenMeta = from_value(token.meta).unwrap();

        Ok(CreateResponseKind::Created(Token::new(
            Some(token.id),
            token.expiration.and_local_timezone(Local).unwrap(),
            MultiTypeMeta::EmailConfirmation(meta),
        )))
    }

    async fn create_password_change_token(
        &self,
        meta: PasswordChangeTokenMeta,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut meta_clone = meta.clone();
        if let Err(err) = meta_clone.encrypted_token() {
            return creation_err(format!(
                "Unexpected error detected on token processing: {}",
                err
            ))
            .as_error();
        }

        let meta_value = match to_value(meta_clone) {
            Ok(value) => value,
            Err(_) => {
                return creation_err("Could not serialize the meta data")
                    .as_error()
            }
        };

        let token = diesel::insert_into(token_model::table)
            .values((
                token_model::meta.eq(meta_value),
                token_model::expiration.eq(expires.naive_utc()),
            ))
            .returning(TokenModel::as_returning())
            .get_result::<TokenModel>(conn)
            .map_err(|e| {
                creation_err(format!(
                    "Unexpected error detected on create record: {}",
                    e
                ))
            })?;

        let meta: PasswordChangeTokenMeta = from_value(token.meta).unwrap();

        Ok(CreateResponseKind::Created(Token::new(
            Some(token.id),
            token.expiration.and_local_timezone(Local).unwrap(),
            MultiTypeMeta::PasswordChange(meta),
        )))
    }

    async fn create_account_scoped_connection_string(
        &self,
        meta: AccountScopedConnectionString,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut meta_clone = meta.clone();
        if let Err(err) = meta_clone.encrypted_token() {
            return creation_err(format!(
                "Unexpected error detected on token processing: {}",
                err
            ))
            .as_error();
        }

        let meta_value = match to_value(meta_clone) {
            Ok(value) => value,
            Err(_) => {
                return creation_err("Could not serialize the meta data")
                    .as_error()
            }
        };

        let token = diesel::insert_into(token_model::table)
            .values((
                token_model::meta.eq(meta_value),
                token_model::expiration.eq(expires.naive_utc()),
            ))
            .returning(TokenModel::as_returning())
            .get_result::<TokenModel>(conn)
            .map_err(|e| {
                creation_err(format!(
                    "Unexpected error detected on create record: {}",
                    e
                ))
            })?;

        let meta: AccountScopedConnectionString =
            from_value(token.meta).unwrap();

        Ok(CreateResponseKind::Created(Token::new(
            Some(token.id),
            token.expiration.and_local_timezone(Local).unwrap(),
            MultiTypeMeta::AccountScopedConnectionString(meta),
        )))
    }

    async fn create_role_scoped_connection_string(
        &self,
        meta: RoleScopedConnectionString,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut meta_clone = meta.clone();
        if let Err(err) = meta_clone.encrypted_token() {
            return creation_err(format!(
                "Unexpected error detected on token processing: {}",
                err
            ))
            .as_error();
        }

        let meta_value = match to_value(meta_clone) {
            Ok(value) => value,
            Err(_) => {
                return creation_err("Could not serialize the meta data")
                    .as_error()
            }
        };

        let token = diesel::insert_into(token_model::table)
            .values((
                token_model::meta.eq(meta_value),
                token_model::expiration.eq(expires.naive_utc()),
            ))
            .returning(TokenModel::as_returning())
            .get_result::<TokenModel>(conn)
            .map_err(|e| {
                creation_err(format!(
                    "Unexpected error detected on create record: {}",
                    e
                ))
            })?;

        let meta: RoleScopedConnectionString = from_value(token.meta).unwrap();

        Ok(CreateResponseKind::Created(Token::new(
            Some(token.id),
            token.expiration.and_local_timezone(Local).unwrap(),
            MultiTypeMeta::RoleScopedConnectionString(meta),
        )))
    }

    async fn create_tenant_scoped_connection_string(
        &self,
        meta: TenantScopedConnectionString,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut meta_clone = meta.clone();
        if let Err(err) = meta_clone.encrypted_token() {
            return creation_err(format!(
                "Unexpected error detected on token processing: {}",
                err
            ))
            .as_error();
        }

        let meta_value = match to_value(meta_clone) {
            Ok(value) => value,
            Err(_) => {
                return creation_err("Could not serialize the meta data")
                    .as_error()
            }
        };

        let token = diesel::insert_into(token_model::table)
            .values((
                token_model::meta.eq(meta_value),
                token_model::expiration.eq(expires.naive_utc()),
            ))
            .returning(TokenModel::as_returning())
            .get_result::<TokenModel>(conn)
            .map_err(|e| {
                creation_err(format!(
                    "Unexpected error detected on create record: {}",
                    e
                ))
            })?;

        let meta: TenantScopedConnectionString =
            from_value(token.meta).unwrap();

        Ok(CreateResponseKind::Created(Token::new(
            Some(token.id),
            token.expiration.and_local_timezone(Local).unwrap(),
            MultiTypeMeta::TenantScopedConnectionString(meta),
        )))
    }
}
