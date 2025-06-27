use actix_web::HttpRequest;
use myc_core::domain::dtos::service::Service;
use myc_http_tools::responses::GatewayError;
use mycelium_base::dtos::Parent;
use wildmatch::WildMatch;

/// Check the source reliability
///
/// This method implements the zero-trust security policy of the api gateway. It
/// checks if the source of the request is allowed to access the service. If the
/// allowed-sources is `None` AND this method was not able to check the source
/// reliability, the request will be rejected with a 401 unauthorized error.
///
#[tracing::instrument(name = "check_the_source_reliability", skip_all)]
pub(super) async fn check_source_reliability<T>(
    req: HttpRequest,
    parent_service: &Parent<Service, T>,
) -> Result<(), GatewayError> {
    let service = if let Parent::Record(ref service) = parent_service {
        service
    } else {
        tracing::error!("Service not found");

        return Err(GatewayError::InternalServerError(String::from(
            "Service not found",
        )));
    };

    //
    // When the allowed-sources is present, try to check if the source is
    // allowed to access the service.
    //
    if let Some(allowed_sources) = service.allowed_sources.clone() {
        let host = req.headers().get("Host");

        if host.is_none() {
            tracing::warn!(
                "The host is not present in the request when it is required to check the source reliability"
            );

            return Err(GatewayError::Unauthorized(String::from(
                "The source is not allowed to access this service",
            )));
        }

        let host = host.unwrap().to_str().map_err(|_| {
            tracing::error!(
                "The host is present in the request but it is not a string when it is required to check the source reliability"
            );

            GatewayError::Unauthorized(String::from(
                "The source is not allowed to access this service",
            ))
        })?;

        //
        // Here we try to match the host with the allowed sources. In any other
        // case this method will return an gateway error.
        //
        for allowed_source in allowed_sources {
            if WildMatch::new(allowed_source.as_str()).matches(host) {
                return Ok(());
            }
        }

        tracing::warn!(
            "The host is not allowed to access the service {}",
            service.name
        );

        return Err(GatewayError::Unauthorized(String::from(
            "The source is not allowed to access this service",
        )));
    }

    //
    // When the allowed-sources is not present, the source is allowed to access
    // any service.
    //
    return Ok(());
}
