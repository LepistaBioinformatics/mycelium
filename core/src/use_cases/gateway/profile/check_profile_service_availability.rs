use std::fmt::Error;

use crate::{domain::dtos::service::ProfileService, settings::PROFILE_SERVICE};

use log::warn;

/// Check if the profile service is available.
pub async fn check_profile_service_availability(
) -> Result<ProfileService, Error> {
    let svc_config = PROFILE_SERVICE
        .lock()
        .await
        .to_owned()
        .expect("Profile Service not loaded.");

    warn!(
        "TODO: {:?}",
        "profile service checking not already implemented. Do this!"
    );

    warn!("svc_config: {:?}", svc_config);

    Ok(svc_config)
}
