use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use worker::{Date, Env, Method, Request, Response};

use crate::{ApiError, ApiResult, body, capability_valid, json_response, valid_origin};

const COOKIE: &str = "__Host-tsunoru_staging";
const LIFETIME_SECONDS: u64 = 12 * 60 * 60;

struct Configuration {
    token: String,
    origin: String,
}

impl Configuration {
    fn read(env: &Env) -> ApiResult<Self> {
        let unavailable = ApiError::new(503, "staging_unavailable");
        let token = env
            .secret("STAGING_API_TOKEN")
            .map_err(|_| unavailable)?
            .to_string();
        let origin = env.var("APP_ORIGIN").map_err(|_| unavailable)?.to_string();
        if !capability_valid(&token) || !valid_origin(&origin) {
            return Err(unavailable);
        }
        Ok(Self { token, origin })
    }

    fn matches_code(&self, supplied: &str) -> bool {
        let expected: [u8; 32] = Sha256::digest(self.token.as_bytes()).into();
        let actual: [u8; 32] = Sha256::digest(supplied.as_bytes()).into();
        capability_valid(supplied) && bool::from(expected.ct_eq(&actual))
    }

    fn check_origin(&self, request: &Request, required: bool) -> ApiResult<()> {
        let sent = request.headers().get("origin")?;
        if sent.as_ref().is_some_and(|origin| origin != &self.origin)
            || (required && sent.is_none())
        {
            return Err(ApiError::new(403, "origin_forbidden"));
        }
        Ok(())
    }

    fn mac(&self, expires: u64) -> ApiResult<Hmac<Sha256>> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.token.as_bytes())
            .map_err(|_| ApiError::new(503, "staging_unavailable"))?;
        mac.update(format!("tsunoru-staging-session:v1\n{}\n{expires}", self.origin).as_bytes());
        Ok(mac)
    }

    fn issue_cookie(&self) -> ApiResult<String> {
        let expires = now_seconds() + LIFETIME_SECONDS;
        let signature = self.mac(expires)?.finalize().into_bytes();
        let signature: String = signature.iter().map(|byte| format!("{byte:02x}")).collect();
        Ok(format!(
            "{COOKIE}=v1.{expires}.{signature}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age={LIFETIME_SECONDS}"
        ))
    }

    fn valid_cookie(&self, request: &Request) -> ApiResult<bool> {
        let Some(header) = request.headers().get("cookie")? else {
            return Ok(false);
        };
        let mut values = header.split(';').filter_map(|item| {
            let (name, value) = item.trim().split_once('=')?;
            (name == COOKIE).then_some(value)
        });
        let Some(value) = values.next() else {
            return Ok(false);
        };
        // Reject ambiguous duplicate cookies instead of trusting parser order.
        if values.next().is_some() {
            return Ok(false);
        }
        let fields: Vec<_> = value.split('.').collect();
        let ["v1", expiry, signature] = fields.as_slice() else {
            return Ok(false);
        };
        let Ok(expires) = expiry.parse::<u64>() else {
            return Ok(false);
        };
        let now = now_seconds();
        if expires <= now || expires > now + LIFETIME_SECONDS || !capability_valid(signature) {
            return Ok(false);
        }
        let mut bytes = [0u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let Ok(value) = u8::from_str_radix(&signature[index * 2..index * 2 + 2], 16) else {
                return Ok(false);
            };
            *byte = value;
        }
        Ok(self.mac(expires)?.verify_slice(&bytes).is_ok())
    }

    fn authorize(&self, request: &Request) -> ApiResult<()> {
        let using_cookie = if let Some(header) = request.headers().get("authorization")? {
            if !self.matches_code(header.strip_prefix("Bearer ").unwrap_or_default()) {
                return Err(ApiError::new(401, "unauthorized"));
            }
            false
        } else {
            if !self.valid_cookie(request)? {
                return Err(ApiError::new(401, "unauthorized"));
            }
            true
        };
        let mutation = !matches!(
            request.method(),
            Method::Get | Method::Head | Method::Options
        );
        self.check_origin(request, using_cookie && mutation)
    }
}

fn now_seconds() -> u64 {
    Date::now().as_millis() / 1_000
}

pub(crate) fn authorize(request: &Request, env: &Env) -> ApiResult<()> {
    Configuration::read(env)?.authorize(request)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Login {
    access_code: String,
}

pub(crate) async fn route(request: &mut Request, env: &Env) -> ApiResult<Response> {
    let configuration = Configuration::read(env)?;
    match request.method() {
        Method::Post => {
            configuration.check_origin(request, true)?;
            let login: Login = body(request).await?;
            if !configuration.matches_code(&login.access_code) {
                return Err(ApiError::new(401, "unauthorized"));
            }
            let mut response = json_response(200, &serde_json::json!({"authenticated": true}))?;
            response
                .headers_mut()
                .set("Set-Cookie", &configuration.issue_cookie()?)?;
            Ok(response)
        }
        Method::Get => {
            configuration.authorize(request)?;
            json_response(200, &serde_json::json!({"authenticated": true}))
        }
        Method::Delete => {
            configuration.check_origin(request, true)?;
            let mut response = json_response(200, &serde_json::json!({"authenticated": false}))?;
            response.headers_mut().set(
                "Set-Cookie",
                &format!("{COOKIE}=; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=0"),
            )?;
            Ok(response)
        }
        _ => Err(ApiError::new(405, "method_not_allowed")),
    }
}
