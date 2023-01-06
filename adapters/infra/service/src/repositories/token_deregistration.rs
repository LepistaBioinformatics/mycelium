use super::client::get_client;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use myc_core::domain::{dtos::token::Token, entities::TokenDeregistration};
use reqwest::StatusCode;
use shaku::Component;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TokenDeregistration)]
pub struct TokenDeregistrationSvcRepository {
    pub url: String,
}

#[async_trait]
impl TokenDeregistration for TokenDeregistrationSvcRepository {
    async fn get_then_delete(
        &self,
        token: Token,
    ) -> Result<FetchResponseKind<Token, Uuid>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Built HTTP client
        // ? -------------------------------------------------------------------

        let client = get_client().await;

        // ? -------------------------------------------------------------------
        // ? Get the user profile
        // ? -------------------------------------------------------------------

        let response = match client
            .get(format!("{}{}", self.url.to_owned(), token.token))
            .query(&[("service", token.own_service.to_owned())])
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
                return Ok(FetchResponseKind::NotFound(Some(token.token)))
            }
            StatusCode::OK => {
                let json_res = match response.json::<Token>().await {
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
