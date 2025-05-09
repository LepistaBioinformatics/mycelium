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
mod check_protocol_permission;
mod check_security_group;
mod check_source_reliability;
mod initialize_downstream_request;
mod inject_downstream_secret;

use check_protocol_permission::*;
use check_security_group::*;
use check_source_reliability::*;
use initialize_downstream_request::*;
use inject_downstream_secret::*;

use crate::{models::api_config::ApiConfig, settings::GATEWAY_API_SCOPE};

use actix_web::{http::uri::PathAndQuery, web, HttpRequest, HttpResponse};
use awc::{
    error::{ConnectError, SendRequestError},
    Client,
};
use myc_core::use_cases::gateway::routes::match_forward_address;
use myc_http_tools::{
    responses::GatewayError,
    settings::{
        DEFAULT_PROFILE_KEY, DEFAULT_REQUEST_ID_KEY, FORWARDING_KEYS,
        FORWARD_FOR_KEY,
    },
};
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::{dtos::Parent, entities::FetchResponseKind};
use shaku::HasComponent;
use std::str::FromStr;
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
        myc.router.req_path = tracing::field::Empty,
        myc.router.req_method = tracing::field::Empty,
        myc.router.req_protocol = tracing::field::Empty,
        //
        // Downstream information
        //
        myc.router.down_service_id = tracing::field::Empty,
        myc.router.down_service_name = tracing::field::Empty,
        myc.router.down_match_path = tracing::field::Empty,
        myc.router.down_path_type = tracing::field::Empty,
        myc.router.down_protocol = tracing::field::Empty,
        //
        // Response information
        //
        myc.router.res_status = tracing::field::Empty,
        myc.router.res_duration = tracing::field::Empty,
        myc.router.res_size = tracing::field::Empty,
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
    // ? Try to match the forward address
    //
    // Check if the specified client already exists. Case not, returns a
    // BadClient error. Otherwise proceed the pipeline.
    //
    // ? -----------------------------------------------------------------------

    tracing::trace!("Discovering route for request");

    let uri_str = &req
        .uri()
        .path()
        .to_string()
        .as_str()
        .replace(gateway_base_path, "");

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
    // ? Submit the request
    //
    // Submit the request and stream the response to the downstream service.
    //
    // ? -----------------------------------------------------------------------

    let binding_response = match downstream_request.send_stream(payload).await {
        Err(err) => match err {
            SendRequestError::Connect(e) => {
                match e {
                    ConnectError::SslIsNotSupported => {
                        tracing::warn!("SSL is not supported");

                        return Err(GatewayError::InternalServerError(
                            "SSL is not supported".to_string(),
                        ));
                    }
                    ConnectError::SslError(e) => {
                        tracing::warn!("SSL error: {e}");

                        return Err(GatewayError::InternalServerError(
                            "SSL error".to_string(),
                        ));
                    }
                    _ => (),
                }

                tracing::warn!("Error on route/connect to service: {e}");

                return Err(GatewayError::InternalServerError(String::from(
                    "Unexpected error on route request",
                )));
            }
            SendRequestError::Url(e) => {
                tracing::warn!("Error on route/url to service: {e}");

                return Err(GatewayError::InternalServerError(String::from(
                    format!("{e}"),
                )));
            }
            err => {
                tracing::warn!("Error on route/stream to service: {err}");

                return Err(GatewayError::InternalServerError(String::from(
                    format!("{err}"),
                )));
            }
        },
        Ok(res) => res,
    };

    let mut client_response = HttpResponse::build(binding_response.status());

    if let Some(request_id) = request_id {
        client_response
            .insert_header((DEFAULT_REQUEST_ID_KEY, request_id.to_owned()));
    }

    // ! Remove `Connection` as peer and forward service name
    //
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
    //
    // Both headers contain sensitive information about the system internals.
    // Thus, be careful on edit this section.

    //
    // Start the headers with the route key if exists
    //
    let mut headers = if let Some(key) = route_key {
        [key].to_vec().into_iter().collect::<Vec<String>>()
    } else {
        vec![]
    };

    //
    // Append the standard forwarding keys
    //
    headers.append(
        &mut FORWARDING_KEYS
            .to_vec()
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );

    //
    // Append the default profile and forward for keys
    //
    headers.append(&mut vec![
        FORWARD_FOR_KEY.to_string(),
        DEFAULT_PROFILE_KEY.to_string(),
    ]);

    //
    // Filter the headers of the response before send it to the client
    //
    for (header_name, header_value) in
        binding_response.headers().iter().filter(|(h, _)| {
            headers
                .to_owned()
                .into_iter()
                .map(|h| h.to_lowercase())
                .collect::<Vec<String>>()
                .contains(&h.to_owned().to_string().to_lowercase())
        })
    {
        client_response
            .insert_header((header_name.clone(), header_value.clone()));
    }

    if let Some(size) = binding_response
        .headers()
        .get("content-length")
        .map(|h| h.to_str().unwrap_or("0").parse::<u64>().unwrap_or(0))
    {
        span.record("myc.router.res_size", &Some(size));
    }

    tracing::trace!("Route request completed");

    Ok(client_response.streaming(binding_response))
}
