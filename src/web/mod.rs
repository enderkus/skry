//! Read-only web dashboard with live updates over server-sent events, and a
//! Prometheus-compatible `/metrics` endpoint.
//!
//! Binds to `127.0.0.1` by default. Binding to any other address requires an
//! access token, which clients present as `Authorization: Bearer <token>`,
//! as `?token=<token>` (the dashboard then keeps it in an HttpOnly cookie),
//! or in the `skry_token` cookie. Only GET routes exist.

pub mod metrics;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::Router;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use futures::Stream;
use thiserror::Error;
use tokio::sync::watch;

use crate::config::Secret;
use crate::engine::FleetState;

const INDEX_HTML: &str = include_str!("assets/index.html");
const APP_JS: &str = include_str!("assets/app.js");
const STYLE_CSS: &str = include_str!("assets/style.css");
const COOKIE: &str = "skry_token";

#[derive(Debug, Error)]
pub enum WebError {
    #[error("invalid bind address {0:?}")]
    BadAddress(String),
    #[error(
        "refusing to bind to non-loopback address {0} without an access token; set [web] token in the config or SKRY_WEB_TOKEN"
    )]
    TokenRequired(String),
    #[error("cannot bind {addr}: {source}")]
    Bind {
        addr: String,
        source: std::io::Error,
    },
    #[error("web server: {0}")]
    Serve(std::io::Error),
}

#[derive(Clone)]
struct AppState {
    fleet: Arc<RwLock<FleetState>>,
    changes: watch::Receiver<u64>,
    token: Option<Arc<Secret>>,
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn query_token(req: &Request) -> Option<String> {
    req.uri().query()?.split('&').find_map(|kv| {
        kv.strip_prefix("token=").map(|v| {
            // Tokens are expected to be URL-safe; decode %XX minimally.
            let mut out = Vec::new();
            let b = v.as_bytes();
            let mut i = 0;
            while i < b.len() {
                if b[i] == b'%'
                    && i + 2 < b.len()
                    && let Ok(x) = u8::from_str_radix(&v[i + 1..i + 3], 16)
                {
                    out.push(x);
                    i += 3;
                    continue;
                }
                out.push(if b[i] == b'+' { b' ' } else { b[i] });
                i += 1;
            }
            String::from_utf8_lossy(&out).into_owned()
        })
    })
}

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|c| c.trim().strip_prefix("skry_token=").map(str::to_string))
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|t| t.trim().to_string())
}

async fn auth(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let Some(expected) = state.token.clone() else {
        return next.run(req).await;
    };
    let ok = |t: &str| constant_time_eq(t.as_bytes(), expected.expose().as_bytes());
    if bearer_token(req.headers()).is_some_and(|t| ok(&t))
        || cookie_token(req.headers()).is_some_and(|t| ok(&t))
    {
        return next.run(req).await;
    }
    if query_token(&req).is_some_and(|t| ok(&t)) {
        let mut res = next.run(req).await;
        let cookie = format!(
            "{COOKIE}={}; HttpOnly; SameSite=Strict; Path=/",
            expected.expose()
        );
        if let Ok(v) = HeaderValue::from_str(&cookie) {
            res.headers_mut().insert(header::SET_COOKIE, v);
        }
        return res;
    }
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Bearer")],
        "access token required\n",
    )
        .into_response()
}

async fn security_headers(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; img-src 'self' data:; frame-ancestors 'none'",
        ),
    );
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    h.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    res
}

fn asset(content_type: &'static str, body: &'static str) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
        .into_response()
}

fn json_response(body: String) -> Response {
    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}

fn fleet_json(state: &AppState) -> String {
    let f = state.fleet.read().expect("fleet lock");
    serde_json::to_string(&*f).unwrap_or_else(|_| "{}".into())
}

async fn api_fleet(State(state): State<AppState>) -> Response {
    json_response(fleet_json(&state))
}

async fn api_host(State(state): State<AppState>, Path(name): Path<String>) -> Response {
    let f = state.fleet.read().expect("fleet lock");
    match f.host(&name) {
        Some(h) => json_response(serde_json::to_string(h).unwrap_or_default()),
        None => (StatusCode::NOT_FOUND, "unknown host\n").into_response(),
    }
}

async fn prometheus(State(state): State<AppState>) -> Response {
    let body = metrics::render(&state.fleet.read().expect("fleet lock"));
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
        .into_response()
}

async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let initial = fleet_json(&state);
    let stream = futures::stream::unfold((state, Some(initial)), |(mut state, first)| async move {
        if let Some(json) = first {
            return Some((
                Ok(Event::default().event("fleet").data(json)),
                (state, None),
            ));
        }
        state.changes.changed().await.ok()?;
        // Coalesce bursts: at most one update per 500 ms.
        tokio::time::sleep(Duration::from_millis(500)).await;
        state.changes.borrow_and_update();
        let json = fleet_json(&state);
        Some((
            Ok(Event::default().event("fleet").data(json)),
            (state, None),
        ))
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/",
            get(|| async { asset("text/html; charset=utf-8", INDEX_HTML) }),
        )
        .route(
            "/assets/app.js",
            get(|| async { asset("text/javascript; charset=utf-8", APP_JS) }),
        )
        .route(
            "/assets/style.css",
            get(|| async { asset("text/css; charset=utf-8", STYLE_CSS) }),
        )
        .route("/api/fleet", get(api_fleet))
        .route("/api/host/{name}", get(api_host))
        .route("/api/events", get(events))
        .route("/metrics", get(prometheus))
        .route("/healthz", get(|| async { "ok\n" }))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

/// Binds the listener, enforcing the token rule for non-loopback addresses.
pub async fn bind(addr: &str, token: Option<&Secret>) -> Result<tokio::net::TcpListener, WebError> {
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host(addr)
        .await
        .map_err(|_| WebError::BadAddress(addr.to_string()))?
        .collect();
    if addrs.is_empty() {
        return Err(WebError::BadAddress(addr.to_string()));
    }
    if token.is_none() && addrs.iter().any(|a| !a.ip().is_loopback()) {
        return Err(WebError::TokenRequired(addr.to_string()));
    }
    tokio::net::TcpListener::bind(&addrs[..])
        .await
        .map_err(|source| WebError::Bind {
            addr: addr.to_string(),
            source,
        })
}

/// Serves the dashboard until `shutdown` resolves.
pub async fn serve(
    listener: tokio::net::TcpListener,
    fleet: Arc<RwLock<FleetState>>,
    changes: watch::Receiver<u64>,
    token: Option<Secret>,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<(), WebError> {
    let state = AppState {
        fleet,
        changes,
        token: token.map(Arc::new),
    };
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(WebError::Serve)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn start(token: Option<&str>) -> (String, watch::Sender<u64>, Arc<RwLock<FleetState>>) {
        let now = chrono::Utc::now();
        let fleet = Arc::new(RwLock::new(crate::tui::demo::fleet(now, 0.0)));
        let (tx, rx) = watch::channel(0);
        let secret = token.map(Secret::new);
        let listener = bind("127.0.0.1:0", secret.as_ref()).await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(serve(
            listener,
            fleet.clone(),
            rx,
            secret,
            std::future::pending(),
        ));
        (format!("http://{addr}"), tx, fleet)
    }

    #[tokio::test]
    async fn serves_dashboard_api_and_metrics() {
        let (base, _tx, _f) = start(None).await;
        let c = reqwest::Client::new();
        let index = c.get(format!("{base}/")).send().await.unwrap();
        assert_eq!(index.status(), 200);
        assert!(
            index.headers()["content-security-policy"]
                .to_str()
                .unwrap()
                .contains("default-src 'self'")
        );
        assert!(index.text().await.unwrap().contains("<title>skry</title>"));
        let fleet: serde_json::Value = c
            .get(format!("{base}/api/fleet"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(fleet["hosts"].as_array().unwrap().len(), 8);
        let host = c.get(format!("{base}/api/host/db-1")).send().await.unwrap();
        assert_eq!(host.status(), 200);
        assert_eq!(
            c.get(format!("{base}/api/host/nope"))
                .send()
                .await
                .unwrap()
                .status(),
            404
        );
        let m = c
            .get(format!("{base}/metrics"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(m.contains("skry_up{host=\"legacy-app\"} 0"));
        assert!(m.contains("skry_cpu_usage_percent{host=\"db-1\"}"));
        // Read-only: no write methods.
        let post = c.post(format!("{base}/api/fleet")).send().await.unwrap();
        assert_eq!(post.status(), 405);
    }

    #[tokio::test]
    async fn token_is_enforced() {
        let (base, _tx, _f) = start(Some("s3cret")).await;
        let c = reqwest::Client::new();
        assert_eq!(
            c.get(format!("{base}/metrics"))
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        assert_eq!(
            c.get(format!("{base}/metrics"))
                .bearer_auth("wrong")
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        assert_eq!(
            c.get(format!("{base}/metrics"))
                .bearer_auth("s3cret")
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        let res = c.get(format!("{base}/?token=s3cret")).send().await.unwrap();
        assert_eq!(res.status(), 200);
        let cookie = res.headers()["set-cookie"].to_str().unwrap().to_string();
        assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"));
        let with_cookie = c
            .get(format!("{base}/api/fleet"))
            .header("cookie", cookie.split(';').next().unwrap())
            .send()
            .await
            .unwrap();
        assert_eq!(with_cookie.status(), 200);
    }

    #[tokio::test]
    async fn non_loopback_requires_token() {
        let err = bind("0.0.0.0:0", None).await.unwrap_err();
        assert!(matches!(err, WebError::TokenRequired(_)));
        let token = Secret::new("t");
        assert!(bind("0.0.0.0:0", Some(&token)).await.is_ok());
        assert!(matches!(
            bind("not an address", None).await,
            Err(WebError::BadAddress(_))
        ));
    }

    #[tokio::test]
    async fn sse_streams_updates() {
        use futures::StreamExt;
        let (base, tx, fleet) = start(None).await;
        let res = reqwest::Client::new()
            .get(format!("{base}/api/events"))
            .send()
            .await
            .unwrap();
        assert_eq!(res.headers()["content-type"], "text/event-stream");
        let mut body = res.bytes_stream();
        let mut buf = String::new();
        while !buf.contains("\n\n") {
            buf.push_str(&String::from_utf8_lossy(
                &body.next().await.unwrap().unwrap(),
            ));
        }
        assert!(buf.starts_with("event: fleet\ndata: {"));
        fleet.write().unwrap().generation = 42;
        tx.send(42).unwrap();
        let mut next = String::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !next.contains("\"generation\":42") {
            let chunk = tokio::time::timeout_at(deadline, body.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            next.push_str(&String::from_utf8_lossy(&chunk));
        }
    }

    #[test]
    fn token_parsing() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            HeaderValue::from_static("a=1; skry_token=xyz"),
        );
        assert_eq!(cookie_token(&h).as_deref(), Some("xyz"));
        let req = Request::builder()
            .uri("/?a=1&token=a%2Bb")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(query_token(&req).as_deref(), Some("a+b"));
    }
}
