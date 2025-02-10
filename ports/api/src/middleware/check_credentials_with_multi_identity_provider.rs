use crate::{dtos::JWKS, middleware::get_email_or_provider_from_request};

use crate::{
    dtos::{Audience, GenericAccessTokenClaims, JWKS},
    middleware::get_email_or_provider_from_request,
    models::api_config::ApiConfig,
};

use crate::{
    dtos::{Audience, GenericIDTokenClaims, JWKS},
    middleware::get_email_or_provider_from_request,
};

use actix_web::{web, HttpRequest};
use base64::{engine::general_purpose, Engine};
use cached::proc_macro::io_cached;
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use myc_core::domain::{
    dtos::email::Email,
    entities::{KVArtifactRead, KVArtifactWrite},
};
use myc_http_tools::{
    models::external_providers_config::ExternalProviderConfig,
    responses::GatewayError,
};
use myc_kv::repositories::KVAppModule;
use mycelium_base::entities::FetchResponseKind;
use openssl::{stack::Stack, x509::X509};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use shaku::HasComponent;

#[derive(Deserialize)]
struct UserInfo {
    email: Option<String>,
}

/// Try to populate profile to request header
///
/// This function is used to check credentials from multiple identity providers.
#[tracing::instrument(
    name = "check_credentials_with_multi_identity_provider",
    skip_all
)]
pub(crate) async fn check_credentials_with_multi_identity_provider(
    req: HttpRequest,
) -> Result<Email, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Extract issuer and token from request
    //
    // If the function parse_issuer_from_request_v2 found an valid email,
    // indicate that this found a internal provider. Otherwise, the function
    // will return a vector of external providers. If the internal and external
    // providers are not found, the function will return an Unauthorized error.
    //
    // ? -----------------------------------------------------------------------

    let (optional_email, optional_provider, token) =
        get_email_or_provider_from_request(req.clone()).await?;

    // ? -----------------------------------------------------------------------
    // ? If email is found, return it
    //
    // An email response indicates that the request is coming from the internal
    // provider. Then, the function will return the email.
    //
    // ? -----------------------------------------------------------------------

    if let Some(email) = optional_email {
        return Ok(email);
    }

    // ? -----------------------------------------------------------------------
    // ? Proceed to the external providers
    //
    // If the email is not found, the function will proceed to the external
    // providers.
    //
    // ? -----------------------------------------------------------------------

    if let Some(provider) = optional_provider {
        return get_email_from_external_provider(&provider, &token, &req).await;
    }

    // ? -----------------------------------------------------------------------
    // ? If no provider is found, return an error
    // ? -----------------------------------------------------------------------

    Err(GatewayError::Unauthorized(
        "Could not check issuer.".to_string(),
    ))
}

/// Attempt to extract the `kid`-claim o
///
/// This function is used to fetch the JWKS from the given URI.
#[io_cached(
    disk = true,
    time = 3600,
    map_error = r##"|e| GatewayError::InternalServerError(format!("{e}", e = e))"##
)]
#[tracing::instrument(name = "fetch_jwks", skip_all)]
async fn fetch_jwks(uri: &str) -> Result<JWKS, GatewayError> {
    let res = reqwest::get(uri).await.map_err(|e| {
        tracing::error!("Error fetching JWKS: {}", e);

        GatewayError::InternalServerError(
            "Unexpected error on fetch JWKS".to_string(),
        )
    })?;

    let val = res.json::<JWKS>().await.map_err(|e| {
        tracing::error!("Error parsing JWKS: {}", e);

        GatewayError::InternalServerError(
            "Unexpected error on parse JWKS".to_string(),
        )
    })?;

    return Ok(val);
}

#[tracing::instrument(name = "get_email_from_external_provider", skip_all)]
async fn get_email_from_external_provider(
    provider: &ExternalProviderConfig,
    token: &str,
    req: &HttpRequest,
) -> Result<Email, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Collect public keys from provider
    //
    // The public keys are collected from the provider. If the public keys are
    // not found, return an error. Public keys should be used to verify the
    // token signature.
    //
    // ? -----------------------------------------------------------------------

    let jwks_uri =
        provider.jwks_uri.async_get_or_error().await.map_err(|e| {
            GatewayError::InternalServerError(format!(
                "Error fetching JWKS: {e}"
            ))
        })?;

    //
    // Extract kid from token
    //
    let decoded_headers = decode_header(&token).map_err(|err| {
        tracing::error!("Error decoding header: {err}");

        GatewayError::Unauthorized(format!(
            "JWT token has not valid format. Unable to decode header: {token}"
        ))
    })?;

    //
    // Extract kid from token
    //
    let kid =
        decoded_headers
            .kid
            .ok_or(GatewayError::Unauthorized(format!(
                "JWT kid not found: {token}"
            )))?;

    //
    // Find JWK in JWKS
    //
    let jwks = fetch_jwks(&jwks_uri).await?;
    let jwk = jwks.find(&kid).ok_or(GatewayError::Unauthorized(format!(
        "JWT kid not found in JWKS: {kid}"
    )))?;

    // ? -----------------------------------------------------------------------
    // ? Start token verification
    //
    // The token verification is performed using the public key collected from
    // the provider. If the public key is not found, return an error.
    //
    // ? -----------------------------------------------------------------------

    let decoded_key = if let Some(x5c) = &jwk.x5c {
        //
        // Case token is signed with X5C perform the verification of the token
        // using the root certificate
        //
        let mut certs = Stack::new().map_err(|err| {
            tracing::error!("Error on create stack: {err}");

            GatewayError::Unauthorized("Error on parse token".to_string())
        })?;

        for cert in x5c {
            let cert_der =
                general_purpose::STANDARD.decode(cert).map_err(|err| {
                    tracing::error!("Error on decode X5C: {err}");

                    GatewayError::Unauthorized(
                        "Error on parse token".to_string(),
                    )
                })?;

            let x509 = X509::from_der(&cert_der).map_err(|err| {
                tracing::error!("Error on create X509 from der: {err}");

                GatewayError::Unauthorized("Error on parse token".to_string())
            })?;

            certs.push(x509).map_err(|err| {
                tracing::error!("Error on push X509 to stack: {err}");

                GatewayError::Unauthorized("Error on parse token".to_string())
            })?;
        }

        let root_cert = certs.get(0).ok_or(GatewayError::Unauthorized(
            "No certificates found".to_string(),
        ))?;

        let public_key = root_cert.public_key().map_err(|err| {
            tracing::error!("Error getting public key: {err}");

            GatewayError::Unauthorized("Error on parse token".to_string())
        })?;

        let leaf_cert =
            certs
                .get(certs.len() - 1)
                .ok_or(GatewayError::Unauthorized(
                    "No leaf certificate found".to_string(),
                ))?;

        leaf_cert.verify(public_key.as_ref()).map_err(|err| {
            tracing::error!("Error on verify X509: {err}");

            GatewayError::Unauthorized("Error on parse token".to_string())
        })?;

        let public_key_pem = public_key.public_key_to_pem().map_err(|err| {
            tracing::error!("Error on generate public key pem from X5C: {err}");

            GatewayError::Unauthorized("Error on parse token".to_string())
        })?;

        DecodingKey::from_rsa_pem(&public_key_pem).map_err(|err| {
            tracing::error!("Error on create RSA decoding key: {err}");

            GatewayError::Unauthorized("Error on parse token".to_string())
        })?
    } else {
        //
        // Case token is signed with RS256 perform the verification of the token
        // using the RSA components
        //
        DecodingKey::from_rsa_components(&jwk.n, &jwk.e).map_err(|err| {
            tracing::error!("Error creating RSA decoding key: {err}");

            GatewayError::Unauthorized("Error on parse token".to_string())
        })?
    };

    //
    // Decode token
    //
    let token_data = decode::<GenericAccessTokenClaims>(
        &token,
        &decoded_key,
        &Validation::new(decoded_headers.alg),
    )
    .map_err(|err| {
        tracing::error!("Error decoding token: {err}");

        GatewayError::Unauthorized("Error on parse token".to_string())
    })?;

    //
    // Extract expected audience from issuer v2
    //
    let expected_audience =
        provider.audience.async_get_or_error().await.map_err(|e| {
            tracing::error!("Error getting audience: {e}");

            GatewayError::Unauthorized("JWT audience not found".to_string())
        })?;

    //
    // Validate audience
    //
    match token_data.claims.audience.to_owned() {
        Audience::Single(aud) => {
            if aud != expected_audience {
                tracing::trace!("Expected audience: {:?}", expected_audience);
                tracing::trace!("Token audience: {:?}", aud);

                return Err(GatewayError::Unauthorized(format!(
                    "Invalid audience: {expected_audience}"
                )));
            }
        }
        Audience::Multiple(auds) => {
            if !auds.contains(&expected_audience) {
                tracing::trace!("Expected audience: {:?}", expected_audience);
                tracing::trace!("Token audience: {:?}", auds);

                return Err(GatewayError::Unauthorized(format!(
                    "Invalid audience: {expected_audience}"
                )));
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Try to extract email from token
    //
    // In some the claims must include the email, upn or unique_name. Try to
    // extract the email from the token. If the email is not found, return an
    // error.
    //
    // ? -----------------------------------------------------------------------

    let token_email = {
        if let Some(email) = token_data.claims.email {
            Some(email)
        } else {
            None
        }
    };

    if let Some(email) = token_email {
        let parsed_email = Email::from_string(email).map_err(|err| {
            tracing::error!("Error on extract email from token: {err}");

            GatewayError::Unauthorized(
                "Error on extract email from token".to_string(),
            )
        })?;

        return Ok(parsed_email);
    };

    // ? -----------------------------------------------------------------------
    // ? Try to extract email from user info url
    //
    // Try to request the user info from the declared user info url. If the
    // user info is not found, return an error.
    //
    // ? -----------------------------------------------------------------------

    let token_identifier =
        if let Some(jid) = token_data.claims.json_web_token_id {
            jid
        } else {
            format!(
                "{sub}_{iat}",
                sub = token_data.claims.subject,
                iat = token_data.claims.issued_at
            )
        };

    if let Some(user_info_url) = &provider.user_info_url {
        let user_info_url =
            user_info_url.async_get_or_error().await.map_err(|e| {
                GatewayError::InternalServerError(format!(
                    "Error getting user info url: {e}"
                ))
            })?;

        let email = get_user_info_from_url(
            &user_info_url,
            token,
            token_identifier.to_owned(),
            req,
        )
        .await?;

        if let Some(email) = email {
            return Ok(email);
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Try to extract user info url the authority
    // ? -----------------------------------------------------------------------

    if let Audience::Multiple(auds) = token_data.claims.audience {
        if let Some(user_info_url) =
            auds.iter().find(|aud| aud.ends_with("/userinfo"))
        {
            let email = get_user_info_from_url(
                &user_info_url,
                token,
                token_identifier,
                req,
            )
            .await?;

            if let Some(email) = email {
                return Ok(email);
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ? If no email is found, return an error
    // ? -----------------------------------------------------------------------

    Err(GatewayError::Unauthorized("Email not found".to_string()))
}

#[tracing::instrument(name = "fetch_email_from_cache", skip_all)]
async fn fetch_email_from_cache(
    token_identifier: String,
    req: &HttpRequest,
) -> Option<Email> {
    let app_module = match req.app_data::<web::Data<KVAppModule>>() {
        Some(app_module) => app_module,
        None => {
            tracing::error!(
                "Unable to extract profile fetching module from request"
            );

            return None;
        }
    };

    let kv_artifact_read: &dyn KVArtifactRead = app_module.resolve_ref();

    let profile_response = match kv_artifact_read
        .get_encoded_artifact(token_identifier)
        .await
    {
        Err(err) => {
            tracing::error!(
                "Unexpected error on fetch profile from cache: {err}"
            );

            return None;
        }
        Ok(res) => res,
    };

    let profile_base64 = match profile_response {
        FetchResponseKind::NotFound(_) => return None,
        FetchResponseKind::Found(payload) => payload,
    };

    let profile_slice = match general_purpose::STANDARD.decode(profile_base64) {
        Ok(res) => res,
        Err(err) => {
            tracing::warn!(
                "Unexpected error on fetch profile from cache: {err}"
            );

            return None;
        }
    };

    match serde_json::from_slice::<Email>(&profile_slice) {
        Ok(email) => {
            tracing::trace!("Cache email: {:?}", email.redacted_email());

            Some(email)
        }
        Err(err) => {
            tracing::warn!(
                "Unexpected error on fetch profile from cache: {err}"
            );

            return None;
        }
    }
}

#[tracing::instrument(name = "set_email_in_cache", skip_all)]
async fn set_email_in_cache(
    token_identifier: String,
    email: Email,
    req: &HttpRequest,
) {
    let app_module = match req.app_data::<web::Data<KVAppModule>>() {
        Some(app_module) => app_module,
        None => {
            tracing::error!(
                "Unable to extract profile caching module from request"
            );

            return;
        }
    };

    let ttl = if let Some(cache_ttl) = req.app_data::<web::Data<ApiConfig>>() {
        cache_ttl.cache_ttl.unwrap_or(60)
    } else {
        60
    };

    let kv_artifact_write: &dyn KVArtifactWrite = app_module.resolve_ref();

    let serialized_email = match serde_json::to_string(&email) {
        Ok(serialized_email) => serialized_email,
        Err(err) => {
            tracing::error!("Unexpected error on serialize email: {err}");

            return;
        }
    };

    let encoded_email =
        general_purpose::STANDARD.encode(serialized_email.as_bytes());

    match kv_artifact_write
        .set_encoded_artifact(token_identifier, encoded_email, ttl)
        .await
    {
        Ok(_) => (),
        Err(err) => {
            tracing::error!("Unexpected error on cache profile: {err}");

            return;
        }
    }
}

async fn get_user_info_from_url(
    user_info_url: &str,
    token: &str,
    token_identifier: String,
    req: &HttpRequest,
) -> Result<Option<Email>, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Try to fetch email from cache
    //
    // If the email is found in the cache, return it. Otherwise, proceed to the
    // user info url request.
    //
    // ? -----------------------------------------------------------------------

    let email = fetch_email_from_cache(token_identifier.to_owned(), req).await;

    if let Some(email) = email {
        return Ok(Some(email));
    }

    // ? -----------------------------------------------------------------------
    // ? Request user info url
    //
    // Request the user info url and extract the email from the response. If the
    // email is not found, return an error.
    //
    // ? -----------------------------------------------------------------------

    let res = reqwest::Client::new()
        .get(user_info_url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Error fetching user info url: {e}");

            GatewayError::Unauthorized(
                "Error on fetch user info url".to_string(),
            )
        })?;

    let user_info = res.json::<UserInfo>().await.map_err(|e| {
        tracing::error!("Error parsing user info url: {e}");

        GatewayError::Unauthorized("Error on parse user info url".to_string())
    })?;

    let email = user_info.email.ok_or(GatewayError::Unauthorized(
        "Email not found in user info".to_string(),
    ))?;

    let parsed_email = Email::from_string(email).map_err(|err| {
        tracing::error!("Error on extract email from token: {err}");

        GatewayError::Unauthorized(
            "Error on extract email from token".to_string(),
        )
    })?;

    // ? -----------------------------------------------------------------------
    // ? Cache email
    //
    // Cache the email in the cache.
    //
    // ? -----------------------------------------------------------------------

    set_email_in_cache(token_identifier, parsed_email.to_owned(), req).await;

    // ? -----------------------------------------------------------------------
    // ? Return email
    //
    // Return the email found in the user info url.
    //
    // ? -----------------------------------------------------------------------

    Ok(Some(parsed_email))
}
