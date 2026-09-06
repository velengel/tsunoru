mod api;
mod organizer_auth;
mod session;

use futures_util::StreamExt;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use sha2::{Digest, Sha256};
use worker::*;

const MAX_BODY_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy)]
struct ApiError {
    status: u16,
    code: &'static str,
}

type ApiResult<T> = std::result::Result<T, ApiError>;

impl ApiError {
    fn new(status: u16, code: &'static str) -> Self {
        Self { status, code }
    }

    fn invalid() -> Self {
        Self::new(400, "invalid_request")
    }
}

// Worker/D1 exception strings can contain SQL or payloads. Never expose or log them.
impl From<worker::Error> for ApiError {
    fn from(_: worker::Error) -> Self {
        Self::new(500, "internal_error")
    }
}

fn json_response(status: u16, value: &impl Serialize) -> ApiResult<Response> {
    Ok(Response::from_json(value)?.with_status(status))
}

fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn capability_valid(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|c| c.is_ascii_hexdigit())
}

fn identifier_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

fn name_valid(value: &str) -> bool {
    !value.is_empty() && value.chars().count() <= 100 && !value.chars().any(char::is_control)
}

fn header_capability(request: &Request, name: &str) -> ApiResult<String> {
    request
        .headers()
        .get(name)?
        .filter(|v| capability_valid(v))
        .ok_or(ApiError::new(403, "forbidden"))
}

fn valid_origin(origin: &str) -> bool {
    let Ok(url) = Url::parse(origin) else {
        return false;
    };
    (url.scheme() == "https"
        || (url.scheme() == "http" && matches!(url.host_str(), Some("localhost" | "127.0.0.1"))))
        && url.username().is_empty()
        && url.password().is_none()
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none()
        && url.origin().ascii_serialization() == origin
}

async fn body<T: DeserializeOwned>(request: &mut Request) -> ApiResult<T> {
    let content_type = request.headers().get("content-type")?.unwrap_or_default();
    if !content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .eq_ignore_ascii_case("application/json")
    {
        return Err(ApiError::new(415, "unsupported_media_type"));
    }
    if request
        .headers()
        .get("content-length")?
        .and_then(|v| v.parse::<u64>().ok())
        .is_some_and(|n| n > MAX_BODY_BYTES as u64)
    {
        return Err(ApiError::new(413, "payload_too_large"));
    }
    let mut stream = request.stream().map_err(|_| ApiError::invalid())?;
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ApiError::invalid())?;
        if chunk.len() > MAX_BODY_BYTES - bytes.len() {
            return Err(ApiError::new(413, "payload_too_large"));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| ApiError::invalid())
}

async fn route(mut request: Request, env: Env) -> ApiResult<Response> {
    let path = request.path();
    let method = request.method();
    if path == "/health" && method == Method::Get {
        return json_response(200, &json!({"status": "ok", "runtime": "rust-worker"}));
    }
    if !path.starts_with("/api/") {
        if matches!(method, Method::Get | Method::Head) {
            return Ok(Response::try_from(
                env.assets("ASSETS")?.fetch_request(request).await?,
            )?);
        }
        return Err(ApiError::new(404, "not_found"));
    }
    if path == "/api/staging/session" {
        return session::route(&mut request, &env).await;
    }
    if path == "/api/organizer/session" {
        return organizer_auth::route(&mut request, &env).await;
    }
    if path == "/api/organizer/config" && method == Method::Get {
        let client_id = env.var("GOOGLE_CLIENT_ID")?.to_string();
        return json_response(200, &json!({"client_id": client_id}));
    }
    let segments: Vec<_> = path.split('/').collect();
    let google_enabled = env
        .var("GOOGLE_CLIENT_ID")
        .map(|v| !v.to_string().trim().is_empty())
        .unwrap_or(false);
    let organizer_mutation = matches!(
        (&method, segments.as_slice()),
        (Method::Post, ["", "api", "events"])
            | (Method::Get, ["", "api", "events", _, "responses"])
            | (Method::Delete, ["", "api", "events", _])
    );
    if organizer_mutation {
        if google_enabled {
            organizer_auth::authorize(&request, &env)?;
        } else {
            session::authorize(&request, &env)?;
        }
    } else if !google_enabled {
        session::authorize(&request, &env)?;
    }
    match (method, segments.as_slice()) {
        (Method::Post, ["", "api", "events"]) => api::create_event(&mut request, &env).await,
        (Method::Get, ["", "api", "events", id]) if identifier_valid(id) => {
            api::get_event(id, &env).await
        }
        (Method::Post, ["", "api", "events", id, "responses"]) if identifier_valid(id) => {
            if google_enabled {
                let origin = env
                    .var("APP_ORIGIN")
                    .map_err(|_| ApiError::new(503, "auth_unavailable"))?
                    .to_string();
                if request.headers().get("origin")?.as_deref() != Some(&origin) {
                    return Err(ApiError::new(403, "origin_forbidden"));
                }
            }
            api::submit_response(id, &mut request, &env).await
        }
        (Method::Get, ["", "api", "events", id, "responses"]) if identifier_valid(id) => {
            api::get_responses(id, &request, &env).await
        }
        (Method::Delete, ["", "api", "events", id]) if identifier_valid(id) => {
            api::delete_event(id, &request, &env).await
        }
        _ => Err(ApiError::new(404, "not_found")),
    }
}

#[event(fetch)]
pub async fn fetch(request: Request, env: Env, _ctx: Context) -> Result<Response> {
    let private_response = request.path().starts_with("/api/") || request.path() == "/health";
    let mut response = match route(request, env).await {
        Ok(response) => response,
        Err(error) => {
            Response::from_json(&json!({"error": {"code": error.code}}))?.with_status(error.status)
        }
    };
    if private_response || response.status_code() >= 400 {
        response.headers_mut().set("Cache-Control", "no-store")?;
    }
    response
        .headers_mut()
        .set("X-Content-Type-Options", "nosniff")?;
    response
        .headers_mut()
        .set("Referrer-Policy", "no-referrer")?;
    response.headers_mut().set("X-Frame-Options", "DENY")?;
    response.headers_mut().set("Content-Security-Policy", "default-src 'self'; script-src 'self' https://accounts.google.com/gsi/client 'wasm-unsafe-eval'; frame-src https://accounts.google.com/gsi/; style-src 'self' 'unsafe-inline' https://accounts.google.com/gsi/style; img-src 'self' data:; connect-src 'self' https://accounts.google.com/gsi/; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'")?;
    if response.status_code() == 401 {
        response.headers_mut().set("WWW-Authenticate", "Bearer")?;
    }
    Ok(response)
}
