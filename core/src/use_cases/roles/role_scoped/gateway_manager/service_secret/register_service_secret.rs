use crate::domain::dtos::{profile::Profile, service_secret::ServiceSecret};

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};

/// Register a new HTTP secret.
///
/// Http secrets should be used to store sensitive information that should be
/// used during the api Gateway downstream authentication.
///
#[tracing::instrument(name = "register_service_secret", skip_all)]
pub async fn register_service_secret(
    profile: Profile,
) -> Result<CreateResponseKind<ServiceSecret>, MappedErrors> {
    unimplemented!("register_service_secret")
}
