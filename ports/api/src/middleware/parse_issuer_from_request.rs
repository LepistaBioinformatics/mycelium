use actix_web::HttpRequest;
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use awc::http::header::Header;
use jwt::{Header as JwtHeader, RegisteredClaims, Token};
use myc_http_tools::responses::GatewayError;

/// Parse issuer from request
///
/// This function is used to parse issuer from request.
#[tracing::instrument(name = "parse_issuer_from_request", skip_all)]
pub(crate) async fn parse_issuer_from_request(
    req: HttpRequest,
) -> Result<(String, String), GatewayError> {
    let auth = match Authorization::<Bearer>::parse(&req) {
        Err(err) => {
            let msg =
                format!("Unexpected error on get bearer from request: {err}");

            tracing::error!("{msg}");

            return Err(GatewayError::Unauthorized(msg));
        }
        Ok(res) => res,
    }
    .to_string()
    .replace("Bearer ", "")
    .replace("bearer ", "");

    let unverified: Token<JwtHeader, RegisteredClaims, _> =
        match Token::parse_unverified(&auth) {
            Err(err) => {
                let msg = format!(
                    "Unexpected error on parse unverified token: {err}"
                );

                tracing::error!("{msg}");

                return Err(GatewayError::Unauthorized(msg));
            }
            Ok(res) => res,
        };

    let issuer = unverified.claims().issuer.as_ref().ok_or(
        GatewayError::Unauthorized("Could not check issuer.".to_string()),
    )?;

    Ok((issuer.to_owned().to_lowercase(), auth))
}
