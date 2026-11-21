use myc_http_tools::settings::PEER_RESPONSE_MAX_BYTES;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use reqwest::Response;

/// The body of a 2xx peer response, refused past `PEER_RESPONSE_MAX_BYTES`
pub(crate) async fn read_peer_response(
    response: Response,
) -> Result<Vec<u8>, MappedErrors> {
    let status = response.status();
    if !status.is_success() {
        return execution_err(format!("peer answered {status}")).as_error();
    }

    let mut response = response;
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| execution_err(format!("peer body unreadable: {e}")))?
    {
        if body.len() + chunk.len() > PEER_RESPONSE_MAX_BYTES {
            return execution_err("peer response too large").as_error();
        }
        body.extend_from_slice(&chunk);
    }

    Ok(body)
}
