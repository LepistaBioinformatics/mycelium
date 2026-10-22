use super::stream_request_to_downstream::DownstreamResponse;

use actix_web::web::{Bytes, Payload};
use awc::{
    error::{ConnectError, SendRequestError},
    ClientRequest, FrozenClientRequest,
};
use myc_core::domain::dtos::{
    http::HttpMethod, route_retry_policy::RouteRetryPolicy,
};

type SendResult = Result<DownstreamResponse, SendRequestError>;

/// Send the request to the downstream service, retrying transient failures.
///
/// Requests are retried only when the route has a retry policy that applies
/// to the method and the body is buffered. Each attempt waits the policy
/// back-off first. The last attempt's outcome is returned as-is, so callers
/// see the same status or error they would without the policy.
///
/// Worst-case latency is `maxAttempts * timeout` plus the back-off delays.
#[tracing::instrument(name = "send_to_downstream_with_retry", skip_all)]
pub(super) async fn send_to_downstream_with_retry(
    downstream_request: ClientRequest,
    payload: Option<Payload>,
    body: Option<Bytes>,
    retry: Option<&RouteRetryPolicy>,
    method: &HttpMethod,
) -> SendResult {
    let Some(body) = body else {
        return send_stream_once(downstream_request, payload).await;
    };

    let Some(policy) = retry.filter(|policy| policy.applies_to(method)) else {
        return downstream_request.send_body(body).await;
    };

    send_with_policy(downstream_request.freeze()?, body, policy, method).await
}

async fn send_stream_once(
    downstream_request: ClientRequest,
    payload: Option<Payload>,
) -> SendResult {
    let Some(payload) = payload else {
        return Err(SendRequestError::Custom(
            "request body is neither buffered nor streamed".into(),
            Box::new("missing payload"),
        ));
    };

    downstream_request.send_stream(payload).await
}

async fn send_with_policy(
    request: FrozenClientRequest,
    body: Bytes,
    policy: &RouteRetryPolicy,
    method: &HttpMethod,
) -> SendResult {
    let mut attempt: u8 = 1;

    loop {
        let result = request.send_body(body.clone()).await;

        if attempt >= policy.max_attempts {
            return result;
        }

        let Some(reason) = retry_reason(&result, policy, method) else {
            return result;
        };

        let delay = policy.delay_for(u32::from(attempt));

        tracing::warn!(
            attempt,
            max_attempts = policy.max_attempts,
            reason,
            delay_ms = delay.as_millis() as u64,
            "Retrying downstream request"
        );

        actix_web::rt::time::sleep(delay).await;
        attempt += 1;
    }
}

/// Why the attempt should be repeated, or `None` when its outcome is final.
///
/// Connection failures never reached the service, so they are retried for
/// every method the policy applies to. Failures after the request was sent
/// may have had side effects and are retried only for idempotent methods.
fn retry_reason(
    result: &SendResult,
    policy: &RouteRetryPolicy,
    method: &HttpMethod,
) -> Option<String> {
    match result {
        Ok(response) => {
            let status = response.status().as_u16();
            policy
                .retries_status(status)
                .then(|| format!("status {status}"))
        }
        Err(SendRequestError::Connect(error)) => connect_retry_reason(error),
        Err(
            error @ (SendRequestError::Send(_)
            | SendRequestError::Response(_)
            | SendRequestError::Timeout),
        ) => RouteRetryPolicy::is_idempotent(method).then(|| error.to_string()),
        Err(_) => None,
    }
}

fn connect_retry_reason(error: &ConnectError) -> Option<String> {
    match error {
        ConnectError::SslIsNotSupported | ConnectError::SslError(_) => None,
        error => Some(format!("connect: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{web, App, HttpResponse, HttpServer};
    use std::{
        net::TcpListener,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::{Duration, Instant},
    };

    /// Downstream that answers 503 for the first `failures` hits, then 200.
    fn start_flaky_downstream(failures: usize) -> (String, Arc<AtomicUsize>) {
        let hits = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server_hits = hits.clone();

        let server = HttpServer::new(move || {
            let hits = server_hits.clone();
            App::new().default_service(web::to(move |body: Bytes| {
                let hits = hits.clone();
                async move {
                    let hit = hits.fetch_add(1, Ordering::SeqCst);
                    if hit < failures {
                        return HttpResponse::ServiceUnavailable().finish();
                    }
                    HttpResponse::Ok().body(body)
                }
            }))
        })
        .workers(1)
        .listen(listener)
        .unwrap()
        .run();

        actix_web::rt::spawn(server);

        (format!("http://{address}/"), hits)
    }

    fn policy(
        max_attempts: u8,
        retry_non_idempotent: bool,
    ) -> RouteRetryPolicy {
        toml::from_str(&format!(
            "maxAttempts = {max_attempts}\nbaseDelayMs = 50\n\
             maxDelayMs = 1000\nretryNonIdempotent = {retry_non_idempotent}"
        ))
        .unwrap()
    }

    async fn send(
        url: &str,
        method: HttpMethod,
        policy: Option<&RouteRetryPolicy>,
    ) -> SendResult {
        let client = awc::Client::default();
        let request = client.request(
            awc::http::Method::from_bytes(method.to_string().as_bytes())
                .unwrap(),
            url,
        );

        send_to_downstream_with_retry(
            request,
            None,
            Some(Bytes::from_static(b"payload")),
            policy,
            &method,
        )
        .await
    }

    #[actix_web::test]
    async fn retries_until_the_downstream_recovers() {
        let (url, hits) = start_flaky_downstream(2);
        let policy = policy(3, false);
        let started = Instant::now();

        let mut response =
            send(&url, HttpMethod::Put, Some(&policy)).await.unwrap();

        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(response.body().await.unwrap(), "payload");
        assert_eq!(hits.load(Ordering::SeqCst), 3);
        // 50ms * 1² + 50ms * 2²
        assert!(started.elapsed() >= Duration::from_millis(250));
    }

    #[actix_web::test]
    async fn returns_the_last_response_when_attempts_run_out() {
        let (url, hits) = start_flaky_downstream(10);
        let policy = policy(2, false);

        let response =
            send(&url, HttpMethod::Get, Some(&policy)).await.unwrap();

        assert_eq!(response.status().as_u16(), 503);
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[actix_web::test]
    async fn does_not_retry_non_idempotent_methods_by_default() {
        let (url, hits) = start_flaky_downstream(1);
        let policy = policy(3, false);

        let response =
            send(&url, HttpMethod::Post, Some(&policy)).await.unwrap();

        assert_eq!(response.status().as_u16(), 503);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[actix_web::test]
    async fn retries_non_idempotent_methods_when_opted_in() {
        let (url, hits) = start_flaky_downstream(1);
        let policy = policy(3, true);

        let response =
            send(&url, HttpMethod::Post, Some(&policy)).await.unwrap();

        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[actix_web::test]
    async fn sends_once_without_a_policy() {
        let (url, hits) = start_flaky_downstream(1);

        let response = send(&url, HttpMethod::Get, None).await.unwrap();

        assert_eq!(response.status().as_u16(), 503);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[actix_web::test]
    async fn retries_connection_failures() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        drop(listener);
        let policy = policy(2, false);
        let started = Instant::now();

        let result = send(&url, HttpMethod::Get, Some(&policy)).await;

        assert!(matches!(result, Err(SendRequestError::Connect(_))));
        assert!(started.elapsed() >= Duration::from_millis(50));
    }
}
