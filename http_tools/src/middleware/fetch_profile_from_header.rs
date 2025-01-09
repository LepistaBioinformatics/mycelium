use crate::{
    dtos::gateway_profile_data::GatewayProfileData, responses::GatewayError,
    settings::PROFILE_FETCHING_URL,
};

use myc_core::domain::entities::ProfileFetching;
use myc_svc::repositories::ProfileFetchingSvcRepo;
use mycelium_base::entities::FetchResponseKind;

pub(crate) async fn fetch_profile_from_token(
    token: String,
) -> Result<GatewayProfileData, GatewayError> {
    let repo = ProfileFetchingSvcRepo {
        url: PROFILE_FETCHING_URL.to_string(),
    };

    match repo.get_from_token(token.to_string()).await {
        Err(err) => Err(GatewayError::InternalServerError(err.to_string())),
        Ok(res) => match res {
            FetchResponseKind::NotFound(email) => {
                Err(GatewayError::Forbidden(email.unwrap_or("".to_string())))
            }
            FetchResponseKind::Found(profile) => {
                Ok(GatewayProfileData::from_profile(profile))
            }
        },
    }
}
