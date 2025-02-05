use crate::{
    dtos::MyceliumProfileData,
    middleware::check_credentials_with_multi_identity_provider,
};

use actix_web::{web, HttpRequest};
use base64::{engine::general_purpose, Engine};
use hex;
use myc_core::{
    domain::{
        dtos::route_type::PermissionedRoles,
        entities::{KVArtifactRead, KVArtifactWrite},
    },
    use_cases::service::profile::{fetch_profile_from_email, ProfileResponse},
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{responses::GatewayError, Email, Profile};
use myc_kv::repositories::KVAppModule;
use mycelium_base::entities::FetchResponseKind;
use openssl::sha::Sha256;
use shaku::HasComponent;
use std::vec;
use uuid::Uuid;

/// Try to populate profile to request header
///
/// This function is auxiliary of the MyceliumProfileData struct used to extract
/// the Mycelium Profile from the request on mycelium native APIs.
#[tracing::instrument(name = "fetch_profile_from_request", skip(req))]
pub(crate) async fn fetch_profile_from_request(
    req: HttpRequest,
    tenant: Option<Uuid>,
    roles: Option<Vec<String>>,
    permissioned_roles: Option<PermissionedRoles>,
) -> Result<MyceliumProfileData, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Fetch email from request
    // ? -----------------------------------------------------------------------

    let email =
        check_credentials_with_multi_identity_provider(req.clone()).await?;

    // ? -----------------------------------------------------------------------
    // ? Try to fetch profile from cache
    // ? -----------------------------------------------------------------------

    let search_key = hash_profile_request(
        email.to_owned(),
        tenant,
        roles.to_owned(),
        permissioned_roles.to_owned(),
    );

    if let Some(profile) =
        fetch_profile_from_cache(search_key.to_owned(), req.clone()).await?
    {
        tracing::trace!("Profile: {:?}", profile.profile_redacted());

        return Ok(MyceliumProfileData::from_profile(profile));
    }

    // ? -----------------------------------------------------------------------
    // ? Fetch profile from datastore
    // ? -----------------------------------------------------------------------

    let profile = fetch_profile_from_datastore(
        req.clone(),
        email.to_owned(),
        tenant,
        roles.to_owned(),
        permissioned_roles.to_owned(),
    )
    .await?
    .ok_or_else(|| {
        tracing::warn!("Profile not found in datastore");

        GatewayError::Forbidden("Profile not found in datastore".to_string())
    })?;

    // ? -----------------------------------------------------------------------
    // ? Cache profile
    // ? -----------------------------------------------------------------------

    cache_profile(search_key, profile.clone(), req.clone()).await?;

    tracing::trace!("Profile: {:?}", profile.profile_redacted());

    Ok(MyceliumProfileData::from_profile(profile))
}

/// Generate a hash of the profile request
///
/// The hash should be used to identify the profile request in the cache. The
/// hash is generated from the email, tenant, roles and permissioned roles
///
#[tracing::instrument(name = "hash_profile_request", skip_all)]
fn hash_profile_request(
    email: Email,
    tenant: Option<Uuid>,
    roles: Option<Vec<String>>,
    permissioned_roles: Option<PermissionedRoles>,
) -> String {
    let email = email.email();
    let email_based_uuid = Uuid::new_v3(&Uuid::NAMESPACE_DNS, email.as_bytes());

    //
    // Initialize the hasher
    //
    let mut hasher = Sha256::new();
    hasher.update(email.as_bytes());

    //
    // If tenant is None, generate a hash from the email
    //
    let tenant = tenant.unwrap_or_else(|| email_based_uuid);
    hasher.update(tenant.as_bytes());

    //
    // If roles is None, generate a hash from the email
    //
    let roles = roles
        .unwrap_or_else(|| vec![email_based_uuid.to_string()])
        .join("");

    hasher.update(roles.as_bytes());

    //
    // If permissioned roles is None, generate a hash from the email
    //
    let permissioned_roles = if let Some(permissioned_roles) =
        permissioned_roles
    {
        permissioned_roles
            .iter()
            .map(|(role, permission)| {
                format!("{role}:{permission}", permission = permission.to_i32())
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        email_based_uuid.to_string()
    };

    hasher.update(permissioned_roles.as_bytes());

    hex::encode(hasher.finish())
}

/// Fetch profile from cache
///
/// This function is used to fetch the profile from the cache. If the profile is
/// not found, the function returns `None`.
///
#[tracing::instrument(name = "fetch_profile_from_cache", skip_all)]
async fn fetch_profile_from_cache(
    search_key: String,
    req: HttpRequest,
) -> Result<Option<Profile>, GatewayError> {
    let app_module =
        req.app_data::<web::Data<KVAppModule>>().ok_or_else(|| {
            tracing::error!(
                "Unable to extract profile fetching module from request"
            );

            GatewayError::InternalServerError(
                "Unexpected error on get profile".to_string(),
            )
        })?;

    let kv_artifact_read: &dyn KVArtifactRead = app_module.resolve_ref();

    let profile_base64 = match kv_artifact_read
        .get_encoded_artifact(search_key)
        .await
        .map_err(|err| {
            tracing::warn!(
                "Unexpected error on fetch profile from cache: {err}"
            );

            GatewayError::InternalServerError(
                "Unexpected error on fetch profile from cache.".to_string(),
            )
        })? {
        FetchResponseKind::NotFound(_) => return Ok(None),
        FetchResponseKind::Found(payload) => payload,
    };

    let profile_str = general_purpose::STANDARD
        .decode(profile_base64)
        .map_err(|err| {
            tracing::warn!(
                "Unexpected error on fetch profile from cache: {err}"
            );

            GatewayError::InternalServerError(
                "Unexpected error on fetch profile from cache.".to_string(),
            )
        })?;

    serde_json::from_slice(&profile_str)
        .map(|profile| Some(profile))
        .map_err(|err| {
            tracing::warn!(
                "Unexpected error on fetch profile from cache: {err}"
            );

            GatewayError::InternalServerError(
                "Unexpected error on fetch profile from cache.".to_string(),
            )
        })
}

/// Fetch profile from datastore
///
/// This function is used to fetch the profile from the datastore. If the profile
/// is not found, the function returns `None`.
///
#[tracing::instrument(name = "fetch_profile_from_datastore", skip_all)]
async fn fetch_profile_from_datastore(
    req: HttpRequest,
    email: Email,
    tenant: Option<Uuid>,
    roles: Option<Vec<String>>,
    permissioned_roles: Option<PermissionedRoles>,
) -> Result<Option<Profile>, GatewayError> {
    let app_module =
        req.app_data::<web::Data<SqlAppModule>>().ok_or_else(|| {
            tracing::error!(
                "Unable to extract profile fetching module from request"
            );

            GatewayError::InternalServerError(
                "Unexpected error on get profile".to_string(),
            )
        })?;

    match fetch_profile_from_email(
        email.to_owned(),
        None,
        tenant,
        roles,
        permissioned_roles,
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    .map_err(|err| {
        tracing::warn!("Unexpected error on fetch profile from email: {err}");

        GatewayError::InternalServerError(
            "Unexpected error on fetch profile from email.".to_string(),
        )
    })? {
        ProfileResponse::RegisteredUser(res) => Ok(Some(res)),
        ProfileResponse::UnregisteredUser(email) => {
            return Err(GatewayError::Forbidden(format!(
                "Unauthorized access: {email}",
                email = email.email(),
            )))
        }
    }
}

/// Cache profile
///
/// This function is used to cache the profile in the cache.
///
#[tracing::instrument(name = "cache_profile", skip_all)]
async fn cache_profile(
    search_key: String,
    profile: Profile,
    req: HttpRequest,
) -> Result<(), GatewayError> {
    let app_module =
        req.app_data::<web::Data<KVAppModule>>().ok_or_else(|| {
            tracing::error!(
                "Unable to extract profile caching module from request"
            );

            GatewayError::InternalServerError(
                "Unexpected error on cache profile".to_string(),
            )
        })?;

    let kv_artifact_write: &dyn KVArtifactWrite = app_module.resolve_ref();

    let serialized_profile =
        serde_json::to_string(&profile).map_err(|err| {
            tracing::warn!("Unexpected error on serialize profile: {err}");

            GatewayError::InternalServerError(
                "Unexpected error on serialize profile.".to_string(),
            )
        })?;

    let encoded_profile =
        general_purpose::STANDARD.encode(serialized_profile.as_bytes());

    kv_artifact_write
        .set_encoded_artifact(search_key, encoded_profile)
        .await
        .map_err(|err| {
            tracing::warn!("Unexpected error on cache profile: {err}");

            GatewayError::InternalServerError(
                "Unexpected error on cache profile.".to_string(),
            )
        })?;

    Ok(())
}
