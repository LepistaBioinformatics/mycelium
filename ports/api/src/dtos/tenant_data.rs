use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::Future;
use myc_core::settings::DEFAULT_TENANT_ID_KEY;
use myc_http_tools::responses::GatewayError;
use serde::Deserialize;
use std::{pin::Pin, str::FromStr};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TenantData(Uuid);

impl TenantData {
    pub fn tenant_id(&self) -> &Uuid {
        &self.0
    }
}

impl FromRequest for TenantData {
    type Error = GatewayError;
    type Future =
        Pin<Box<dyn Future<Output = Result<TenantData, Self::Error>>>>;

    /// Extracts the tenant id from the request header
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req_clone = req.clone();

        Box::pin(async move {
            let tenant_id = req_clone
                .headers()
                .get(DEFAULT_TENANT_ID_KEY)
                .ok_or_else(|| {
                    GatewayError::BadRequest(format!(
                        "Missing tenant id in request. Expected header: {}",
                        DEFAULT_TENANT_ID_KEY
                    ))
                })?
                .to_str()
                .map_err(|_| {
                    GatewayError::BadRequest(
                        "Invalid tenant id in request".to_string(),
                    )
                })?;

            Ok(TenantData(Uuid::from_str(tenant_id).map_err(|_| {
                GatewayError::BadRequest(
                    "Invalid tenant id in request".to_string(),
                )
            })?))
        })
    }
}
