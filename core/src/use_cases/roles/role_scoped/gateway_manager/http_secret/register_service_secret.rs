use crate::domain::dtos::profile::Profile;

/// Register a new HTTP secret.
///
/// Http secrets should be used to store sensitive information that should be
/// used during the api Gateway downstream authentication.
///
#[tracing::instrument(name = "register_service_secret", skip_all)]
pub async fn register_service_secret(profile: Profile) {}
