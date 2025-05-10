use actix_web::HttpRequest;
use myc_core::domain::dtos::{http::HttpMethod, route::Route};
use myc_http_tools::responses::GatewayError;

/// Check the protocol permission
///
/// This method checks if the protocol of the request is allowed to access the
/// service. If the protocol is not allowed, the request will be rejected with a
/// 401 unauthorized error.
///
#[tracing::instrument(name = "check_protocol_permission", skip_all)]
pub(super) async fn check_protocol_permission(
    req: HttpRequest,
    route: &Route,
) -> Result<(), GatewayError> {
    match route
        .allow_method(HttpMethod::from_reqwest_method(req.method().to_owned()))
        .await
    {
        None => {
            tracing::warn!("Method not allowed for this route");

            return Err(GatewayError::MethodNotAllowed(String::from(
                "Invalid HTTP method or not allowed for this route",
            )));
        }
        Some(method) => match method {
            HttpMethod::None => {
                return Err(GatewayError::MethodNotAllowed(String::from(
                    "HTTP method not allowed for this route",
                )))
            }
            _ => (),
        },
    }

    Ok(())
}
