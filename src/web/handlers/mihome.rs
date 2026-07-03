use axum::{
    body::Body,
    extract::{OriginalUri, State},
    http::{Request, StatusCode},
    response::Response,
};
use std::sync::Arc;

use crate::error::AppError;
use crate::state::AppState;

const MIHOME_TARGET: &str = "http://127.0.0.1:7123";

pub async fn mihome_proxy(
    State(_state): State<Arc<AppState>>,
    OriginalUri(original_uri): OriginalUri,
    req: Request<Body>,
) -> Result<Response, AppError> {
    let path_and_query = original_uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/mihome");

    let target_path = if path_and_query.starts_with("/mihome?") || path_and_query == "/mihome" || path_and_query == "/mihome/" {
        // API requests (with query params) or page requests (without query params)
        // /mihome?type=login -> /mihome?type=login (API)
        // /mihome -> /webui (page)
        if path_and_query.contains('?') {
            // Has query parameters - this is an API request, forward to /mihome
            path_and_query.to_string()
        } else {
            // No query parameters - this is a page request, forward to /webui
            "/webui".to_string()
        }
    } else if let Some(rest) = path_and_query.strip_prefix("/mihome/") {
        if rest.contains('?') || !rest.is_empty() {
            // Sub-path with or without query params - forward to /mihome/
            format!("/mihome/{}", rest)
        } else {
            "/webui".to_string()
        }
    } else {
        // Fallback - strip /mihome prefix and prepend /webui
        format!("/webui{}", &path_and_query["/mihome".len()..])
    };

    let target_url = format!("{}{}", MIHOME_TARGET, target_path);

    let client = reqwest::Client::new();
    let method = req.method().clone();
    let headers = req.headers().clone();
    let body_bytes = axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024)
        .await
        .unwrap_or_default();

    let mut proxy_req = client.request(method, &target_url);

    for (name, value) in headers.iter() {
        let name_lower = name.as_str().to_lowercase();
        if matches!(
            name_lower.as_str(),
            "connection"
                | "keep-alive"
                | "transfer-encoding"
                | "upgrade"
                | "host"
                | "accept-encoding"
        ) {
            continue;
        }
        if let Ok(v) = value.to_str() {
            proxy_req = proxy_req.header(name.as_str(), v);
        }
    }

    proxy_req = proxy_req.header("host", "127.0.0.1:7123");
    proxy_req = proxy_req.body(body_bytes);

    let proxy_resp = proxy_req
        .send()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("MiHome service unavailable: {}", e)))?;

    let status = StatusCode::from_u16(proxy_resp.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);

    let resp_headers = proxy_resp.headers().clone();
    let resp_body = proxy_resp
        .bytes()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read MiHome response: {}", e)))?;

    let mut builder = Response::builder().status(status);

    for (name, value) in resp_headers.iter() {
        let name_lower = name.as_str().to_lowercase();
        if !matches!(
            name_lower.as_str(),
            "connection" | "keep-alive" | "transfer-encoding"
        ) {
            builder = builder.header(name.as_str(), value.as_bytes());
        }
    }

    builder
        .body(Body::from(resp_body))
        .map_err(|e| AppError::Internal(format!("Failed to build response: {}", e)))
}