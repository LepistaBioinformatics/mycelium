use crate::middleware::fetch_role_scoped_connection_string_from_request;

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::Future;
use myc_core::domain::dtos::token::RoleScopedConnectionString;
use myc_http_tools::responses::GatewayError;
use serde::Deserialize;
use std::pin::Pin;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MyceliumRoleScopedConnectionStringData(
    RoleScopedConnectionString,
);

impl MyceliumRoleScopedConnectionStringData {
    pub fn new(connection_string: RoleScopedConnectionString) -> Self {
        Self(connection_string)
    }

    pub fn connection_string(&self) -> &RoleScopedConnectionString {
        &self.0
    }

    pub fn tenant_id(&self) -> Option<Uuid> {
        self.0.get_tenant_id()
    }
}

impl FromRequest for MyceliumRoleScopedConnectionStringData {
    type Error = GatewayError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req_clone = req.clone();

        Box::pin(async move {
            fetch_role_scoped_connection_string_from_request(req_clone).await
        })
    }
}
