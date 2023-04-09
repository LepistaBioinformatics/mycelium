use super::client::get_client;

use async_trait::async_trait;
use clean_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{email::Email, profile::Profile},
    entities::ProfileFetching,
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
    async fn get(
        &self,
        _: Option<Email>,
        token: Option<String>,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
        let token = if let None = token {
            return Err(fetching_err(
                String::from(
                    "Token could not be empty during profile checking.",
                ),
                Some(false),
                None,
            ));
        } else {
            token.unwrap()
        };

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
                return Err(fetching_err(
                    format!("Unexpected error on fetch profile: {err}"),
                    Some(true),
                    None,
                ))
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
                        return Err(fetching_err(
                            format!("Unexpected error on parse profile: {err}"),
                            Some(true),
                            None,
                        ))
                    }
                    Ok(res) => res,
                };

                return Ok(FetchResponseKind::Found(json_res));
            }
            _ => {
                return Err(fetching_err(
                    format!("Invalid response from server."),
                    Some(true),
                    None,
                ))
            }
        }
    }
}
