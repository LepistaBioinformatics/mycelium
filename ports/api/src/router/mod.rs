/// Gateway router
///
/// This module is responsible for routing the request to the appropriate
/// service.
///
/// Steps executed here includes:
///
/// - Check if the source of the request is allowed to access the service.
/// - Check if the method of the request is allowed to access the service.
/// - Build the downstream URL address.
/// - Check security group and inject the email, profile, and role scoped
///   connection string into the request.
/// - Inject the secret into the request if needed.
/// - Build the downstream url if the address has match.
/// - Cleanup the headers of the response before send it to the client.
/// - Stream the response to the requester.
/// - Inject spans to the request to be used by the tracing system.
///
mod build_the_gateway_response;
mod check_protocol_permission;
mod check_security_group;
mod check_source_reliability;
mod initialize_downstream_request;
mod inject_downstream_secret;
mod match_downstream_route_from_request;
mod stream_request_to_downstream;

use build_the_gateway_response::*;
use check_protocol_permission::*;
use check_security_group::*;
use check_source_reliability::*;
use initialize_downstream_request::*;
use inject_downstream_secret::*;
use match_downstream_route_from_request::*;
use stream_request_to_downstream::*;

use crate::{models::api_config::ApiConfig, settings::GATEWAY_API_SCOPE};

use actix_web::{web, HttpRequest, HttpResponse};
use awc::Client;
use myc_http_tools::{
    responses::GatewayError, settings::DEFAULT_REQUEST_ID_KEY,
};
use myc_mem_db::repositories::MemDbAppModule;
use tracing::Instrument;

/// Forward request to the client service.
///
/// The client request should be redirected to the client services if the
/// service name exists and the current user has enough permissions to perform
/// the desired action.
///
/// TODO: This forwarded implementation is incomplete as it only handles the
/// TODO: unofficial X-Forwarded-For header but not the official Forwarded
/// TODO: one.
///
#[tracing::instrument(
    name = "route_request", 
    skip_all,
    fields(
        //
        // Request information
        //
        myc.router.req_id = tracing::field::Empty,
        myc.router.req_method = tracing::field::Empty,
        myc.router.req_protocol = tracing::field::Empty,
        //
        // Response information
        //
        myc.router.res_status = tracing::field::Empty,
        myc.router.res_duration = tracing::field::Empty,
    )
)]
pub(crate) async fn route_request(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    api_config: web::Data<ApiConfig>,
    app_module: web::Data<MemDbAppModule>,
) -> Result<HttpResponse, GatewayError> {
    let gateway_base_path = &format!("/{}", GATEWAY_API_SCOPE);

    // ? -----------------------------------------------------------------------
    // ? Initialize route span
    // ? -----------------------------------------------------------------------

    let span = tracing::Span::current();

    span.record("myc.router.req_method", &Some(req.method().to_string()))
        .record("myc.router.req_protocol", &Some(req.full_url().scheme()));

    // ? -----------------------------------------------------------------------
    // ? Populate request id if exists
    // ? -----------------------------------------------------------------------

    let request_id = if let Some(request_id) =
        req.headers().get(DEFAULT_REQUEST_ID_KEY)
    {
        span.record("myc.router.req_id", &Some(request_id.to_str().unwrap()));

        Some(request_id.to_owned())
    } else {
        None
    };

    // ? -----------------------------------------------------------------------
    // ? Try to match the downstream route
    //
    // Try to match the downstream route from the request.
    //
    // ? -----------------------------------------------------------------------

    let route = match_downstream_route_from_request(
        req.clone(),
        gateway_base_path,
        app_module.clone(),
    )
    .instrument(span.to_owned())
    .await?;

    // ? -----------------------------------------------------------------------
    // ? Check if the source is allowed
    //
    // Check if the source of the request is allowed to access the service.
    // Sources are defined in the service configuration and should be defined as
    // a list of strings with or without wildcards.
    //
    // ? -----------------------------------------------------------------------

    check_source_reliability(req.clone(), &route.service)
        .instrument(span.to_owned())
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Check if the method is allowed
    //
    // Check if the method of the request is allowed to access the service.
    // Methods are defined in the service configuration.
    //
    // ? -----------------------------------------------------------------------

    check_protocol_permission(req.clone(), &route)
        .instrument(span.to_owned())
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Build the downstream URL address
    //
    // With the service collected, try to build the downstream URL.
    //
    // ? -----------------------------------------------------------------------

    let mut downstream_request = initialize_downstream_request(
        req.clone(),
        &route,
        client.clone(),
        api_config.clone(),
        gateway_base_path,
    )
    .instrument(span.to_owned())
    .await?;

    // ? -----------------------------------------------------------------------
    // ? Check authentication and get permissions
    //
    // Inject the email, profile, and role scoped connection string into the
    // request if the route has a security group.
    //
    // ? -----------------------------------------------------------------------

    downstream_request =
        check_security_group(req.clone(), downstream_request, route.clone())
            .instrument(span.to_owned())
            .await?;

    // ? -----------------------------------------------------------------------
    // ? Inject the downstream secret into the request
    //
    // Inject the downstream secret into the request if the service requested
    // by the route has a secret.
    //
    // ? -----------------------------------------------------------------------

    let (downstream_request, route_key) = inject_downstream_secret(
        downstream_request,
        route.clone(),
        None,
        api_config.clone(),
    )
    .instrument(span.to_owned())
    .await?;

    // ? -----------------------------------------------------------------------
    // ? Submit downstream request
    //
    // Submit the request and stream the response to the downstream service.
    //
    // ? -----------------------------------------------------------------------

    let downstream_response =
        stream_request_to_downstream(downstream_request, payload)
            .instrument(span.to_owned())
            .await?;

    // ? -----------------------------------------------------------------------
    // ? Build the gateway response
    //
    // Build the gateway response with the downstream response.
    //
    // ? -----------------------------------------------------------------------

    let mut gateway_response =
        build_the_gateway_response(request_id, route_key, &downstream_response)
            .instrument(span.to_owned())
            .await?;

    // ? -----------------------------------------------------------------------
    // ? Stream the response to the client
    //
    // Final response should be streamed to the client to avoid memory
    // exhaustion.
    //
    // ? -----------------------------------------------------------------------

    Ok(gateway_response.streaming(downstream_response))
}
