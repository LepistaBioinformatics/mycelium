use crate::read_peer_response::read_peer_response;

use async_trait::async_trait;
use myc_core::domain::{
    dtos::federation::InstanceSelfDescription, entities::PeerClient,
};
use myc_http_tools::settings::{
    INSTANCE_SELF_DESCRIPTION_PATH, PEER_REQUEST_SIGNATURE_KEY,
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use reqwest::{header::CONTENT_TYPE, Client};
use std::time::Duration;

/// `PeerClient` over HTTP, with the timeouts of `[core.federation]`
pub struct HttpPeerClient {
    client: Client,
}

impl HttpPeerClient {
    /// `peer_timeout_ms` bounds both the connection and the whole call
    pub fn new(peer_timeout_ms: u64) -> Result<Self, MappedErrors> {
        let timeout = Duration::from_millis(peer_timeout_ms);
        let client = Client::builder()
            .connect_timeout(timeout)
            .timeout(timeout)
            .build()
            .map_err(|e| execution_err(format!("peer client: {e}")))?;

        Ok(Self { client })
    }
}

#[async_trait]
impl PeerClient for HttpPeerClient {
    #[tracing::instrument(name = "peer_fetch_self_description", skip_all)]
    async fn fetch_self_description(
        &self,
        base_url: &str,
    ) -> Result<InstanceSelfDescription, MappedErrors> {
        let url = format!("{}{INSTANCE_SELF_DESCRIPTION_PATH}", trim(base_url));
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| execution_err(format!("peer unreachable: {e}")))?;

        serde_json::from_slice(&read_peer_response(response).await?).map_err(
            |e| execution_err(format!("peer self-description invalid: {e}")),
        )
    }

    #[tracing::instrument(name = "peer_post_signed", skip_all)]
    async fn post_signed(
        &self,
        base_url: &str,
        path: &str,
        body: Vec<u8>,
        signature: &str,
    ) -> Result<Vec<u8>, MappedErrors> {
        let response = self
            .client
            .post(format!("{}{path}", trim(base_url)))
            .header(CONTENT_TYPE, "application/json")
            .header(PEER_REQUEST_SIGNATURE_KEY, signature)
            .body(body)
            .send()
            .await
            .map_err(|e| execution_err(format!("peer unreachable: {e}")))?;

        read_peer_response(response).await
    }
}

fn trim(base_url: &str) -> &str {
    base_url.trim_end_matches('/')
}

#[cfg(test)]
mod tests {
    use super::*;
    use myc_http_tools::settings::PEER_RESPONSE_MAX_BYTES;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    use uuid::Uuid;

    /// A one-shot HTTP server: answers the first request with `response`,
    /// or never answers when `response` is `None`. Returns its base URL and
    /// the raw request it received.
    async fn serve_once(
        response: Option<String>,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());

        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0u8; 8192];
            let read = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();

            match response {
                Some(response) => {
                    socket.write_all(response.as_bytes()).await.unwrap();
                }
                None => tokio::time::sleep(Duration::from_secs(5)).await,
            }

            request
        });

        (base_url, handle)
    }

    fn http(status: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\n\
             content-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    fn description() -> String {
        serde_json::json!({
            "instanceId": Uuid::nil(),
            "baseUrl": "https://myc-02.example.com",
            "version": "9.0.1",
            "assertionKinds": [],
            "keys": [],
        })
        .to_string()
    }

    #[tokio::test]
    async fn fetches_the_self_description() {
        let (base_url, server) =
            serve_once(Some(http("200 OK", &description()))).await;
        let client = HttpPeerClient::new(2_000).unwrap();

        let fetched = client
            .fetch_self_description(&format!("{base_url}/"))
            .await
            .unwrap();

        assert_eq!(fetched.base_url, "https://myc-02.example.com");
        assert!(server
            .await
            .unwrap()
            .starts_with(&format!("GET {INSTANCE_SELF_DESCRIPTION_PATH} ")));
    }

    #[tokio::test]
    async fn posts_the_body_with_the_signature_header() {
        let (base_url, server) =
            serve_once(Some(http("200 OK", r#"{"ok":true}"#))).await;
        let client = HttpPeerClient::new(2_000).unwrap();

        let body = client
            .post_signed(
                &base_url,
                "/federation/handshake",
                b"{}".to_vec(),
                "a.b.c",
            )
            .await
            .unwrap();
        let request = server.await.unwrap().to_lowercase();

        assert_eq!(body, br#"{"ok":true}"#);
        assert!(request.starts_with("post /federation/handshake "));
        assert!(
            request.contains(&format!("{PEER_REQUEST_SIGNATURE_KEY}: a.b.c"))
        );
    }

    #[tokio::test]
    async fn a_silent_peer_times_out() {
        let (base_url, _server) = serve_once(None).await;
        let client = HttpPeerClient::new(200).unwrap();
        let started = std::time::Instant::now();

        assert!(client.fetch_self_description(&base_url).await.is_err());
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn a_non_2xx_answer_is_an_error() {
        let (base_url, _server) =
            serve_once(Some(http("503 Service Unavailable", "{}"))).await;
        let client = HttpPeerClient::new(2_000).unwrap();

        let error = client
            .post_signed(&base_url, "/federation/handshake", vec![], "a.b.c")
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("503"), "{error}");
    }

    #[tokio::test]
    async fn an_oversized_answer_is_an_error() {
        let big = "x".repeat(PEER_RESPONSE_MAX_BYTES + 1);
        let (base_url, _server) = serve_once(Some(http("200 OK", &big))).await;
        let client = HttpPeerClient::new(2_000).unwrap();

        assert!(client.fetch_self_description(&base_url).await.is_err());
    }

    #[tokio::test]
    async fn an_unreachable_peer_is_an_error() {
        let client = HttpPeerClient::new(500).unwrap();

        assert!(client
            .fetch_self_description("http://127.0.0.1:9")
            .await
            .is_err());
    }
}
