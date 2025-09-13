use actix_web::{web, HttpRequest};
use http::uri::PathAndQuery;
use myc_core::{
    domain::dtos::route::Route,
    use_cases::gateway::routes::match_forward_address,
};
use myc_http_tools::responses::GatewayError;
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::{dtos::Parent, entities::FetchResponseKind};
use shaku::HasComponent;
use std::str::FromStr;
use tracing::Instrument;

/// Match the downstream route from the request
///
/// This function matches the downstream route from the request.
///
#[tracing::instrument(name 
    = "match_downstream_route_from_request", 
    skip_all,
    fields(
        //
        // Request information
        //
        myc.router.req_path = tracing::field::Empty,
        //
        // Downstream information
        //
        myc.router.down_service_id = tracing::field::Empty,
        myc.router.down_service_name = tracing::field::Empty,
        myc.router.down_match_path = tracing::field::Empty,
        myc.router.down_path_type = tracing::field::Empty,
        myc.router.down_protocol = tracing::field::Empty,
    )
)]
pub(super) async fn match_downstream_route_from_request(
    req: HttpRequest,
    app_module: web::Data<MemDbAppModule>,
) -> Result<Route, GatewayError> {
    let span = tracing::Span::current();

    let uri_str = &req
        .uri()
        .path();

    let request_path = PathAndQuery::from_str(uri_str).map_err(|err| {
        tracing::warn!("{:?}", err);
        GatewayError::BadRequest(String::from("Invalid request path"))
    })?;

    span.record("myc.router.req_path", &Some(request_path.path()));

    let route = match match_forward_address(
        request_path.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .instrument(span.to_owned())
    .await
    .map_err(|err| {
        tracing::warn!("{:?}", err);

        GatewayError::InternalServerError(String::from(
            "Invalid client service",
        ))
    })? {
        FetchResponseKind::Found(route) => route,
        _ => {
            return Err(GatewayError::BadRequest(String::from(
                "Request path does not match any service",
            )))
        }
    };

    span.record(
        "myc.router.down_service_id",
        &Some(route.get_service_id().to_string()),
    )
    .record("myc.router.down_match_path", &Some(route.path.clone()))
    .record(
        "myc.router.down_path_type",
        &Some(route.security_group.to_string()),
    );

    if let Parent::Record(ref service) = route.service {
        span.record(
            "myc.router.down_protocol",
            &Some(service.protocol.to_string()),
        )
        .record(
            "myc.router.down_service_name",
            &Some(service.name.to_string()),
        );
    }

    Ok(route)
}
