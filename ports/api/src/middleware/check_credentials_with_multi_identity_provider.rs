use crate::{dtos::JWKS, middleware::parse_issuer_from_request_v2};

use actix_web::{error::ParseError, http::header::Header, web, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use base64::{engine::general_purpose, Engine};
use jsonwebtoken::{
    decode, decode_header, errors::ErrorKind, DecodingKey, Validation,
};
use myc_config::optional_config::OptionalConfig;
use myc_core::domain::dtos::email::Email;
use myc_http_tools::{
    functions::decode_jwt_hs512,
    models::{
        auth_config::AuthConfig, internal_auth_config::InternalOauthConfig,
    },
    providers::{az_check_credentials, gc_check_credentials},
    responses::GatewayError,
};
use openssl::{stack::Stack, x509::X509};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    // ? -----------------------------------------------------------------------
    // ? Microsoft claim fields
    // ? -----------------------------------------------------------------------
    /// Field `upn` is the Microsoft email address
    #[serde(rename = "upn")]
    upn: Option<String>,

    /// Field `unique_name` is the Microsoft name
    #[serde(rename = "unique_name")]
    unique_name: Option<String>,

    // ? -----------------------------------------------------------------------
    // ? Google claim fields
    // ? -----------------------------------------------------------------------
    /// Google email address
    #[serde(rename = "email")]
    email: Option<String>,

    /// Google email verified
    #[serde(rename = "email_verified")]
    email_verified: Option<bool>,

    // ? -----------------------------------------------------------------------
    // ? Other providers claim fields
    // ? -----------------------------------------------------------------------
    #[serde(flatten)]
    fields: HashMap<String, Value>,
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
) -> Result<Option<Email>, GatewayError> {
    //
    // Extract issuer and token from request
    //
    let (issuer_v2, token) = parse_issuer_from_request_v2(req.clone()).await?;

    //
    // Fetch JWKS url from issuer v2
    //
    let jwks_uri =
        issuer_v2.jwks_uri.async_get_or_error().await.map_err(|e| {
            GatewayError::InternalServerError(format!(
                "Error fetching JWKS: {e}"
            ))
        })?;

    //
    // Fetch JWKS from url collected from issuer v2
    //
    let jwks = fetch_jwks(&jwks_uri).await?;

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
    let jwk = jwks.find(&kid).ok_or(GatewayError::Unauthorized(format!(
        "JWT kid not found in JWKS: {kid}"
    )))?;

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

        // Verifying using the root certificate public key
        let root_cert = certs.get(0).ok_or(GatewayError::Unauthorized(
            "No certificates found".to_string(),
        ))?;

        let public_key = root_cert.public_key().map_err(|err| {
            tracing::error!("Error getting public key: {err}");

            GatewayError::Unauthorized("Error on parse token".to_string())
        })?;

        tracing::trace!("public_key: {:?}", public_key);

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
    // Create validation object
    //
    let mut validation = Validation::new(decoded_headers.alg);

    //
    // If the issuer is Microsoft Graph, disable signature validation
    //
    let issuer = issuer_v2
        .issuer
        .async_get_or_error()
        .await
        .map_err(|err| {
            tracing::error!("Error getting issuer: {err}");
            GatewayError::Unauthorized("JWT issuer not found".to_string())
        })?
        .to_string();

    tracing::trace!("Issuer: {:?}", issuer);

    if ["sts.windows.net", "azure-ad", "microsoft"]
        .iter()
        .any(|i| issuer.contains(i))
    {
        //
        // TODO: Remove this section after implement the signature validation
        // for Microsoft Graph
        //
        validation.insecure_disable_signature_validation();
    }

    tracing::trace!("Validation: {:?}", validation);

    //
    // Decode token
    //
    let token_data = decode::<Claims>(&token, &decoded_key, &validation)
        .map_err(|err| {
            tracing::error!("Error decoding token: {err}");

            GatewayError::Unauthorized("Error on parse token".to_string())
        })?;

    tracing::trace!("Token data: {:?}", token_data);

    //
    // Extract expected audience from issuer v2
    //
    let expected_audience =
        issuer_v2.audience.async_get_or_error().await.map_err(|e| {
            tracing::error!("Error getting audience: {e}");

            GatewayError::Unauthorized("JWT audience not found".to_string())
        })?;

    tracing::trace!("Expected audience: {:?}", expected_audience);

    //
    // Extract token audience
    //
    let token_audience = token_data
        .claims
        .fields
        .get("aud")
        .and_then(|v| v.as_str())
        .ok_or(GatewayError::Unauthorized(format!(
            "Missing aud in token: {token}"
        )))?;

    tracing::trace!("Token audience: {:?}", token_audience);

    //
    // Validate audience
    //
    if token_audience != expected_audience {
        return Err(GatewayError::Unauthorized(format!(
            "Invalid audience: {expected_audience}"
        )));
    }

    //
    // Extract email from token
    //
    let email = Email::from_string({
        if let Some(upn) = token_data.claims.upn {
            upn
        } else if let Some(email) = token_data.claims.email {
            email
        } else {
            return Err(GatewayError::Unauthorized(
                "Email not found in token".to_string(),
            ));
        }
    })
    .map_err(|err| {
        tracing::error!("Error on extract email from token: {err}");

        GatewayError::Unauthorized(
            "Error on extract email from token".to_string(),
        )
    })?;

    tracing::trace!("Email: {:?}", email);

    Ok(Some(email))

    //
    // IMPORTANT:
    //
    // Remove this section after implement the new issuer parser
    //
    //let issuer = parse_issuer_from_request(req.clone()).await?;
    //tracing::trace!("Issuer: {:?}", issuer);
    //discover_provider(issuer.to_owned().to_lowercase(), req).await
}

/// Attempt to extract the `kid`-claim o
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

/// Discover identity provider
///
/// This function is used to discover identity provider and check credentials.
#[tracing::instrument(name = "discover_provider", skip_all)]
async fn discover_provider(
    issuer: String,
    req: HttpRequest,
) -> Result<Option<Email>, GatewayError> {
    tracing::trace!("Issuer: {:?}", issuer);

    let provider = if issuer.contains("sts.windows.net")
        || issuer.contains("azure-ad")
    {
        tracing::trace!("Checking credentials with Azure AD");
        az_check_credentials(req).await
    } else if issuer.contains("google") {
        tracing::trace!("Checking credentials with Google OAuth2");
        //
        // Try to extract authentication configurations from HTTP request.
        //
        let req_auth_config = req.app_data::<web::Data<AuthConfig>>();
        //
        // If Google OAuth2 config if not available the returns a Unauthorized.
        //
        if let None = req_auth_config {
            return Err(GatewayError::Unauthorized(format!(
                "Unable to extract Google auth config from request."
            )));
        }
        //
        // If Google OAuth2 config if not available the returns a Unauthorized
        // response.
        //
        let config = match req_auth_config.unwrap().google.clone() {
            OptionalConfig::Disabled => {
                tracing::warn!(
                    "Users trying to request and the Google OAuth2 is disabled."
                );

                return Err(GatewayError::Unauthorized(format!(
                    "Unable to extract auth config from request."
                )));
            }
            OptionalConfig::Enabled(config) => config,
        };
        //
        // Check if credentials are valid.
        //
        gc_check_credentials(req, config).await
    } else if issuer.contains("mycelium") {
        tracing::trace!("Checking credentials with Mycelium Auth");
        //
        // Extract the internal OAuth2 configuration from the HTTP request. If
        // the configuration is not available returns a InternalServerError
        // response.
        //
        let req_auth_config = match req
            .app_data::<web::Data<InternalOauthConfig>>()
        {
            Some(config) => config.jwt_secret.to_owned(),
            None => {
                return Err(GatewayError::InternalServerError(format!(
                        "Unexpected error on validate internal auth config. Please contact the system administrator."
                    )));
            }
        };
        //
        // Extract the token from the request. If the token is not available
        // returns a InternalServerError response.
        //
        let jwt_token = match req_auth_config.async_get_or_error().await {
            Ok(token) => token,
            Err(err) => {
                return Err(GatewayError::InternalServerError(format!(
                    "Unexpected error on get jwt token: {err}"
                )));
            }
        };
        //
        // Extract the bearer from the request. If the bearer is not available
        // returns a Unauthorized response.
        //
        let auth = match Authorization::<Bearer>::parse(&req) {
            Err(err) => match err {
                ParseError::Header => {
                    return Err(GatewayError::Unauthorized(format!(
                        "Bearer token not found or invalid in request: {err}"
                    )));
                }
                _ => {
                    return Err(GatewayError::Unauthorized(format!(
                        "Invalid Bearer token: {err}"
                    )));
                }
            },
            Ok(res) => res,
        };
        //
        // Decode the JWT token. If the token is not valid returns a
        // Unauthorized response.
        //
        match decode_jwt_hs512(auth, jwt_token) {
            Err(err) => match err.kind() {
                ErrorKind::ExpiredSignature => {
                    return Err(GatewayError::Unauthorized(format!(
                        "Expired token: {err}"
                    )));
                }
                _ => {
                    return Err(GatewayError::Unauthorized(format!(
                        "Unexpected error on decode jwt token: {err}"
                    )))
                }
            },
            Ok(res) => {
                let claims = res.claims;
                let email = claims.email;

                match Email::from_string(email) {
                    Err(err) => {
                        return Err(GatewayError::Unauthorized(format!(
                            "Invalid email: {err}"
                        )));
                    }
                    Ok(res) => return Ok(Some(res)),
                }
            }
        }
    } else {
        return Err(GatewayError::Unauthorized(format!(
            "Unknown identity provider: {issuer}",
        )));
    };

    match provider {
        Err(err) => {
            let msg =
                format!("Unexpected error on match Oauth2 provider: {err}");

            tracing::warn!("Unexpected error on discovery provider: {msg}");
            Err(GatewayError::Forbidden(msg))
        }
        Ok(res) => {
            tracing::trace!("Requesting Email: {:?}", res);
            Ok(Some(res))
        }
    }
}
