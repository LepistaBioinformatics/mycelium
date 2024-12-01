use crate::middleware::fetch_tenant_scoped_connection_string_from_request;

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::Future;
use myc_core::domain::dtos::token::TenantScopedConnectionString;
use myc_http_tools::responses::GatewayError;
use serde::Deserialize;
use std::pin::Pin;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MyceliumTenantScopedConnectionStringData(
    TenantScopedConnectionString,
);

impl MyceliumTenantScopedConnectionStringData {
    pub fn new(connection_string: TenantScopedConnectionString) -> Self {
        Self(connection_string)
    }

    pub fn connection_string(&self) -> &TenantScopedConnectionString {
        &self.0
    }

    pub fn tenant_id(&self) -> Option<Uuid> {
        self.0.get_tenant_id()
    }
}

impl FromRequest for MyceliumTenantScopedConnectionStringData {
    type Error = GatewayError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req_clone = req.clone();

        Box::pin(async move {
            fetch_tenant_scoped_connection_string_from_request(req_clone).await
        })
    }
}
