use crate::{
    prisma::token as token_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::Utc;
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        token::{EmailConfirmationTokenMeta, PasswordChangeTokenMeta},
    },
    entities::TokenInvalidation,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use prisma_client_rust::{PrismaValue, Raw};
use serde_json::from_value;
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TokenInvalidation)]
pub struct TokenInvalidationSqlDbRepository {}

#[async_trait]
impl TokenInvalidation for TokenInvalidationSqlDbRepository {
    async fn get_and_invalidate_email_confirmation_token(
        &self,
        meta: EmailConfirmationTokenMeta,
    ) -> Result<FetchResponseKind<Uuid, String>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Try to fetch user
        // ? -------------------------------------------------------------------

        let (user_id, deleted) = match client
            ._transaction()
            .run(|client| async move {
                let mut token_option: Vec<token_model::Data> = client
                    ._query_raw(Raw::new(
                        "SELECT id, expiration, meta FROM token WHERE meta->'email'->>'username' = {} AND meta->'email'->>'domain' = {} AND meta->>'userId' = {}",
                        vec![
                            PrismaValue::String(meta.to_owned().email.username),
                            PrismaValue::String(meta.to_owned().email.domain),
                            PrismaValue::String(meta.to_owned().user_id.to_string()),
                        ],
                    ))
                    .exec()
                    .await?;

                if token_option.is_empty() {
                    return Ok((None, false));
                }

                // Get the token with the earliest expiration date
                token_option.sort_by(|a, b| a.expiration.cmp(&b.expiration));

                if let Some(token_data) = token_option.to_owned().first() {
                    if token_data.expiration < Utc::now() {
                        return Ok((None, false));
                    }

                    let delete_res = match client
                        .token()
                        .delete(token_model::id::equals(token_data.id))
                        .exec()
                        .await
                    {
                        Err(err) => return Err(err),
                        Ok(_) => true,
                    };

                    Ok((Some(token_data.to_owned()), delete_res))
                } else {
                    Ok((None, false))
                }
            })
            .await
        {
            Ok((data, deleted)) => match data {
                Some(data) => {
                    let user_id =
                        from_value::<EmailConfirmationTokenMeta>(data.meta.to_owned())
                            .unwrap()
                            .user_id;

                    (Some(user_id), deleted)
                }
                None => (None, deleted),
            },
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error detected on fetching token: {err}"
                ))
                .as_error()
            }
        };

        if !deleted {
            return Ok(FetchResponseKind::NotFound(Some(
                "Invalid token".to_string(),
            )));
        }

        if let Some(id) = user_id {
            return Ok(FetchResponseKind::Found(id));
        }

        Ok(FetchResponseKind::NotFound(Some(
            "Invalid operation".to_string(),
        )))
    }

    async fn get_and_invalidate_password_change_token(
        &self,
        meta: PasswordChangeTokenMeta,
    ) -> Result<FetchResponseKind<Uuid, String>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Try to fetch user
        // ? -------------------------------------------------------------------

        let (user_id, deleted) = match client
            ._transaction()
            .run(|client| async move {
                let token_option: Vec<token_model::Data> = client
                    ._query_raw(Raw::new(
                        "SELECT DISTINCT id, expiration, meta FROM token WHERE meta->'email'->>'username' = {} AND meta->'email'->>'domain' = {} AND meta->>'userId' = {} ORDER BY expiration DESC LIMIT 1",
                        vec![
                            PrismaValue::String(meta.to_owned().email.username),
                            PrismaValue::String(meta.to_owned().email.domain),
                            PrismaValue::String(meta.to_owned().user_id.to_string()),
                        ],
                    ))
                    .exec()
                    .await?;

                if token_option.is_empty() {
                    return Ok((None, false));
                }

                // Get the token with the earliest expiration date
                //token_option.sort_by(|a, b| a.expiration.cmp(&b.expiration));

                if let Some(token_data) = token_option.to_owned().first() {
                    if token_data.expiration < Utc::now() {
                        return Ok((None, false));
                    }

                    let delete_res = match client
                        .token()
                        .delete(token_model::id::equals(token_data.id))
                        .exec()
                        .await
                    {
                        Err(err) => return Err(err),
                        Ok(_) => true,
                    };

                    Ok((Some(token_data.to_owned()), delete_res))
                } else {
                    Ok((None, false))
                }
            })
            .await
        {
            Ok((data, deleted)) => match data {
                Some(data) => {
                    let user_id =
                        from_value::<PasswordChangeTokenMeta>(data.meta.to_owned())
                            .unwrap()
                            .user_id;

                    (Some(user_id), deleted)
                }
                None => (None, deleted),
            },
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error detected on fetching token: {err}"
                ))
                .as_error()
            }
        };

        if !deleted {
            return Ok(FetchResponseKind::NotFound(Some(
                "Invalid token".to_string(),
            )));
        }

        if let Some(id) = user_id {
            return Ok(FetchResponseKind::Found(id));
        }

        Ok(FetchResponseKind::NotFound(Some(
            "Invalid operation".to_string(),
        )))
    }
}
