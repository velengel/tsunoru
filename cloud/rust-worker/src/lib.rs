#![allow(unused_must_use)]

mod api;
mod organizer_auth;
mod session;

use futures_util::StreamExt;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use sha2::{Digest, Sha256};
use worker::*;

const MAX_BODY_BYTES: usize = 64 * 1024;
const RETENTION_SECONDS: u64 = 30 * 24 * 60 * 60;

#[derive(Clone, Copy)]
struct ApiError {
    status: u16,
    code: &'static str,
    retry_after: Option<u64>,
}

type ApiResult<T> = std::result::Result<T, ApiError>;

impl ApiError {
    fn new(status: u16, code: &'static str) -> Self {
        Self {
            status,
            code,
            retry_after: None,
        }
    }

    fn invalid() -> Self {
        Self::new(400, "invalid_request")
    }
}

impl ApiError {
    fn rate_limited(retry_after: u64) -> Self {
        Self {
            status: 429,
            code: "rate_limited",
            retry_after: Some(retry_after),
        }
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

const RATE_WINDOW_SECONDS: u64 = 60;

fn rate_limit_for(operation: &str) -> u64 {
    match operation {
        "create" => 60,
        "response" => 120,
        "read" => 240,
        _ => 60,
    }
}

async fn enforce_rate_limit(request: &Request, env: &Env, operation: &str) -> ApiResult<()> {
    let source = request
        .headers()
        .get("cf-connecting-ip")?
        .unwrap_or_else(|| "unknown".to_owned());
    let source_hash = hash(&format!("tsunoru-rate-limit:v1\n{source}"));
    let now = Date::now().as_millis() / 1_000;
    let window_start = now - (now % RATE_WINDOW_SECONDS);
    let limit = rate_limit_for(operation);
    let db = env.d1("DB")?;
    #[derive(serde::Deserialize)]
    struct RateLimitRow {
        request_count: u64,
    }
    let result = db
        .batch(vec![
            db.prepare("INSERT INTO rate_limits(source_hash,route,window_start,request_count) VALUES(?1,?2,?3,1) ON CONFLICT(source_hash,route,window_start) DO UPDATE SET request_count=request_count+1")
                .bind(&[source_hash.clone().into(), operation.to_owned().into(), window_start.to_string().into()])?,
            db.prepare("SELECT request_count FROM rate_limits WHERE source_hash=?1 AND route=?2 AND window_start=?3")
                .bind(&[source_hash.into(), operation.to_owned().into(), window_start.to_string().into()])?,
        ])
        .await?;
    let count = result[1]
        .results::<RateLimitRow>()?
        .into_iter()
        .next()
        .map(|row| row.request_count)
        .unwrap_or(limit + 1);
    if count > limit {
        return Err(ApiError::rate_limited(
            RATE_WINDOW_SECONDS - (now % RATE_WINDOW_SECONDS),
        ));
    }
    Ok(())
}

async fn cleanup_expired(env: &Env, now: u64) -> Result<()> {
    let cutoff = now.saturating_sub(RETENTION_SECONDS).to_string();
    let rate_cutoff = now.saturating_sub(24 * 60 * 60).to_string();
    let db = env.d1("DB")?;
    db.batch(vec![
        db.prepare("DELETE FROM answers WHERE event_id IN (SELECT id FROM events WHERE created_at IS NOT NULL AND created_at < ?1)")
            .bind(&[cutoff.clone().into()])?,
        db.prepare("DELETE FROM responses WHERE event_id IN (SELECT id FROM events WHERE created_at IS NOT NULL AND created_at < ?1)")
            .bind(&[cutoff.clone().into()])?,
        db.prepare("DELETE FROM candidates WHERE event_id IN (SELECT id FROM events WHERE created_at IS NOT NULL AND created_at < ?1)")
            .bind(&[cutoff.clone().into()])?,
        db.prepare("DELETE FROM events WHERE created_at IS NOT NULL AND created_at < ?1")
            .bind(&[cutoff.into()])?,
        db.prepare("DELETE FROM rate_limits WHERE window_start < ?1")
            .bind(&[rate_cutoff.into()])?,
    ]).await?;
    Ok(())
}

#[allow(unused_must_use)]
#[event(scheduled)]
pub async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) -> Result<()> {
    cleanup_expired(&env, Date::now().as_millis() / 1_000).await
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
            | (
                Method::Delete,
                ["", "api", "events", _, "responses", _, "capability"]
            )
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
    let operation = match (&method, segments.as_slice()) {
        (Method::Post, ["", "api", "events"]) => Some("create"),
        (Method::Post, ["", "api", "events", _, "responses"]) => Some("response"),
        (Method::Get, ["", "api", "events", _])
        | (Method::Get, ["", "api", "events", _, "responses"]) => Some("read"),
        (Method::Delete, ["", "api", "events", _, "responses", _, "capability"]) => None,
        _ => None,
    };
    if let Some(operation) = operation {
        enforce_rate_limit(&request, &env, operation).await?;
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
        (
            Method::Delete,
            [
                "",
                "api",
                "events",
                event_id,
                "responses",
                response_id,
                "capability",
            ],
        ) if identifier_valid(event_id) && identifier_valid(response_id) => {
            api::revoke_response(event_id, response_id, &request, &env).await
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
            let mut response = Response::from_json(&json!({"error": {"code": error.code}}))?
                .with_status(error.status);
            if let Some(retry_after) = error.retry_after {
                response
                    .headers_mut()
                    .set("Retry-After", &retry_after.to_string())?;
            }
            response
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
