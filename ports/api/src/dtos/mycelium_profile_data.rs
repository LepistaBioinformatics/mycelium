use crate::middleware::fetch_profile_from_request;

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::Future;
use myc_core::domain::dtos::{
    account::VerboseStatus,
    profile::{LicensedResources, Owner},
};
use myc_http_tools::{
    responses::GatewayError, settings::DEFAULT_MYCELIUM_ROLE_KEY, Profile,
};
use serde::Deserialize;
use std::pin::Pin;
use tracing::error;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MyceliumProfileData {
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

impl MyceliumProfileData {
    pub(crate) fn from_profile(profile: Profile) -> Self {
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

    pub(crate) fn to_profile(&self) -> Profile {
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

impl FromRequest for MyceliumProfileData {
    type Error = GatewayError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req_clone = req.clone();

        //
        // Get the roles from the request
        //
        let roles: Option<Vec<String>> =
            match req_clone.headers().get(DEFAULT_MYCELIUM_ROLE_KEY) {
                Some(roles) => {
                    let roles: Option<Vec<String>> =
                        match serde_json::from_str(roles.to_str().unwrap()) {
                            Ok(roles) => roles,
                            Err(err) => {
                                error!("Failed to parse roles: {err}");

                                None
                            }
                        };

                    if let Some(roles) = roles {
                        if roles.is_empty() {
                            None
                        } else {
                            Some(roles)
                        }
                    } else {
                        None
                    }
                }
                None => None,
            };

        Box::pin(async move {
            fetch_profile_from_request(req_clone, roles, None).await
        })
    }
}
