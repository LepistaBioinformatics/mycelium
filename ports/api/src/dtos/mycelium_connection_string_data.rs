use crate::middleware::fetch_connection_string_from_request;

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::Future;
use myc_core::domain::dtos::token::UserAccountConnectionString;
use myc_http_tools::responses::GatewayError;
use serde::Deserialize;
use std::pin::Pin;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MyceliumConnectionStringData(UserAccountConnectionString);

impl MyceliumConnectionStringData {
    pub fn new(connection_string: UserAccountConnectionString) -> Self {
        Self(connection_string)
    }

    pub fn connection_string(&self) -> &UserAccountConnectionString {
        &self.0
    }
}

impl FromRequest for MyceliumConnectionStringData {
    type Error = GatewayError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req_clone = req.clone();

        Box::pin(async move {
            fetch_connection_string_from_request(req_clone).await
        })
    }
}
