//! The local web service: the device API plus the UI, which is compiled into the binary.
//!
//! It binds to 127.0.0.1 only. The one thing it keeps is the history of phones read on this
//! computer (see `history.rs`).

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{delete, get, post};
use rust_embed::Embed;
use serde_json::json;

use crate::history::History;
use crate::monitor::{Monitor, valid_udid};
use crate::reading::build_reading;
use crate::tools;

/// What `GET /api/ping` answers, so a second launch can tell this app from something else that
/// happens to hold the port.
pub const PING_APP: &str = "iphone-inspector";

#[derive(Embed)]
#[folder = "web/dist"]
struct Ui;

#[derive(Clone)]
pub struct AppState {
    pub monitor: Arc<Monitor>,
    pub history: Arc<History>,
}

pub async fn serve(listener: tokio::net::TcpListener, state: AppState) -> std::io::Result<()> {
    let app = Router::new()
        .route("/api/ping", get(|| async { Json(json!({ "app": PING_APP, "version": env!("CARGO_PKG_VERSION") })) }))
        .route("/api/devices", get(devices))
        .route("/api/devices/{udid}/pair", post(pair))
        .route("/api/devices/{udid}/read", get(read))
        .route("/api/history", get(history_list))
        .route("/api/history/{udid}", get(history_get))
        .route("/api/history/{udid}", delete(history_remove))
        .fallback(ui)
        .layer(middleware::from_fn(local_only))
        .with_state(state);
    axum::serve(listener, app).await
}

fn error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}

fn is_local(host: &str) -> bool {
    let name = if let Some(rest) = host.strip_prefix('[') { rest.split(']').next().unwrap_or("") } else { host.split(':').next().unwrap_or("") };
    matches!(name, "127.0.0.1" | "localhost" | "::1")
}

/// Binding to loopback keeps the network out, but not a web page open in the same browser. The
/// Host check stops DNS rebinding (a hostile domain re-pointed at 127.0.0.1); the Origin check stops
/// another site's page from posting to the API, e.g. to start pairing.
async fn local_only(headers: HeaderMap, request: Request, next: Next) -> Response {
    let host_ok = headers.get(header::HOST).and_then(|h| h.to_str().ok()).is_some_and(is_local);
    let origin_ok = match headers.get(header::ORIGIN).map(|o| o.to_str()) {
        None => true,
        Some(Ok(origin)) => origin.strip_prefix("http://").is_some_and(is_local),
        Some(Err(_)) => false,
    };
    if host_ok && origin_ok { next.run(request).await } else { error(StatusCode::FORBIDDEN, "Only this computer can use iPhone Inspector.") }
}

async fn devices(State(state): State<AppState>) -> Response {
    Json(state.monitor.refresh().await).into_response()
}

async fn pair(State(state): State<AppState>, Path(udid): Path<String>) -> Response {
    if !valid_udid(&udid) {
        return error(StatusCode::BAD_REQUEST, "Invalid device ID.");
    }
    match tools::pair(&udid).await {
        Ok(message) => {
            state.monitor.forget(&udid).await;
            Json(json!({ "message": message })).into_response()
        }
        Err(e) => error(StatusCode::BAD_REQUEST, e.message),
    }
}

/// `?disk=1` adds the disk-usage domains, which iOS can take the best part of a minute to compute on
/// a phone it has just seen. The UI reads without them first so the page opens at once.
///
/// The full read - the one with disk usage - is what goes into the history: that is the point at
/// which everything the phone will say has been gathered.
async fn read(State(state): State<AppState>, Path(udid): Path<String>, Query(query): Query<HashMap<String, String>>) -> Response {
    if !valid_udid(&udid) {
        return error(StatusCode::BAD_REQUEST, "Invalid device ID.");
    }
    let full = query.get("disk").is_some_and(|v| v == "1");
    match tools::collect(&udid, full).await {
        Ok(src) => {
            let reading = build_reading(&udid, &src);
            if full && let Ok(value) = serde_json::to_value(&reading) {
                state.history.save(&udid, value).await;
            }
            Json(reading).into_response()
        }
        Err(e) => error(StatusCode::INTERNAL_SERVER_ERROR, e.message),
    }
}

async fn history_list(State(state): State<AppState>) -> Response {
    Json(json!({ "devices": state.history.list().await })).into_response()
}

async fn history_get(State(state): State<AppState>, Path(udid): Path<String>) -> Response {
    match state.history.get(&udid).await {
        Some(reading) => Json(reading).into_response(),
        None => error(StatusCode::NOT_FOUND, "This iPhone is not in the history."),
    }
}

async fn history_remove(State(state): State<AppState>, Path(udid): Path<String>) -> Response {
    if state.history.remove(&udid).await { StatusCode::NO_CONTENT.into_response() } else { error(StatusCode::NOT_FOUND, "This iPhone is not in the history.") }
}

/// The built UI. Hashed assets are cached for good; index.html never is, so a new build shows up
/// on the next reload. Every other path is the app itself, which routes on the URL hash.
async fn ui(method: Method, uri: axum::http::Uri) -> Response {
    if method != Method::GET && method != Method::HEAD {
        return error(StatusCode::METHOD_NOT_ALLOWED, "Method not allowed.");
    }
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    let (path, file) = match Ui::get(path) {
        Some(file) => (path, file),
        None if path.starts_with("api/") || path.starts_with("assets/") => return error(StatusCode::NOT_FOUND, "Not found."),
        None => match Ui::get("index.html") {
            Some(file) => ("index.html", file),
            None => return error(StatusCode::NOT_FOUND, "The UI was not built into this binary."),
        },
    };

    let cache = if path.starts_with("assets/") { "public, max-age=31536000, immutable" } else { "no-cache" };
    let mime = file.metadata.mimetype().to_string();
    (
        [
            (header::CONTENT_TYPE, HeaderValue::from_str(&mime).unwrap_or(HeaderValue::from_static("application/octet-stream"))),
            (header::CACHE_CONTROL, HeaderValue::from_static(cache)),
        ],
        file.data,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::is_local;

    #[test]
    fn local_hosts() {
        assert!(is_local("127.0.0.1:3820"));
        assert!(is_local("localhost:5173"));
        assert!(is_local("[::1]:3820"));
        assert!(!is_local("evil.example:3820"));
        assert!(!is_local("127.0.0.1.evil.example"));
    }
}
