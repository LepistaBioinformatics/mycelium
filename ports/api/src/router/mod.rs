mod match_forward_address;

use super::middleware::fetch_and_inject_profile_to_forward;
use crate::{modules::RoutesFetchingModule, settings::GATEWAY_API_SCOPE};

use actix_web::{
    error, http::uri::PathAndQuery, web, HttpRequest, HttpResponse,
};
use awc::Client;
use match_forward_address::{match_forward_address, RoutesMatchResponseEnum};
use myc_core::domain::{
    dtos::{http::HttpMethod, route_type::RouteType},
    entities::RoutesFetching,
};
use myc_http_tools::{
    responses::GatewayError,
    settings::{DEFAULT_PROFILE_KEY, FORWARDING_KEYS, FORWARD_FOR_KEY},
};
use shaku_actix::Inject;
use std::{str::FromStr, time::Duration};
use tracing::{debug, trace, warn};
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
#[tracing::instrument(
    name = "route_request", 
    skip_all,
    fields(
        routing_time = tracing::field::Empty,
    )
)]
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

    trace!("Discovering route for request");

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

    trace!("Checking if method is allowed");

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

    trace!("Building downstream URL");

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

    trace!("Checking authentication and permissions");

    match route.group.to_owned() {
        //
        // Public routes do not need any authentication or profile injection.
        //
        RouteType::Public => (),
        //
        // Protected routes should include the full qualified user profile into
        // the header
        //
        RouteType::Protected => {
            //
            // Try to populate profile from the request
            //
            forwarded_req = fetch_and_inject_profile_to_forward(
                req,
                forwarded_req,
                None,
                None,
            )
            .await?;
        }
        //
        // Protected routes should include the user profile filtered by roles
        // into the header
        //
        RouteType::ProtectedByRoles { roles } => {
            //
            // Try to populate profile from the request filtering licensed
            // resources by roles
            //
            forwarded_req = fetch_and_inject_profile_to_forward(
                req,
                forwarded_req,
                Some(roles),
                None,
            )
            .await?;
        }
        //
        // Protected routes should include the user profile filtered by roles
        // and permissions into the header
        //
        RouteType::ProtectedByPermissionedRoles { permissioned_roles } => {
            //
            // Try to populate profile from the request filtering licensed
            // resources by roles and permissions
            //
            forwarded_req = fetch_and_inject_profile_to_forward(
                req,
                forwarded_req,
                None,
                Some(permissioned_roles),
            )
            .await?;
        }
        //
        // Protected routes by service token should include the users role which
        // the service token is associated
        //
        RouteType::ProtectedByServiceTokenWithRole { roles } => {
            //
            // Try to populate profile from the request filtering licensed
            // resources by roles and permissions
            //
            println!("roles: {:?}", roles);

            unimplemented!("ProtectedByServiceToken not implemented yet");
        }
        //
        // Protected routes by service token should include the users role which
        // the service token is associated
        //
        RouteType::ProtectedByServiceTokenWithPermissionedRoles {
            permissioned_roles,
        } => {
            //
            // Try to populate profile from the request filtering licensed
            // resources by roles and permissions
            //
            println!("permissioned_roles: {:?}", permissioned_roles);

            unimplemented!("ProtectedByServiceToken not implemented yet");
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Build the downstream url if the address has match.
    //
    // Submit the request and stream the response to the requester.
    // ? -----------------------------------------------------------------------

    trace!("Forwarding request to service");

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

    trace!("Route request completed");

    Ok(client_response.streaming(binding_response))
}
