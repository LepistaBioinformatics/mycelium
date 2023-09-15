use crate::{modules::RoutesFetchingModule, settings::GATEWAY_API_SCOPE};

use actix_web::{
    error, http::uri::PathAndQuery, web, HttpRequest, HttpResponse,
};
use awc::Client;
use log::{debug, warn};
use myc_core::{
    domain::{
        dtos::http::{HttpMethod, RouteType},
        entities::RoutesFetching,
    },
    settings::{FORWARDING_KEYS, FORWARD_FOR_KEY},
    use_cases::gateway::routes::{
        match_forward_address, RoutesMatchResponseEnum,
    },
};
use myc_http_tools::{
    middleware::fetch_and_inject_profile_to_forward, responses::GatewayError,
    DEFAULT_PROFILE_KEY,
};
use shaku_actix::Inject;
use std::{str::FromStr, time::Duration};
use url::Url;

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
pub(crate) async fn route_request(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    timeout: web::Data<u64>,
    routing_fetching_repo: Inject<RoutesFetchingModule, dyn RoutesFetching>,
) -> Result<HttpResponse, GatewayError> {
    let replace_path = &format!("/{}", GATEWAY_API_SCOPE);

    // ? -----------------------------------------------------------------------
    // ? Try to match the forward address
    //
    // Check if the specified client already exists. Case not, returns a
    // BadClient error. Otherwise proceed the pipeline.
    //
    // ? -----------------------------------------------------------------------

    let request_path = match PathAndQuery::from_str(
        &req.uri()
            .path()
            .to_string()
            .as_str()
            .replace(replace_path, ""),
    ) {
        Err(err) => {
            warn!("{:?}", err);
            return Err(GatewayError::BadRequest(String::from(
                "Invalid request path",
            )));
        }
        Ok(res) => res,
    };

    let route = match match_forward_address(
        request_path,
        Box::new(&*routing_fetching_repo),
    )
    .await
    {
        Err(err) => {
            warn!("{:?}", err);

            return Err(GatewayError::InternalServerError(String::from(
                "Invalid client service",
            )));
        }
        Ok(res) => {
            debug!("match routes res: {:?}", res);

            match res {
                RoutesMatchResponseEnum::Found(route) => route,
                _ => {
                    return Err(GatewayError::BadRequest(String::from(
                        "Request path does not match any service",
                    )))
                }
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Check if the method is allowed
    // ? -----------------------------------------------------------------------

    match route
        .allow_method(HttpMethod::from_reqwest_method(req.method().to_owned()))
        .await
    {
        None => {
            return Err(GatewayError::MethodNotAllowed(String::from(
                "Invalid HTTP method or not allowed for this route",
            )))
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

    // ? -----------------------------------------------------------------------
    // ? Build the downstream URL address
    //
    // With the service collected, try to build the downstream URL.
    //
    // ? -----------------------------------------------------------------------

    let registered_uri = match route.build_uri().await {
        Err(err) => {
            warn!("{:?}", err);
            return Err(GatewayError::InternalServerError(format!("{err}")));
        }
        Ok(res) => match Url::parse(res.to_string().as_str()) {
            Err(err) => {
                warn!("{:?}", err);
                return Err(GatewayError::InternalServerError(format!(
                    "{err}"
                )));
            }
            Ok(mut url) => {
                let name = route.service.name.to_owned();

                url.set_path(
                    req.uri()
                        .path()
                        .replace(replace_path, "")
                        .replace(format!("/{name}").as_str(), "")
                        .as_str(),
                );

                url.set_query(req.uri().query());
                url
            }
        },
    };

    let forwarded_req = client
        .request_from(registered_uri.as_str(), req.head())
        .no_decompress()
        .timeout(Duration::from_secs(*timeout.into_inner()));

    let mut forwarded_req = match req.head().peer_addr {
        Some(addr) => forwarded_req
            .insert_header((FORWARD_FOR_KEY, format!("{}", addr.ip()))),
        None => forwarded_req,
    };

    // ? -----------------------------------------------------------------------
    // ? Check authentication and get permissions
    //
    // Protected routes (RouteType::Protected) should include valid information
    // of the user email. This step try to collect this information and fetch
    // the user profile. Case email is valid but the user is not registered on
    // the system, it returns a Forbidden response. Case the user was previously
    // registered, then include the profile-pack into the header response to be
    // collected by client service.
    //
    // ? -----------------------------------------------------------------------

    if let RouteType::Protected = route.group.to_owned() {
        //
        // Try to populate profile from the request
        //
        forwarded_req =
            fetch_and_inject_profile_to_forward(req, forwarded_req).await?;
    }

    // ? -----------------------------------------------------------------------
    // ? Build the downstream url if the address has match.
    //
    // Submit the request and stream the response to the requester.
    // ? -----------------------------------------------------------------------

    let binding_response = match forwarded_req
        .send_stream(payload)
        .await
        .map_err(error::ErrorInternalServerError)
    {
        Err(err) => {
            warn!("Error on route/stream to service: {err}");

            return Err(GatewayError::InternalServerError(String::from(
                format!("{err}"),
            )));
        }
        Ok(res) => res,
    };

    let mut client_response = HttpResponse::build(binding_response.status());

    // ! Remove `Connection` as peer and forward service name
    //
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
    //
    // Both headers contain sensitive information about the system internals.
    // Thus, be careful on edit this section.
    for (header_name, header_value) in
        binding_response.headers().iter().filter(|(h, _)| {
            let mut headers = FORWARDING_KEYS.to_vec();
            headers.append(&mut vec![FORWARD_FOR_KEY, DEFAULT_PROFILE_KEY]);

            headers
                .into_iter()
                .map(|h| h.to_lowercase())
                .collect::<Vec<String>>()
                .contains(&h.to_owned().to_string().to_lowercase())
        })
    {
        client_response
            .insert_header((header_name.clone(), header_value.clone()));
    }

    Ok(client_response.streaming(binding_response))
}
