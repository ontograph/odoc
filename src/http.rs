//! Folder-scoped Streamable HTTP MCP listener (ADR revision 10, HTTP-0).
//!
//! One loopback listener serves `/<folder>/mcp` for every `docs_share`
//! folder through the existing [`OntoDocServer`] handlers. rmcp runs in
//! stateless JSON mode: one POST carries one JSON-RPC message and gets one
//! bounded JSON response; there are no sessions, SSE streams or replay caches.
//! Authentication and TLS belong to the trusted ingress in front of it.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full, Limited, combinators::BoxBody};
use hyper_util::rt::{TokioIo, TokioTimer};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::cli::MAX_RESPONSE_BYTES;
use crate::mcp::{OntoDocServer, Registry, RegistryError};

/// Request body ceiling, enforced while streaming before JSON decoding.
pub const MAX_REQUEST_BYTES: usize = 128 * 1024;
/// Concurrent in-flight HTTP requests; excess is refused, never queued.
pub const MAX_IN_FLIGHT: usize = 256;
/// Process-wide actual blocking tool work.
pub const GLOBAL_WORK_PERMITS: usize = 4;
/// Actual blocking tool work per corpus.
pub const CORPUS_WORK_PERMITS: usize = 2;
/// Deadline for receiving a complete request head (hyper) and request body.
pub const READ_TIMEOUT: Duration = Duration::from_secs(30);

type Body = BoxBody<Bytes, Infallible>;
type Mcp = StreamableHttpService<OntoDocServer, NeverSessionManager>;

#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("{0}")]
    Registry(#[from] RegistryError),
    #[error("listen address must be loopback")]
    NotLoopback,
    #[error("listener failed")]
    Io(#[from] std::io::Error),
}

/// Routes `/<percent-encoded folder>/mcp` to that folder's MCP service.
pub struct Router {
    routes: Vec<(String, Mcp)>,
    in_flight: Arc<Semaphore>,
    /// Connections admitted; an accepted socket beyond it is closed at once.
    connections: Arc<Semaphore>,
    /// rmcp calls run here so a client disconnect cannot drop them midway.
    calls: TaskTracker,
}

impl Router {
    /// Discovers `docs_share` once; `authority` is the exact `Host` accepted.
    pub fn new(docs_share: &Path, authority: &str) -> Result<Self, HttpError> {
        let global = Arc::new(Semaphore::new(GLOBAL_WORK_PERMITS));
        let registry = Registry::discover_with(docs_share, CORPUS_WORK_PERMITS, Some(global))?;
        let config = StreamableHttpServerConfig::default()
            .with_stateful_mode(false)
            .with_json_response(true)
            .with_sse_keep_alive(None)
            .with_sse_retry(None)
            .with_allowed_hosts([authority.to_string()]);
        let mut routes: Vec<(String, Mcp)> = registry
            .servers()
            .map(|(name, server)| {
                let server = server.clone();
                let service = StreamableHttpService::new(
                    move || Ok(server.clone()),
                    Arc::new(NeverSessionManager::default()),
                    config.clone(),
                );
                (encode_segment(name), service)
            })
            .collect();
        routes.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(Self {
            routes,
            in_flight: Arc::new(Semaphore::new(MAX_IN_FLIGHT)),
            connections: Arc::new(Semaphore::new(MAX_IN_FLIGHT)),
            calls: TaskTracker::new(),
        })
    }

    /// Encoded endpoint paths, sorted.
    pub fn paths(&self) -> impl Iterator<Item = String> + '_ {
        self.routes
            .iter()
            .map(|(segment, _)| format!("/{segment}/mcp"))
    }

    /// One complete request: admission, routing, bounded body, bounded reply.
    pub async fn handle<B>(&self, request: Request<B>) -> Response<Body>
    where
        B: hyper::body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        let Ok(slot) = self.in_flight.clone().try_acquire_owned() else {
            return plain(StatusCode::SERVICE_UNAVAILABLE, "busy");
        };
        // Native-client profile: browser-originated requests are refused.
        if request.headers().contains_key(http::header::ORIGIN) {
            return plain(StatusCode::FORBIDDEN, "origin not allowed");
        }
        let Some(service) = self.lookup(request.uri().path()) else {
            return plain(StatusCode::NOT_FOUND, "not found");
        };
        if request.method() != http::Method::POST {
            return plain(StatusCode::METHOD_NOT_ALLOWED, "method not allowed");
        }
        let declared = request
            .headers()
            .get(http::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        if declared.is_some_and(|n| n > MAX_REQUEST_BYTES as u64) {
            return plain(StatusCode::PAYLOAD_TOO_LARGE, "request too large");
        }
        let (parts, body) = request.into_parts();
        let collect = Limited::new(body, MAX_REQUEST_BYTES).collect();
        let body = match tokio::time::timeout(READ_TIMEOUT, collect).await {
            Ok(Ok(collected)) => collected.to_bytes(),
            Ok(Err(_)) => return plain(StatusCode::PAYLOAD_TOO_LARGE, "request too large"),
            Err(_) => return plain(StatusCode::REQUEST_TIMEOUT, "request timeout"),
        };
        // rmcp's stateless transport only ends its per-request service task
        // after the response is delivered, so the call must run to completion
        // even if this future is dropped. The in-flight slot moves with it.
        let service = service.clone();
        let call = self.calls.spawn(async move {
            let _slot = slot;
            let response = service
                .handle(Request::from_parts(parts, Full::new(body)))
                .await;
            bound_response(response).await
        });
        call.await
            .unwrap_or_else(|_| plain(StatusCode::INTERNAL_SERVER_ERROR, "internal error"))
    }

    fn lookup(&self, path: &str) -> Option<&Mcp> {
        let segment = path.strip_prefix('/')?.strip_suffix("/mcp")?;
        self.routes
            .binary_search_by(|(s, _)| s.as_str().cmp(segment))
            .ok()
            .map(|i| &self.routes[i].1)
    }
}

/// Complete-body bound on the way out; an oversized body is a generic 500.
/// Transport-level refusals from rmcp keep their status but get a generic
/// body, because rmcp echoes parser and header details.
async fn bound_response(response: Response<Body>) -> Response<Body> {
    if !response.status().is_success() {
        return plain(response.status(), "request refused");
    }
    let (parts, body) = response.into_parts();
    match Limited::new(body, MAX_RESPONSE_BYTES).collect().await {
        Ok(collected) => Response::from_parts(parts, Full::new(collected.to_bytes()).boxed()),
        Err(_) => plain(
            StatusCode::INTERNAL_SERVER_ERROR,
            "response exceeds the byte limit",
        ),
    }
}

fn plain(status: StatusCode, message: &'static str) -> Response<Body> {
    let mut response = Response::new(Full::new(Bytes::from_static(message.as_bytes())).boxed());
    *response.status_mut() = status;
    response
}

/// Percent-encodes everything except RFC 3986 unreserved bytes, so each
/// folder name is exactly one lossless, case-preserving path segment.
pub fn encode_segment(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Binds a loopback listener for `docs_share` and serves until `shutdown`.
pub async fn serve(
    docs_share: &Path,
    addr: SocketAddr,
    shutdown: CancellationToken,
) -> Result<(), HttpError> {
    if !addr.ip().is_loopback() {
        return Err(HttpError::NotLoopback);
    }
    let listener = TcpListener::bind(addr).await?;
    let local = listener.local_addr()?;
    let router = Arc::new(Router::new(docs_share, &local.to_string())?);
    for path in router.paths() {
        eprintln!(
            "{}",
            serde_json::json!({ "endpoint": format!("http://{local}{path}") })
        );
    }
    run(listener, router, shutdown).await;
    Ok(())
}

/// Accept loop over a bound listener. On `shutdown` it stops accepting and
/// drains in-flight connections; blocking tool work keeps its permits until
/// it actually stops.
pub async fn run(listener: TcpListener, router: Arc<Router>, shutdown: CancellationToken) {
    let tracker = TaskTracker::new();
    loop {
        let stream = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => stream,
                Err(_) => continue,
            },
            () = shutdown.cancelled() => break,
        };
        // Fail-fast connection admission: excess sockets are closed unread.
        let Ok(admitted) = router.connections.clone().try_acquire_owned() else {
            drop(stream);
            continue;
        };
        let router = router.clone();
        let shutdown = shutdown.clone();
        tracker.spawn(async move {
            let _admitted = admitted;
            let service = hyper::service::service_fn(move |request| {
                let router = router.clone();
                async move { Ok::<_, Infallible>(router.handle(request).await) }
            });
            let connection = hyper::server::conn::http1::Builder::new()
                .timer(TokioTimer::new())
                .header_read_timeout(READ_TIMEOUT)
                .serve_connection(TokioIo::new(stream), service);
            let mut connection = std::pin::pin!(connection);
            tokio::select! {
                _ = connection.as_mut() => {}
                () = shutdown.cancelled() => {
                    connection.as_mut().graceful_shutdown();
                    let _ = connection.await;
                }
            }
        });
    }
    tracker.close();
    tracker.wait().await;
    router.calls.close();
    router.calls.wait().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(path: &str) -> Request<Full<Bytes>> {
        Request::builder()
            .method("POST")
            .uri(path)
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    #[tokio::test]
    async fn in_flight_cap_refuses_without_queueing() {
        let share = tempfile::tempdir().unwrap();
        std::fs::create_dir(share.path().join("a")).unwrap();
        let router = Router::new(share.path(), "127.0.0.1:1").unwrap();
        let held = router
            .in_flight
            .clone()
            .try_acquire_many_owned(MAX_IN_FLIGHT as u32)
            .unwrap();
        let response = router.handle(post("/a/mcp")).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        drop(held);
        // With room again, routing proceeds (unknown path here: 404).
        let response = router.handle(post("/zzz/mcp")).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(router.in_flight.available_permits(), MAX_IN_FLIGHT);
    }

    #[tokio::test]
    async fn dropped_request_still_finishes_and_releases_its_slot() {
        let share = tempfile::tempdir().unwrap();
        std::fs::create_dir(share.path().join("a")).unwrap();
        let router = Router::new(share.path(), "localhost").unwrap();
        let body = br#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
        let request = Request::builder()
            .method("POST")
            .uri("/a/mcp")
            .header("host", "localhost")
            .header("accept", "application/json, text/event-stream")
            .header("content-type", "application/json")
            .header("mcp-protocol-version", "2025-06-18")
            .body(Full::new(Bytes::from_static(body)))
            .unwrap();
        // Poll the handler until the call is spawned, then drop it, as a
        // disconnecting client does.
        let mut pending = Box::pin(router.handle(request));
        // One poll on a current-thread runtime: the spawned call has not run.
        let first = std::future::poll_fn(|cx| {
            std::task::Poll::Ready(pending.as_mut().poll(cx).is_pending())
        })
        .await;
        assert!(first, "the call must still be running");
        drop(pending);
        // The detached call still owns its in-flight slot until it finishes.
        assert_eq!(router.in_flight.available_permits(), MAX_IN_FLIGHT - 1);
        router.calls.close();
        tokio::time::timeout(Duration::from_secs(10), router.calls.wait())
            .await
            .expect("the detached call completes");
        assert_eq!(router.in_flight.available_permits(), MAX_IN_FLIGHT);
    }

    #[tokio::test]
    async fn outbound_body_is_bounded_completely() {
        let fits = Response::new(Full::new(Bytes::from(vec![b'x'; MAX_RESPONSE_BYTES])).boxed());
        let fits = bound_response(fits).await;
        assert_eq!(fits.status(), StatusCode::OK);
        let over =
            Response::new(Full::new(Bytes::from(vec![b'x'; MAX_RESPONSE_BYTES + 1])).boxed());
        let over = bound_response(over).await;
        assert_eq!(over.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = over.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"response exceeds the byte limit");
    }

    #[test]
    fn segments_encode_losslessly_and_case_sensitively() {
        // Hand-checked: space 0x20, '%' 0x25, '/' 0x2F, 'é' = C3 A9.
        assert_eq!(encode_segment("Rust-Book_1.0~x"), "Rust-Book_1.0~x");
        assert_eq!(encode_segment("a b%/é"), "a%20b%25%2F%C3%A9");
        assert_ne!(encode_segment("A"), encode_segment("a"));
    }
}
