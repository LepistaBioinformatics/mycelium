use crate::dtos::GenericAccessTokenClaims;

use actix_web::HttpRequest;
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use awc::http::header::Header;
use jwt::{Header as JwtHeader, Token};
use myc_http_tools::responses::GatewayError;

/// Parse issuer from request
///
/// This function is used to parse issuer from request.
#[tracing::instrument(name = "parse_issuer_from_request", skip_all)]
pub(crate) async fn parse_issuer_from_request(
    req: HttpRequest,
) -> Result<(String, String), GatewayError> {
    let token = Authorization::<Bearer>::parse(&req)
        .map_err(|err| {
            tracing::error!("Unable to parse Authorization header: {err}");

            GatewayError::Unauthorized(
                "Unable to parse Authorization header".to_string(),
            )
        })?
        .to_string()
        .replace("Bearer ", "")
        .replace("bearer ", "");

    let unverified: Token<JwtHeader, GenericAccessTokenClaims, _> =
        Token::parse_unverified(&token).map_err(|err| {
            tracing::error!("Unable to parse unverified token: {err}");

            GatewayError::Unauthorized(
                "Unable to parse unverified token".to_string(),
            )
        })?;

    let issuer = unverified.claims().issuer.as_ref().ok_or(
        GatewayError::Unauthorized("Could not check issuer.".to_string()),
    )?;

    Ok((issuer.to_owned().to_lowercase(), token))
}
