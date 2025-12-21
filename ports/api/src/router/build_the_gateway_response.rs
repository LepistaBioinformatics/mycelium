use super::DownstreamResponse;

use actix_web::{HttpResponse, HttpResponseBuilder};
use awc::error::HeaderValue;
use myc_http_tools::{
    responses::GatewayError,
    settings::{
        DEFAULT_PROFILE_KEY, DEFAULT_REQUEST_ID_KEY, FORWARDING_KEYS,
        FORWARD_FOR_KEY, MYCELIUM_SERVICE_NAME,
    },
};

/// Build the gateway response
///
/// This function builds the gateway response with the downstream response.
///
#[tracing::instrument(
    name = "build_the_gateway_response", 
    skip_all, 
    fields(
        myc.router.res_size = tracing::field::Empty,
    )
)]
pub(super) async fn build_the_gateway_response(
    request_id: Option<HeaderValue>,
    route_key: Option<String>,
    downstream_response: &DownstreamResponse,
) -> Result<HttpResponseBuilder, GatewayError> {
    let span = tracing::Span::current();

    let mut gateway_response =
        HttpResponse::build(downstream_response.status());

    if let Some(request_id) = request_id {
        gateway_response
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
        MYCELIUM_SERVICE_NAME.to_string(),
    ]);

    //
    // Filter the headers of the response before send it to the client
    //
    for (header_name, header_value) in
        downstream_response.headers().iter().filter(|(h, _)| {
            headers
                .to_owned()
                .into_iter()
                .map(|h| h.to_lowercase())
                .collect::<Vec<String>>()
                .contains(&h.to_owned().to_string().to_lowercase())
        })
    {
        gateway_response
            .insert_header((header_name.clone(), header_value.clone()));
    }

    if let Some(size) = downstream_response
        .headers()
        .get("content-length")
        .map(|h| h.to_str().unwrap_or("0").parse::<u64>().unwrap_or(0))
    {
        span.record("myc.router.res_size", &Some(size));
    }

    Ok(gateway_response)
}
