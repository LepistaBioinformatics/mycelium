use crate::{
    prisma::token as token_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        token::{
            AccountScopedConnectionStringMeta, EmailConfirmationTokenMeta,
            MultiTypeMeta, PasswordChangeTokenMeta, Token,
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
use std::process::id as process_id;

#[derive(Component)]
#[shaku(interface = TokenRegistration)]
pub struct TokenRegistrationSqlDbRepository {}

#[async_trait]
impl TokenRegistration for TokenRegistrationSqlDbRepository {
    async fn create_email_confirmation_token(
        &self,
        meta: EmailConfirmationTokenMeta,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        let mut meta_clone = meta.clone();

        if let Err(err) = meta_clone.encrypted_token() {
            return creation_err(format!(
                "Unexpected error detected on token processing: {err}"
            ))
            .as_error();
        }

        let response = client
            .token()
            .create(
                match to_value(meta_clone) {
                    Ok(value) => value,
                    Err(_) => {
                        return creation_err(String::from(
                            "Could not serialize the meta data",
                        ))
                        .as_error()
                    }
                },
                vec![token_model::expiration::set(DateTime::from(expires))],
            )
            .exec()
            .await;

        match response {
            Ok(res) => {
                let meta: EmailConfirmationTokenMeta =
                    from_value(res.meta).unwrap();

                let token = Token::new(
                    Some(res.id),
                    res.expiration.into(),
                    MultiTypeMeta::EmailConfirmation(meta),
                );

                return Ok(CreateResponseKind::Created(token));
            }
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on create record: {err}"
                ))
                .as_error();
            }
        }
    }

    async fn create_password_change_token(
        &self,
        meta: PasswordChangeTokenMeta,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        let mut meta_clone = meta.clone();

        if let Err(err) = meta_clone.encrypted_token() {
            return creation_err(format!(
                "Unexpected error detected on token processing: {err}"
            ))
            .as_error();
        }

        let response = client
            .token()
            .create(
                match to_value(meta_clone) {
                    Ok(value) => value,
                    Err(_) => {
                        return creation_err(String::from(
                            "Could not serialize the meta data",
                        ))
                        .as_error()
                    }
                },
                vec![token_model::expiration::set(DateTime::from(expires))],
            )
            .exec()
            .await;

        match response {
            Ok(res) => {
                let meta: PasswordChangeTokenMeta =
                    from_value(res.meta).unwrap();

                let token = Token::new(
                    Some(res.id),
                    res.expiration.into(),
                    MultiTypeMeta::PasswordChange(meta),
                );

                return Ok(CreateResponseKind::Created(token));
            }
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on create record: {err}"
                ))
                .as_error();
            }
        }
    }

    async fn create_account_scoped_connection_string(
        &self,
        meta: AccountScopedConnectionStringMeta,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        let mut meta_clone = meta.clone();

        if let Err(err) = meta_clone.encrypted_token() {
            return creation_err(format!(
                "Unexpected error detected on token processing: {err}"
            ))
            .as_error();
        }

        let response = client
            .token()
            .create(
                match to_value(meta_clone) {
                    Ok(value) => value,
                    Err(_) => {
                        return creation_err(String::from(
                            "Could not serialize the meta data",
                        ))
                        .as_error()
                    }
                },
                vec![token_model::expiration::set(DateTime::from(expires))],
            )
            .exec()
            .await;

        match response {
            Ok(res) => {
                let meta: AccountScopedConnectionStringMeta =
                    from_value(res.meta).unwrap();

                let token = Token::new(
                    Some(res.id),
                    res.expiration.into(),
                    MultiTypeMeta::AccountScopedConnectionString(meta),
                );

                return Ok(CreateResponseKind::Created(token));
            }
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on create record: {err}"
                ))
                .as_error();
            }
        }
    }
}
