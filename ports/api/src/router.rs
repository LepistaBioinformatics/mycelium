use super::middleware::fetch_and_inject_profile_to_forward;
use crate::{
    models::api_config::ApiConfig, modules::RoutesFetchingModule,
    settings::GATEWAY_API_SCOPE,
};

use actix_web::{http::uri::PathAndQuery, web, HttpRequest, HttpResponse};
use awc::{
    error::{ConnectError, SendRequestError},
    Client,
};
use myc_config::optional_config::OptionalConfig;
use myc_core::{
    domain::{
        dtos::{
            http::{HttpMethod, Protocol},
            http_secret::HttpSecret,
            route_type::RouteType,
        },
        entities::RoutesFetching,
    },
    use_cases::gateway::routes::match_forward_address,
};
use myc_http_tools::{
    responses::GatewayError,
    settings::{
        DEFAULT_PROFILE_KEY, DEFAULT_REQUEST_ID_KEY, FORWARDING_KEYS,
        FORWARD_FOR_KEY,
    },
};
use mycelium_base::{dtos::Parent, entities::FetchResponseKind};
use shaku_actix::Inject;
use std::{str::FromStr, time::Duration};
use tracing::{error, trace, warn};
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
    fields(myc.requestId = tracing::field::Empty)
)]
pub(crate) async fn route_request(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    api_config: web::Data<ApiConfig>,
    timeout: web::Data<u64>,
    routing_fetching_repo: Inject<RoutesFetchingModule, dyn RoutesFetching>,
) -> Result<HttpResponse, GatewayError> {
    let replace_path = &format!("/{}", GATEWAY_API_SCOPE);

    // ? -----------------------------------------------------------------------
    // ? Set the request id to the current span
    // ? -----------------------------------------------------------------------

    if let Some(request_id) = req.headers().get(DEFAULT_REQUEST_ID_KEY) {
        tracing::Span::current()
            .record("myc.requestId", &Some(request_id.to_str().unwrap()));
    }

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
        request_path.to_owned(),
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
        Ok(res) => match res {
            FetchResponseKind::Found(route) => {
                trace!(
                    "[ {request_path} ]: {service} -> {path}",
                    request_path = request_path.path(),
                    service = match route.service {
                        Parent::Record(ref service) => service.name.to_owned(),
                        Parent::Id(id) => id.to_string(),
                    },
                    path = route.path.to_owned()
                );

                route
            }
            _ => {
                return Err(GatewayError::BadRequest(String::from(
                    "Request path does not match any service",
                )))
            }
        },
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
                let service = match route.service {
                    Parent::Record(ref service) => service,
                    Parent::Id(_) => {
                        error!("Service not found");

                        return Err(GatewayError::InternalServerError(
                            String::from("Service not found"),
                        ));
                    }
                };

                let name = service.name.to_owned();

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

    trace!("Injecting downstream secret into request");

    let route_secret = match route.solve_secret() {
        Err(err) => {
            warn!("{:?}", err);
            return Err(GatewayError::InternalServerError(format!("{err}")));
        }
        Ok(res) => res,
    };

    let mut route_key = None;

    if let Some(secret) = route_secret {
        let accept_insecure_routing =
            route.accept_insecure_routing.unwrap_or(false);

        //
        // Check if the service supports TLS
        //
        if let OptionalConfig::Disabled = api_config.tls {
            if !accept_insecure_routing {
                error!("Secrets are only allowed for HTTPS routes");

                return Err(GatewayError::InternalServerError(
                    "Unexpected error on route request".to_string(),
                ));
            }
        }

        //
        // Check if the route supports HTTPS
        //
        if ![Protocol::Https].contains(&route.protocol) {
            if !accept_insecure_routing {
                error!(
                    "Secrets are only allowed for HTTPS routes: {path}",
                    path = route.path
                );

                return Err(GatewayError::InternalServerError(
                    "Unexpected error on route request".to_string(),
                ));
            }
        }

        match secret {
            //
            // Insert the authorization key into the header
            //
            HttpSecret::AuthorizationHeader {
                name,
                prefix,
                token,
            } => {
                //
                // Build the bearer token
                //
                let mut bearer_token = prefix.unwrap_or("Bearer".to_string());
                bearer_token.push_str(format!(" {}", token).as_str());
                let bearer_name = name.unwrap_or("Authorization".to_string());
                route_key = Some(bearer_name.to_owned());

                //
                // Remove any previous Authorization header that may exist
                //
                forwarded_req.headers_mut().remove(bearer_name.to_owned());
                forwarded_req
                    .headers_mut()
                    .remove(bearer_name.to_lowercase().to_owned());
                forwarded_req
                    .headers_mut()
                    .remove(bearer_name.to_uppercase().to_owned());

                //
                // Insert the new Authorization header
                //
                forwarded_req =
                    forwarded_req.insert_header((bearer_name, bearer_token));
            }
            //
            // Insert the query parameter into the header
            //
            HttpSecret::QueryParameter { name, token } => {
                forwarded_req =
                    match forwarded_req.query(&[(name, token.to_owned())]) {
                        Err(err) => {
                            warn!("{:?}", err);
                            return Err(GatewayError::InternalServerError(
                                format!("{err}"),
                            ));
                        }
                        Ok(res) => res,
                    };
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Build the downstream url if the address has match.
    //
    // Submit the request and stream the response to the requester.
    // ? -----------------------------------------------------------------------

    trace!("Forwarding request to service");

    let binding_response = match forwarded_req.send_stream(payload).await {
        Err(err) => match err {
            SendRequestError::Connect(e) => {
                match e {
                    ConnectError::SslIsNotSupported => {
                        warn!("SSL is not supported");

                        return Err(GatewayError::InternalServerError(
                            "SSL is not supported".to_string(),
                        ));
                    }
                    ConnectError::SslError(e) => {
                        warn!("SSL error: {e}");

                        return Err(GatewayError::InternalServerError(
                            "SSL error".to_string(),
                        ));
                    }
                    _ => (),
                }

                warn!("Error on route/connect to service: {e}");

                return Err(GatewayError::InternalServerError(String::from(
                    "Unexpected error on route request",
                )));
            }
            SendRequestError::Url(e) => {
                warn!("Error on route/url to service: {e}");

                return Err(GatewayError::InternalServerError(String::from(
                    format!("{e}"),
                )));
            }
            err => {
                warn!("Error on route/stream to service: {err}");

                return Err(GatewayError::InternalServerError(String::from(
                    format!("{err}"),
                )));
            }
        },
        Ok(res) => res,
    };

    let mut client_response = HttpResponse::build(binding_response.status());

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

    trace!("Route request completed");

    Ok(client_response.streaming(binding_response))
}
