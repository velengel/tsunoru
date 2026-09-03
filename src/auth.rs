//! Password, session-token, cookie, origin, and login-throttle boundaries.

use anyhow::{Context, anyhow};
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use dioxus::server::axum::http::Uri;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fmt,
    net::IpAddr,
    sync::{Arc, Mutex, OnceLock},
};
use tokio::sync::{OnceCell, Semaphore};
use uuid::Uuid;

const ARGON2_MEMORY_KIB: u32 = 19_456;
const ARGON2_ITERATIONS: u32 = 2;
const ARGON2_PARALLELISM: u32 = 1;
const ARGON2_OUTPUT_BYTES: usize = 32;
const PASSWORD_WORKERS: usize = 4;
const SESSION_TOKEN_HEX_LENGTH: usize = 64;
const SESSION_MAX_AGE_SECONDS: i64 = 30 * 24 * 60 * 60;
const DEFAULT_MAXIMUM_TRACKED_IDENTIFIERS: usize = 4_096;

static PASSWORD_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();
static DUMMY_PASSWORD_HASH: OnceCell<String> = OnceCell::const_new();

fn password_semaphore() -> Arc<Semaphore> {
    PASSWORD_SEMAPHORE
        .get_or_init(|| Arc::new(Semaphore::new(PASSWORD_WORKERS)))
        .clone()
}

fn argon2_context() -> anyhow::Result<Argon2<'static>> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(ARGON2_OUTPUT_BYTES),
    )
    .map_err(|error| anyhow!("invalid Argon2 parameters: {error}"))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// Hash one bounded password without occupying the async executor.
pub async fn hash_password(password: &str) -> anyhow::Result<String> {
    let permit = password_semaphore()
        .acquire_owned()
        .await
        .context("password worker semaphore closed")?;
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        argon2_context()?
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|error| anyhow!("password hashing failed: {error}"))
    })
    .await
    .context("password hashing task failed")?
}

/// Verify one password against a PHC string on a bounded blocking worker.
pub async fn verify_password(password: &str, password_hash_phc: &str) -> anyhow::Result<bool> {
    let permit = password_semaphore()
        .acquire_owned()
        .await
        .context("password worker semaphore closed")?;
    let password = password.to_owned();
    let password_hash_phc = password_hash_phc.to_owned();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let parsed = PasswordHash::new(&password_hash_phc)
            .map_err(|error| anyhow!("stored password hash is malformed: {error}"))?;
        Ok(argon2_context()?
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    })
    .await
    .context("password verification task failed")?
}

/// Warm and return the fixed dummy record used to equalize unknown-account verification.
pub async fn dummy_password_hash() -> anyhow::Result<&'static str> {
    DUMMY_PASSWORD_HASH
        .get_or_try_init(|| async { hash_password("TSUNORU internal dummy password record").await })
        .await
        .map(String::as_str)
}

/// Generate a lowercase 64-hex-character bearer token from two UUIDv4 values.
pub fn issue_session_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

/// Validate and hash a raw session bearer for SQLite storage or lookup.
pub fn hash_session_token(raw_token: &str) -> Option<[u8; 32]> {
    if raw_token.len() != SESSION_TOKEN_HEX_LENGTH
        || !raw_token
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    Some(Sha256::digest(raw_token.as_bytes()).into())
}

/// Cookie attributes derived only from an explicit HTTPS or loopback HTTP origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionCookiePolicy {
    origin: String,
    name: String,
    secure: bool,
}

/// A host-specific account cookie, preserving malformed material for explicit expiry.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PresentedSession {
    Absent,
    Invalid,
    Digest([u8; 32]),
}

impl PresentedSession {
    pub fn digest(&self) -> Option<&[u8; 32]> {
        match self {
            Self::Digest(digest) => Some(digest),
            Self::Absent | Self::Invalid => None,
        }
    }
}

impl SessionCookiePolicy {
    pub fn for_origin(origin: &str) -> anyhow::Result<Self> {
        let uri = origin
            .parse::<Uri>()
            .map_err(|_| anyhow!("public origin must be an absolute HTTP URI"))?;
        let scheme = uri
            .scheme_str()
            .ok_or_else(|| anyhow!("public origin is missing its scheme"))?;
        let authority = uri
            .authority()
            .ok_or_else(|| anyhow!("public origin is missing its authority"))?;
        if uri.query().is_some() || !matches!(uri.path(), "" | "/") {
            return Err(anyhow!("public origin must not contain a path or query"));
        }

        let origin = format!("{scheme}://{authority}");
        if scheme == "https" {
            return Ok(Self {
                origin,
                name: "__Host-tsunoru-session".to_owned(),
                secure: true,
            });
        }
        if scheme != "http" || !is_loopback_host(authority.host()) {
            return Err(anyhow!(
                "insecure account cookies are limited to loopback development"
            ));
        }

        let port = authority.port_u16().unwrap_or(80);
        Ok(Self {
            origin,
            name: format!("tsunoru-session-local-{port}"),
            secure: false,
        })
    }

    /// Resolve a loopback request, replacing Dioxus' ephemeral backend port when supplied.
    pub fn for_request_host(
        host_header: &str,
        devserver_port: Option<u16>,
    ) -> anyhow::Result<Self> {
        let backend = format!("http://{host_header}")
            .parse::<Uri>()
            .map_err(|_| anyhow!("request Host is not a valid authority"))?;
        let authority = backend
            .authority()
            .ok_or_else(|| anyhow!("request Host is missing its authority"))?;
        if !is_loopback_host(authority.host()) {
            return Err(anyhow!("unconfigured public origin is not loopback"));
        }
        let Some(port) = devserver_port else {
            return Self::for_origin(&format!("http://{authority}"));
        };
        let host = authority.host();
        let host = if host.contains(':') {
            format!("[{host}]")
        } else {
            host.to_owned()
        };
        Self::for_origin(&format!("http://{host}:{port}"))
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_cookie_header(&self, raw_token: &str) -> String {
        debug_assert!(hash_session_token(raw_token).is_some());
        let secure = if self.secure { "; Secure" } else { "" };
        format!(
            "{}={raw_token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_MAX_AGE_SECONDS}{secure}",
            self.name
        )
    }

    pub fn clear_cookie_header(&self) -> String {
        let secure = if self.secure { "; Secure" } else { "" };
        format!(
            "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure}",
            self.name
        )
    }

    pub fn presented_session_from_cookie_header(
        &self,
        cookie_header: Option<&str>,
    ) -> PresentedSession {
        let Some(cookie_header) = cookie_header else {
            return PresentedSession::Absent;
        };
        for pair in cookie_header.split(';') {
            let Some((name, value)) = pair.trim().split_once('=') else {
                continue;
            };
            if name == self.name {
                return hash_session_token(value)
                    .map(PresentedSession::Digest)
                    .unwrap_or(PresentedSession::Invalid);
            }
        }
        PresentedSession::Absent
    }
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

/// Resolve the trusted public origin from explicit configuration or a loopback Host header.
pub fn public_origin_for_host(host_header: &str) -> anyhow::Result<SessionCookiePolicy> {
    match std::env::var("TSUNORU_PUBLIC_ORIGIN") {
        Ok(origin) if !origin.trim().is_empty() => SessionCookiePolicy::for_origin(origin.trim()),
        _ => {
            let devserver_port = std::env::var("DIOXUS_DEVSERVER_PORT")
                .ok()
                .and_then(|port| port.parse::<u16>().ok());
            SessionCookiePolicy::for_request_host(host_header, devserver_port)
        }
    }
}

/// Accept same-origin browser writes and reject unverifiable unsafe API requests.
pub fn request_origin_is_allowed(
    method: &str,
    path: &str,
    origin: Option<&str>,
    referer: Option<&str>,
    expected_origin: &str,
) -> bool {
    let unsafe_api = matches!(method, "POST" | "PUT" | "PATCH" | "DELETE")
        && (path == "/api" || path.starts_with("/api/"));
    if !unsafe_api {
        return true;
    }
    match origin {
        Some(origin) => origin == expected_origin,
        None => referer
            .and_then(uri_origin)
            .is_some_and(|referer_origin| referer_origin == expected_origin),
    }
}

fn uri_origin(value: &str) -> Option<String> {
    let uri = value.parse::<Uri>().ok()?;
    Some(format!("{}://{}", uri.scheme_str()?, uri.authority()?))
}

#[derive(Clone, Copy, Debug)]
struct AttemptWindow {
    started_at: i64,
    attempts: u32,
}

/// Single-process login failure limiter keyed by a digest of the normalized ID.
pub struct LoginRateLimiter {
    maximum_attempts: u32,
    window_seconds: i64,
    maximum_identifiers: usize,
    attempts: Mutex<HashMap<[u8; 32], AttemptWindow>>,
}

impl LoginRateLimiter {
    pub fn new(maximum_attempts: u32, window_seconds: i64) -> Self {
        Self::with_capacity(
            maximum_attempts,
            window_seconds,
            DEFAULT_MAXIMUM_TRACKED_IDENTIFIERS,
        )
    }

    pub fn with_capacity(
        maximum_attempts: u32,
        window_seconds: i64,
        maximum_identifiers: usize,
    ) -> Self {
        assert!(maximum_attempts > 0);
        assert!(window_seconds > 0);
        assert!(maximum_identifiers > 0);
        Self {
            maximum_attempts,
            window_seconds,
            maximum_identifiers,
            attempts: Mutex::new(HashMap::new()),
        }
    }

    /// Reserve one attempt before password work, returning seconds until another is allowed.
    pub fn record_attempt(&self, login_id: &str, now: i64) -> Result<(), i64> {
        let key: [u8; 32] = Sha256::digest(login_id.trim().to_ascii_lowercase().as_bytes()).into();
        let mut attempts = self
            .attempts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        attempts.retain(|_, window| now.saturating_sub(window.started_at) < self.window_seconds);
        if !attempts.contains_key(&key) && attempts.len() >= self.maximum_identifiers {
            return Err(self.window_seconds);
        }
        let window = attempts.entry(key).or_insert(AttemptWindow {
            started_at: now,
            attempts: 0,
        });
        if window.attempts >= self.maximum_attempts {
            let elapsed = now.saturating_sub(window.started_at);
            return Err(self.window_seconds.saturating_sub(elapsed).max(1));
        }
        window.attempts += 1;
        Ok(())
    }

    /// Compatibility name for callers that only reserve attempts after a failed result.
    pub fn record_failure(&self, login_id: &str, now: i64) -> Result<(), i64> {
        self.record_attempt(login_id, now)
    }

    pub fn tracked_identifier_count(&self) -> usize {
        self.attempts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    pub fn record_success(&self, login_id: &str) {
        let key: [u8; 32] = Sha256::digest(login_id.trim().to_ascii_lowercase().as_bytes()).into();
        self.attempts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&key);
    }
}

impl fmt::Debug for LoginRateLimiter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entry_count = self
            .attempts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len();
        formatter
            .debug_struct("LoginRateLimiter")
            .field("maximum_attempts", &self.maximum_attempts)
            .field("window_seconds", &self.window_seconds)
            .field("maximum_identifiers", &self.maximum_identifiers)
            .field("entry_count", &entry_count)
            .finish()
    }
}
