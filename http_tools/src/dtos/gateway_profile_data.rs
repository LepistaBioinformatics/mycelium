use crate::{
    middleware::fetch_profile_from_token, responses::GatewayError, Profile,
};

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::Future;
use log::warn;
use myc_core::{
    domain::dtos::{
        account::VerboseStatus,
        profile::{LicensedResources, Owner},
    },
    settings::DEFAULT_PROFILE_KEY,
};
use serde::Deserialize;
use std::{pin::Pin, str};
use uuid::Uuid;

/// The Profile data extractor from requests
///
/// A Gateway Profile Data is used to extract the Profile from requests
/// delivered to the gateway downstream services.
///
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayProfileData {
    pub owners: Vec<Owner>,
    pub acc_id: Uuid,
    pub is_subscription: bool,
    pub is_manager: bool,
    pub is_staff: bool,
    pub owner_is_active: bool,
    pub account_is_active: bool,
    pub account_was_approved: bool,
    pub account_was_archived: bool,
    pub verbose_status: Option<VerboseStatus>,
    pub licensed_resources: Option<LicensedResources>,
}

impl GatewayProfileData {
    pub fn from_profile(profile: Profile) -> Self {
        Self {
            owners: profile.owners,
            acc_id: profile.acc_id,
            is_subscription: profile.is_subscription,
            is_manager: profile.is_manager,
            is_staff: profile.is_staff,
            owner_is_active: profile.owner_is_active,
            account_is_active: profile.account_is_active,
            account_was_approved: profile.account_was_approved,
            account_was_archived: profile.account_was_archived,
            verbose_status: profile.verbose_status,
            licensed_resources: profile.licensed_resources,
        }
    }

    pub fn to_profile(&self) -> Profile {
        Profile {
            owners: self.owners.to_owned(),
            acc_id: self.acc_id,
            is_subscription: self.is_subscription,
            is_manager: self.is_manager,
            is_staff: self.is_staff,
            owner_is_active: self.owner_is_active,
            account_is_active: self.account_is_active,
            account_was_approved: self.account_was_approved,
            account_was_archived: self.account_was_archived,
            verbose_status: self.verbose_status.to_owned(),
            licensed_resources: self.licensed_resources.to_owned(),
        }
    }
}

impl FromRequest for GatewayProfileData {
    type Error = GatewayError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        //
        // Try to extract profile from header
        //
        match req.headers().get(DEFAULT_PROFILE_KEY) {
            Some(res) => {
                let unwrapped_response = match str::from_utf8(res.as_bytes()) {
                    Ok(res) => res,
                    Err(err) => {
                        warn!("Unable to check user identity due: {}", err);

                        return Box::pin(async move {
                            Err(GatewayError::Unauthorized(
                                "Unable to check user identity. Please contact administrators".to_string(),
                            ))
                        });
                    }
                };

                match serde_json::from_str::<Self>(unwrapped_response) {
                    Err(error) => {
                        return Box::pin(async move {
                            Err(GatewayError::Unauthorized(format!(
                                "Unable to check user identity due: {error}",
                            )))
                        })
                    }
                    Ok(res) => return Box::pin(async move { Ok(res) }),
                }
            }
            None => (),
        };
        //
        // Try to extract authorization token from header and try to get profile
        // from profile service.
        //
        match req.headers().get("Authorization") {
            Some(res) => {
                let token = res.to_str().unwrap().to_string();

                return Box::pin(async move {
                    fetch_profile_from_token(token).await
                });
            }
            None => (),
        };
        //
        // Return a default forbidden response if the user identity could not be
        // checked.
        //
        Box::pin(async move {
            Err(GatewayError::Unauthorized(
                "Unable to check user identity. Please contact administrators"
                    .to_string(),
            ))
        })
    }
}
