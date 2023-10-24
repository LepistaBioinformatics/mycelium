use crate::{
    prisma::session_token as session_token_model,
    repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    entities::FetchResponseKind,
    utils::errors::{factories::fetching_err, MappedErrors},
};
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::SessionTokenFetching,
};
use shaku::Component;
use std::process::id as process_id;

#[derive(Component)]
#[shaku(interface = SessionTokenFetching)]
pub struct SessionTokenFetchingSqlDbRepository {}

#[async_trait]
impl SessionTokenFetching for SessionTokenFetchingSqlDbRepository {
    async fn get(
        &self,
        session_key: String,
    ) -> Result<FetchResponseKind<String, String>, MappedErrors> {
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
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client
            .session_token()
            .find_unique(session_token_model::key::equals(
                session_key.to_owned(),
            ))
            .exec()
            .await
        {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on parse session token: {err}"
                ))
                .as_error()
            }
            Ok(res) => match res {
                None => Ok(FetchResponseKind::NotFound(Some(session_key))),
                Some(_) => Ok(FetchResponseKind::Found(session_key)),
            },
        }
    }
}
