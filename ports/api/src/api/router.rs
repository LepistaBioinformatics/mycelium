use crate::modules::RoutesFetchingModule;

use actix_web::{
    error,
    http::{
        header::{ContentType, HeaderName},
        uri::PathAndQuery,
        StatusCode,
    },
    web, HttpRequest, HttpResponse,
};
use awc::{error::HeaderValue, Client};
use derive_more::Display;
use log::{debug, warn};
use myc_core::{
    domain::{dtos::http::RouteType, entities::RoutesFetching},
    settings::{FORWARDING_KEYS, FORWARD_FOR_KEY},
    use_cases::{
        gateway::{
            profile::check_credentials,
            routes::{match_forward_address, RoutesMatchResponseEnum},
        },
        roles::service::profile::{fetch_profile_from_email, ProfileResponse},
    },
};
use myc_http_tools::DEFAULT_PROFILE_KEY;
use myc_prisma::repositories::{
    LicensedResourcesFetchingSqlDbRepository, ProfileFetchingSqlDbRepository,
};
use serde::Serialize;
use shaku_actix::Inject;
use std::{fmt::Debug, str::FromStr, time::Duration};
use url::Url;

#[derive(Debug, Serialize)]
struct JsonError {
    msg: String,
    status: u16,
    message: String,
}

/// Internal errors as HTTP responses
///
/// Forwarding errors are fired only by Stomata errors.
#[derive(Debug, Display)]
pub enum ForwardingError {
    // ? -----------------------------------------------------------------------
    // ? Client errors (4xx)
    // ? -----------------------------------------------------------------------
    #[display(fmt = "BadRequest")]
    BadRequest(String),

    #[display(fmt = "Forbidden")]
    Forbidden(String),

    // ? -----------------------------------------------------------------------
    // ? Server errors (5xx)
    // ? -----------------------------------------------------------------------
    #[display(fmt = "InternalServerError")]
    InternalServerError(String),
}

impl error::ResponseError for ForwardingError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(JsonError {
                msg: self.to_string(),
                status: self.status_code().as_u16(),
                message: match self {
                    ForwardingError::BadRequest(msg) => msg.to_owned(),
                    ForwardingError::Forbidden(msg) => msg.to_owned(),
                    ForwardingError::InternalServerError(msg) => msg.to_owned(),
                },
            })
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            ForwardingError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ForwardingError::Forbidden { .. } => StatusCode::FORBIDDEN,
            ForwardingError::InternalServerError { .. } => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

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
pub async fn route_request(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    timeout: web::Data<u64>,
    routing_fetching_repo: Inject<RoutesFetchingModule, dyn RoutesFetching>,
) -> Result<HttpResponse, ForwardingError> {
    // ? -----------------------------------------------------------------------
    // ? Try to match the forward address
    //
    // Check if the specified client already exists. Case not, returns a
    // BadClient error. Otherwise proceed the pipeline.
    //
    // ? -----------------------------------------------------------------------

    let request_path =
        match PathAndQuery::from_str(req.uri().path().to_string().as_str()) {
            Err(err) => {
                warn!("{:?}", err);
                return Err(ForwardingError::BadRequest(String::from(
                    "Invalid request path",
                )));
            }
            Ok(res) => res,
        };

    debug!("Request Path: {:?}", request_path);

    let route = match match_forward_address(
        request_path,
        Box::new(&*routing_fetching_repo),
    )
    .await
    {
        Err(err) => {
            warn!("{:?}", err);
            return Err(ForwardingError::InternalServerError(String::from(
                "Invalid client service",
            )));
        }
        Ok(res) => match res {
            RoutesMatchResponseEnum::Found(route) => route,
            _ => {
                return Err(ForwardingError::BadRequest(String::from(
                    "Invalid request path",
                )))
            }
        },
    };

    debug!("Match Route: {:?}", route);

    // ? -----------------------------------------------------------------------
    // ? Build the downstream URL address
    //
    // With the service collected, try to build the downstream URL.
    //
    // ? -----------------------------------------------------------------------

    let registered_uri = match route.build_uri().await {
        Err(err) => {
            warn!("{:?}", err);
            return Err(ForwardingError::InternalServerError(format!("{err}")));
        }
        Ok(res) => match Url::parse(res.to_string().as_str()) {
            Err(err) => {
                warn!("{:?}", err);
                return Err(ForwardingError::InternalServerError(format!(
                    "{err}"
                )));
            }
            Ok(mut url) => {
                let name = route.service.name.to_owned();

                url.set_path(
                    req.uri()
                        .path()
                        .replace(format!("/{name}").as_str(), "")
                        .as_str(),
                );

                url.set_query(req.uri().query());
                url
            }
        },
    };

    debug!("Client URI: {:?}", registered_uri);

    let forwarded_req = client
        .request_from(registered_uri.as_str(), req.head())
        .no_decompress()
        .timeout(Duration::from_secs(*timeout.into_inner()));

    debug!("Forward Request (1): {:?}", forwarded_req);

    let mut forwarded_req = match req.head().peer_addr {
        Some(addr) => forwarded_req
            .insert_header((FORWARD_FOR_KEY, format!("{}", addr.ip()))),
        None => forwarded_req,
    };

    debug!("Forward Request (2): {:?}", forwarded_req);

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
        // Get email from response
        //
        let email = match check_credentials(req.to_owned()).await {
            Err(err) => {
                warn!("{:?}", err);
                return Err(ForwardingError::Forbidden(format!("{err}")));
            }
            Ok(res) => {
                debug!("Requesting Email: {:?}", res);

                Some(res)
            }
        };
        //
        // Get the profile response
        //
        let profile = match fetch_profile_from_email(
            email.to_owned().unwrap(),
            Box::new(&ProfileFetchingSqlDbRepository {}),
            Box::new(&LicensedResourcesFetchingSqlDbRepository {}),
        )
        .await
        {
            Err(err) => {
                warn!("{:?}", err);
                return Err(ForwardingError::InternalServerError(format!(
                    "{err}"
                )));
            }
            Ok(res) => {
                debug!("Requesting Profile: {:?}", res);

                match res {
                    ProfileResponse::UnregisteredUser(email) => {
                        return Err(ForwardingError::Forbidden(format!(
                            "Unauthorized access: {:?}",
                            email,
                        )))
                    }
                    ProfileResponse::RegisteredUser(res) => res,
                }
            }
        };
        //
        // Insert profile in header
        //
        debug!("Inserting Profile in Requesting Header");

        forwarded_req.headers_mut().insert(
            HeaderName::from_str(DEFAULT_PROFILE_KEY).unwrap(),
            match HeaderValue::from_str(
                &serde_json::to_string(&profile).unwrap(),
            ) {
                Err(err) => {
                    warn!("err: {:?}", err.to_string());
                    return Err(ForwardingError::InternalServerError(format!(
                        "{err}"
                    )));
                }
                Ok(res) => res,
            },
        );
    }

    debug!("Forward Request (3): {:?}", forwarded_req);

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
            warn!("{:?}", err);
            return Err(ForwardingError::InternalServerError(String::from(
                format!("{err}"),
            )));
        }
        Ok(res) => res,
    };

    debug!("Binding Response (1): {:?}", binding_response);

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

    debug!("Binding Response (2): {:?}", binding_response);

    Ok(client_response.streaming(binding_response))
}
