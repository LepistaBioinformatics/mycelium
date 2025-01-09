use super::client::get_client;

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{email::Email, profile::Profile},
    entities::ProfileFetching,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use reqwest::{
    header::{HeaderName, HeaderValue},
    StatusCode,
};
use shaku::Component;
use std::str::FromStr;

#[derive(Component)]
#[shaku(interface = ProfileFetching)]
pub struct ProfileFetchingSvcRepo {
    pub url: String,
}

#[async_trait]
impl ProfileFetching for ProfileFetchingSvcRepo {
    async fn get_from_token(
        &self,
        token: String,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Built HTTP client
        // ? -------------------------------------------------------------------

        let client = get_client().await;

        // ? -------------------------------------------------------------------
        // ? Get the user profile
        // ? -------------------------------------------------------------------

        let response = match client
            .get(self.url.to_owned())
            .header(
                HeaderName::from_str("Authorization").unwrap(),
                HeaderValue::from_str(token.as_str()).unwrap(),
            )
            .send()
            .await
        {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on fetch profile: {err}"
                ))
                .as_error()
            }
            Ok(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Parse response
        // ? -------------------------------------------------------------------

        match response.status() {
            StatusCode::NOT_FOUND => {
                return Ok(FetchResponseKind::NotFound(None))
            }
            StatusCode::OK => {
                let json_res = match response.json::<Profile>().await {
                    Err(err) => {
                        return fetching_err(format!(
                            "Unexpected error on parse profile: {err}"
                        ))
                        .as_error()
                    }
                    Ok(res) => res,
                };

                return Ok(FetchResponseKind::Found(json_res));
            }
            _ => {
                return fetching_err(format!("Invalid response from server."))
                    .as_error()
            }
        }
    }

    async fn get_from_email(
        &self,
        _: Email,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
        unimplemented!("Not implemented yet: Fetch profile from email")
    }
}
