use crate::middleware::fetch_profile_from_request;

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::Future;
use myc_core::domain::dtos::{
    account::VerboseStatus,
    profile::{LicensedResources, Owner, TenantsOwnership},
};
use myc_http_tools::{
    responses::GatewayError,
    settings::{DEFAULT_MYCELIUM_ROLE_KEY, DEFAULT_TENANT_ID_KEY},
    Profile,
};
use serde::Deserialize;
use std::pin::Pin;
use tracing::{error, trace};
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
    pub tenants_ownership: Option<TenantsOwnership>,
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
            tenants_ownership: profile.tenants_ownership,
        }
    }

    pub(crate) fn to_profile(&self) -> Profile {
        Profile::new(
            self.owners.to_owned(),
            self.acc_id,
            self.is_subscription,
            self.is_manager,
            self.is_staff,
            self.owner_is_active,
            self.account_is_active,
            self.account_was_approved,
            self.account_was_archived,
            self.verbose_status.to_owned(),
            self.licensed_resources.to_owned(),
            self.tenants_ownership.to_owned(),
        )
    }
}

impl FromRequest for MyceliumProfileData {
    type Error = GatewayError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    #[tracing::instrument(name = "mycelium_profile_from_request", skip_all)]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req_clone = req.clone();

        //
        // Get the tenant from the request
        //
        let tenant = match req.headers().get(DEFAULT_TENANT_ID_KEY) {
            Some(tenant) => match tenant.to_str() {
                Ok(tenant) => match Uuid::parse_str(tenant) {
                    Ok(tenant_uuid) => Some(tenant_uuid),
                    Err(err) => {
                        error!("Failed to parse tenant: {err}");
                        None
                    }
                },
                Err(err) => {
                    error!("Failed to parse tenant: {err}");

                    None
                }
            },
            None => None,
        };

        if let Some(tenant) = tenant {
            trace!("Requested tenant: {:?}", tenant);
        }

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

        if let Some(roles) = roles.to_owned() {
            trace!("Requested roles: {:?}", roles);
        }

        Box::pin(async move {
            fetch_profile_from_request(req_clone, tenant, roles, None).await
        })
    }
}
