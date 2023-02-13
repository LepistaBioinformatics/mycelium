use crate::{responses::ForwardingError, LicensedResources, Profile};

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures_util::future::{err, ok, Ready};
use myc_core::settings::DEFAULT_PROFILE_KEY;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayProfileData {
    pub email: String,
    pub current_account_id: Uuid,
    pub is_subscription: bool,
    pub is_manager: bool,
    pub is_staff: bool,
    pub owner_is_active: bool,
    pub account_is_active: bool,
    pub account_was_approved: bool,
    pub licensed_resources: Option<Vec<LicensedResources>>,
}

impl GatewayProfileData {
    pub fn to_profile(&self) -> Profile {
        Profile {
            email: self.email.to_owned(),
            current_account_id: self.current_account_id,
            is_subscription: self.is_subscription,
            is_manager: self.is_manager,
            is_staff: self.is_staff,
            owner_is_active: self.owner_is_active,
            account_is_active: self.account_is_active,
            account_was_approved: self.account_was_approved,
            licensed_resources: self.licensed_resources.to_owned(),
        }
    }
}

impl FromRequest for GatewayProfileData {
    type Error = ForwardingError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        match req.headers().get(DEFAULT_PROFILE_KEY) {
            None => err(ForwardingError::Forbidden(
                "Unable to check user identity.".to_string(),
            )),
            Some(res) => {
                match serde_json::from_str::<Self>(res.to_str().unwrap()) {
                    Err(error) => err(ForwardingError::Forbidden(format!(
                        "Unable to check user identity due: {error}",
                    ))),
                    Ok(res) => ok(res),
                }
            }
        }
    }
}
