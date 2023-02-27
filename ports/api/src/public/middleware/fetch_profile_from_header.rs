use crate::{
    responses::ForwardingError, settings::PROFILE_FETCHING_URL,
    LicensedResources, Profile,
};

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use clean_base::entities::default_response::FetchResponseKind;
use futures::Future;
use myc_core::{
    domain::entities::ProfileFetching, settings::DEFAULT_PROFILE_KEY,
};
use myc_svc::repositories::ProfileFetchingSvcRepo;
use serde::Deserialize;
use std::pin::Pin;
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
    pub account_was_archived: bool,
    pub licensed_resources: Option<Vec<LicensedResources>>,
}

impl GatewayProfileData {
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
            licensed_resources: self.licensed_resources.to_owned(),
        }
    }
}

impl FromRequest for GatewayProfileData {
    type Error = ForwardingError;
    //type Future = Ready<Result<Self, Self::Error>>;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        //
        // Try to extract profile from header
        //
        match req.headers().get(DEFAULT_PROFILE_KEY) {
            Some(res) => {
                match serde_json::from_str::<Self>(res.to_str().unwrap()) {
                    Err(error) => {
                        return Box::pin(async move {
                            Err(ForwardingError::Forbidden(format!(
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
        Box::pin(async move { Err(ForwardingError::Forbidden("".to_string())) })
    }
}

async fn fetch_profile_from_token(
    token: String,
) -> Result<GatewayProfileData, ForwardingError> {
    let repo = ProfileFetchingSvcRepo {
        url: PROFILE_FETCHING_URL.to_string(),
    };

    match repo.get(None, Some(token.to_string())).await {
        Err(err) => Err(ForwardingError::Forbidden(err.to_string())),
        Ok(res) => match res {
            FetchResponseKind::NotFound(email) => {
                Err(ForwardingError::Forbidden(email.unwrap_or("".to_string())))
            }
            FetchResponseKind::Found(profile) => {
                Ok(GatewayProfileData::from_profile(profile))
            }
        },
    }
}
