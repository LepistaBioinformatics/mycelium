use actix_web::{
    dev::{Decompress, Payload as DevPayload},
    web::Payload,
};
use awc::{
    error::{ConnectError, SendRequestError},
    ClientRequest, ClientResponse,
};
use myc_http_tools::responses::GatewayError;

pub(super) type DownstreamResponse = ClientResponse<Decompress<DevPayload>>;

/// Stream the request to the downstream service
///
/// This function streams the request to the downstream service.
///
/// Returns the binding response and the client response. Downstream response
/// contains the route response body and important headers, where the client
/// response contains the gateway response headers.
///
#[tracing::instrument(name = "stream_request_to_downstream", skip_all)]
pub(super) async fn stream_request_to_downstream(
    req: ClientRequest,
    payload: Payload,
) -> Result<DownstreamResponse, GatewayError> {
    let downstream_response: DownstreamResponse =
        match req.send_stream(payload).await {
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

                    return Err(GatewayError::InternalServerError(
                        String::from("Unexpected error on route request"),
                    ));
                }
                SendRequestError::Url(e) => {
                    tracing::warn!("Error on route/url to service: {e}");

                    return Err(GatewayError::InternalServerError(
                        String::from(format!("{e}")),
                    ));
                }
                err => {
                    tracing::warn!("Error on route/stream to service: {err}");

                    return Err(GatewayError::InternalServerError(
                        String::from(format!("{err}")),
                    ));
                }
            },
            Ok(res) => res,
        };

    Ok(downstream_response)
}
