use crate::{
    prisma::token as token_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        token::{EmailConfirmationTokenMeta, Token, TokenMeta},
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

        let response = client
            .token()
            .create(
                match to_value(meta) {
                    Ok(value) => value,
                    Err(_) => {
                        return creation_err(String::from(
                            "Could not serialize the meta data",
                        ))
                        .with_code(NativeErrorCodes::MYC00002)
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
                    TokenMeta::EmailConfirmation(meta),
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
        _meta: EmailConfirmationTokenMeta,
        _expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        unimplemented!("`TokenRegistration::create_password_change_token` is not implemented")
    }
}
