use crate::{
    prisma::token as token_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
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
use prisma_client_rust::and;
use serde_json::{from_value, to_value};
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
                .with_code(NativeErrorCodes::MYC00001.as_str())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Try to fetch user
        // ? -------------------------------------------------------------------

        let search_meta = match to_value(meta) {
            Ok(res) => res,
            Err(_) => {
                return fetching_err(String::from(
                    "Could not serialize meta data.",
                ))
                .as_error()
            }
        };

        let (user_id, deleted) = match client
            ._transaction()
            .run(|client| async move {
                let token_option = client
                    .token()
                    .find_first(vec![and![
                        token_model::meta::equals(search_meta),
                        token_model::expiration::gte(DateTime::from(
                            Utc::now(),
                        )),
                    ]])
                    .exec()
                    .await?;

                if let Some(token_data) = token_option {
                    let delete_res = match client
                        .token()
                        .delete(token_model::id::equals(token_data.id))
                        .exec()
                        .await
                    {
                        Err(err) => return Err(err),
                        Ok(_) => true,
                    };

                    Ok((Some(token_data), delete_res))
                } else {
                    Ok((None, false))
                }
            })
            .await
        {
            Ok((data, deleted)) => match data {
                Some(data) => {
                    let user_id =
                        from_value::<EmailConfirmationTokenMeta>(data.meta)
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
        _: PasswordChangeTokenMeta,
    ) -> Result<FetchResponseKind<Uuid, String>, MappedErrors> {
        unimplemented!(
            "TokenFetching::get_and_invalidate_password_change_token not implemented"
        )
    }
}
