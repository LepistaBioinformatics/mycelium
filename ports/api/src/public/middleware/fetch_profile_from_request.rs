use crate::{
    providers::check_credentials, responses::GatewayError, LicensedResources,
    Profile,
};

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::Future;
use log::{debug, warn};
use myc_core::{
    domain::dtos::account::VerboseProfileStatus,
    use_cases::roles::service::profile::{
        fetch_profile_from_email, ProfileResponse,
    },
};
use myc_prisma::repositories::{
    LicensedResourcesFetchingSqlDbRepository, ProfileFetchingSqlDbRepository,
};
use serde::Deserialize;
use std::pin::Pin;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MyceliumProfileData {
    pub email: String,
    pub current_account_id: Uuid,
    pub is_subscription: bool,
    pub is_manager: bool,
    pub is_staff: bool,
    pub owner_is_active: bool,
    pub account_is_active: bool,
    pub account_was_approved: bool,
    pub account_was_archived: bool,
    pub verbose_status: Option<VerboseProfileStatus>,
    pub licensed_resources: Option<Vec<LicensedResources>>,
}

impl MyceliumProfileData {
    pub fn from_profile(profile: Profile) -> Self {
        Self {
            email: profile.email,
            current_account_id: profile.current_account_id,
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
            email: self.email.to_owned(),
            current_account_id: self.current_account_id,
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

        Box::pin(async move { fetch_profile_from_request(req_clone).await })
    }
}

/// Try to populate profile to request header
pub(super) async fn fetch_profile_from_request(
    req: HttpRequest,
) -> Result<MyceliumProfileData, GatewayError> {
    let email = match check_credentials(req.to_owned()).await {
        Err(err) => {
            warn!("{:?}", err);
            return Err(GatewayError::Forbidden(format!("{err}")));
        }
        Ok(res) => {
            debug!("Requesting Email: {:?}", res);

            Some(res)
        }
    };

    if email.is_none() {
        return Err(GatewayError::Forbidden(format!(
            "Unable o extract user identity from request."
        )));
    }

    let profile = match fetch_profile_from_email(
        email.to_owned().unwrap(),
        Box::new(&ProfileFetchingSqlDbRepository {}),
        Box::new(&LicensedResourcesFetchingSqlDbRepository {}),
    )
    .await
    {
        Err(err) => {
            warn!("{:?}", err);
            return Err(GatewayError::InternalServerError(format!("{err}")));
        }
        Ok(res) => {
            debug!("Requesting Profile: {:?}", res);

            match res {
                ProfileResponse::UnregisteredUser(email) => {
                    return Err(GatewayError::Forbidden(format!(
                        "Unauthorized access: {:?}",
                        email,
                    )))
                }
                ProfileResponse::RegisteredUser(res) => res,
            }
        }
    };

    Ok(MyceliumProfileData::from_profile(profile))
}
