use crate::{
    functions::decode_and_decompress_profile_from_base64,
    middleware::fetch_profile_from_token, responses::GatewayError,
    settings::DEFAULT_PROFILE_KEY, Profile,
};

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::Future;
use myc_core::domain::dtos::{
    account::{AccountMetaKey, VerboseStatus},
    profile::{LicensedResources, Owner, TenantsOwnership},
};
use serde::Deserialize;
use std::{collections::HashMap, pin::Pin, str};
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
    pub account_was_deleted: bool,
    pub verbose_status: Option<VerboseStatus>,
    pub licensed_resources: Option<LicensedResources>,
    pub tenants_ownership: Option<TenantsOwnership>,
    pub meta: Option<HashMap<AccountMetaKey, String>>,
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
            account_was_deleted: profile.account_was_deleted,
            verbose_status: profile.verbose_status,
            licensed_resources: profile.licensed_resources,
            tenants_ownership: profile.tenants_ownership,
            meta: profile.meta,
        }
    }

    pub fn to_profile(&self) -> Profile {
        let mut profile = Profile::new(
            self.owners.to_owned(),
            self.acc_id,
            self.is_subscription,
            self.is_manager,
            self.is_staff,
            self.owner_is_active,
            self.account_is_active,
            self.account_was_approved,
            self.account_was_archived,
            self.account_was_deleted,
            self.verbose_status.to_owned(),
            self.licensed_resources.to_owned(),
            self.tenants_ownership.to_owned(),
        );

        profile.meta = self.meta.to_owned();

        profile
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
                        tracing::warn!(
                            "Unable to check user identity due: {err}"
                        );

                        return Box::pin(async move {
                            Err(GatewayError::Unauthorized(
                                "Unable to check user identity. Please contact administrators".to_string(),
                            ))
                        });
                    }
                };

                match decode_and_decompress_profile_from_base64(
                    unwrapped_response.to_string(),
                ) {
                    Ok(profile) => GatewayProfileData::from_profile(profile),
                    Err(e) => {
                        tracing::warn!(
                            "Unable to decode and decompress profile due: {e}"
                        );

                        return Box::pin(async move {
                            Err(GatewayError::Unauthorized("Unable to check user identity. Please contact administrators".to_string()))
                        });
                    }
                };
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
