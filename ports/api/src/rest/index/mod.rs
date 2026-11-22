pub(crate) mod app_public_config_endpoints;
pub(crate) mod heath_check_endpoints;
#[cfg(feature = "federation")]
pub(crate) mod instance_self_description_endpoints;
pub(crate) mod profile_signing_jwks_endpoints;
#[cfg(feature = "federation")]
pub(crate) mod role_catalog_endpoints;
