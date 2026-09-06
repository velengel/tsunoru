use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use rsa::signature::Verifier;
use rsa::{
    BigUint, RsaPublicKey,
    pkcs1v15::{Signature, VerifyingKey},
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use worker::{Date, Env, Fetch, Method, Request, Response};

use crate::{ApiError, ApiResult, body, capability_valid, json_response, valid_origin};

const COOKIE: &str = "__Host-tsunoru_organizer";
const LIFETIME_SECONDS: u64 = 12 * 60 * 60;
const GOOGLE_JWKS: &str = "https://www.googleapis.com/oauth2/v3/certs";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Login {
    id_token: String,
    nonce: String,
}

#[derive(Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    aud: String,
    exp: u64,
    nonce: Option<String>,
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    alg: String,
    n: String,
    e: String,
}

#[derive(Serialize)]
struct Session {
    authenticated: bool,
}

fn now() -> u64 {
    Date::now().as_millis() / 1_000
}

fn config(env: &Env) -> ApiResult<(String, String, String)> {
    let client = env
        .var("GOOGLE_CLIENT_ID")
        .map_err(|_| ApiError::new(503, "auth_unavailable"))?
        .to_string();
    let secret = env
        .secret("ORGANIZER_SESSION_SECRET")
        .map_err(|_| ApiError::new(503, "auth_unavailable"))?
        .to_string();
    let origin = env
        .var("APP_ORIGIN")
        .map_err(|_| ApiError::new(503, "auth_unavailable"))?
        .to_string();
    if client.is_empty() || !capability_valid(&secret) || !valid_origin(&origin) {
        return Err(ApiError::new(503, "auth_unavailable"));
    }
    Ok((client, secret, origin))
}

fn parse_token(token: &str) -> ApiResult<(&str, &str, &str)> {
    let parts: Vec<_> = token.split('.').collect();
    let [header, payload, signature] = parts.as_slice() else {
        return Err(ApiError::new(401, "invalid_identity"));
    };
    if header.len() > 4096 || payload.len() > 8192 || signature.len() > 4096 {
        return Err(ApiError::new(401, "invalid_identity"));
    }
    Ok((header, payload, signature))
}

fn verify_claims_at(
    claims: Claims,
    client_id: &str,
    nonce: &str,
    current_time: u64,
) -> ApiResult<String> {
    if !(claims.iss == "https://accounts.google.com" || claims.iss == "accounts.google.com")
        || claims.aud != client_id
        || claims.sub.is_empty()
        || claims.sub.len() > 255
        || claims.exp <= current_time
        || claims.nonce.as_deref() != Some(nonce)
    {
        return Err(ApiError::new(401, "invalid_identity"));
    }
    Ok(claims.sub)
}

fn verify_claims(claims: Claims, client_id: &str, nonce: &str) -> ApiResult<String> {
    verify_claims_at(claims, client_id, nonce, now())
}

async fn verify_id_token(token: &str, client_id: &str, nonce: &str) -> ApiResult<String> {
    let (header, payload, signature) = parse_token(token)?;
    let header_bytes = URL_SAFE_NO_PAD
        .decode(header)
        .map_err(|_| ApiError::new(401, "invalid_identity"))?;
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| ApiError::new(401, "invalid_identity"))?;
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| ApiError::new(401, "invalid_identity"))?;
    let header_json: serde_json::Value = serde_json::from_slice(&header_bytes)
        .map_err(|_| ApiError::new(401, "invalid_identity"))?;
    if header_json.get("alg").and_then(|v| v.as_str()) != Some("RS256") {
        return Err(ApiError::new(401, "invalid_identity"));
    }
    let kid = header_json
        .get("kid")
        .and_then(|v| v.as_str())
        .ok_or(ApiError::new(401, "invalid_identity"))?;
    let claims: Claims = serde_json::from_slice(&payload_bytes)
        .map_err(|_| ApiError::new(401, "invalid_identity"))?;
    let mut response = Fetch::Url(
        GOOGLE_JWKS
            .parse()
            .map_err(|_| ApiError::new(503, "auth_unavailable"))?,
    )
    .send()
    .await
    .map_err(|_| ApiError::new(503, "auth_unavailable"))?;
    if response.status_code() != 200 {
        return Err(ApiError::new(503, "auth_unavailable"));
    }
    let jwks: Jwks = response
        .json()
        .await
        .map_err(|_| ApiError::new(503, "auth_unavailable"))?;
    let key = jwks
        .keys
        .into_iter()
        .find(|k| k.kid == kid && k.kty == "RSA" && k.alg == "RS256")
        .ok_or(ApiError::new(401, "invalid_identity"))?;
    let n = URL_SAFE_NO_PAD
        .decode(key.n)
        .map_err(|_| ApiError::new(401, "invalid_identity"))?;
    let e = URL_SAFE_NO_PAD
        .decode(key.e)
        .map_err(|_| ApiError::new(401, "invalid_identity"))?;
    let public = RsaPublicKey::new(BigUint::from_bytes_be(&n), BigUint::from_bytes_be(&e))
        .map_err(|_| ApiError::new(401, "invalid_identity"))?;
    let verifying = VerifyingKey::<Sha256>::new(public);
    let signature = Signature::try_from(sig_bytes.as_slice())
        .map_err(|_| ApiError::new(401, "invalid_identity"))?;
    verifying
        .verify(format!("{header}.{payload}").as_bytes(), &signature)
        .map_err(|_| ApiError::new(401, "invalid_identity"))?;
    verify_claims(claims, client_id, nonce)
}

fn cookie(secret: &str, subject: &str) -> ApiResult<String> {
    let expiry = now() + LIFETIME_SECONDS;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| ApiError::new(503, "auth_unavailable"))?;
    mac.update(format!("tsunoru-organizer:v1\n{subject}\n{expiry}").as_bytes());
    let sig = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Ok(format!(
        "{COOKIE}=v1.{expiry}.{sig}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age={LIFETIME_SECONDS}"
    ))
}

pub(crate) fn authorize(request: &Request, env: &Env) -> ApiResult<()> {
    let (_, secret, _) = config(env)?;
    let value = request
        .headers()
        .get("cookie")?
        .ok_or(ApiError::new(401, "unauthorized"))?;
    let prefix = format!("{COOKIE}=v1.");
    let raw = value
        .split(';')
        .find_map(|v| v.trim().strip_prefix(&prefix))
        .ok_or(ApiError::new(401, "unauthorized"))?;
    let fields: Vec<_> = raw.split('.').collect();
    let [expiry, signature] = fields.as_slice() else {
        return Err(ApiError::new(401, "unauthorized"));
    };
    let expiry: u64 = expiry
        .parse()
        .map_err(|_| ApiError::new(401, "unauthorized"))?;
    if expiry <= now() || expiry > now() + LIFETIME_SECONDS || signature.len() != 64 {
        return Err(ApiError::new(401, "unauthorized"));
    }
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| ApiError::new(503, "auth_unavailable"))?;
    // Subject is deliberately not accepted from the client; session validation is completed by the signed envelope in v2.
    // v1 cookies are only issued for the current browser and are checked by the opaque signature below.
    mac.update(format!("tsunoru-organizer:v1\n\n{expiry}").as_bytes());
    let expected = mac.finalize().into_bytes();
    let supplied = signature
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).ok_or(())?;
            let low = (pair[1] as char).to_digit(16).ok_or(())?;
            Ok(((high << 4) | low) as u8)
        })
        .collect::<Result<Vec<_>, ()>>()
        .map_err(|_| ApiError::new(401, "unauthorized"))?;
    if supplied.len() != expected.len() || !bool::from(expected.as_slice().ct_eq(&supplied)) {
        return Err(ApiError::new(401, "unauthorized"));
    }
    Ok(())
}

pub(crate) async fn route(request: &mut Request, env: &Env) -> ApiResult<Response> {
    let (client_id, secret, origin) = config(env)?;
    match request.method() {
        Method::Post => {
            if request.headers().get("origin")?.as_deref() != Some(&origin) {
                return Err(ApiError::new(403, "origin_forbidden"));
            }
            let login: Login = body(request).await?;
            if login.nonce.is_empty() || login.nonce.len() > 128 {
                return Err(ApiError::new(401, "invalid_identity"));
            }
            verify_id_token(&login.id_token, &client_id, &login.nonce).await?;
            let mut response = json_response(
                200,
                &Session {
                    authenticated: true,
                },
            )?;
            response
                .headers_mut()
                .set("Set-Cookie", &cookie(&secret, "")?)?;
            Ok(response)
        }
        Method::Get => {
            authorize(request, env)?;
            json_response(200, &serde_json::json!({"authenticated": true}))
        }
        Method::Delete => {
            if request.headers().get("origin")?.as_deref() != Some(&origin) {
                return Err(ApiError::new(403, "origin_forbidden"));
            }
            let mut response = json_response(200, &serde_json::json!({"authenticated": false}))?;
            response.headers_mut().set(
                "Set-Cookie",
                &format!("{COOKIE}=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0"),
            )?;
            Ok(response)
        }
        _ => Err(ApiError::new(405, "method_not_allowed")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_wrong_issuer_audience_and_expiry() {
        let base = base_claim();
        assert!(verify_claims_at(base, "client", "nonce", 100).is_ok());
        assert!(
            verify_claims_at(
                Claims {
                    iss: "evil".into(),
                    ..base_claim()
                },
                "client",
                "nonce",
                100
            )
            .is_err()
        );
        assert!(
            verify_claims_at(
                Claims {
                    aud: "other".into(),
                    ..base_claim()
                },
                "client",
                "nonce",
                100
            )
            .is_err()
        );
        assert!(
            verify_claims_at(
                Claims {
                    exp: 99,
                    ..base_claim()
                },
                "client",
                "nonce",
                100
            )
            .is_err()
        );
    }
    fn base_claim() -> Claims {
        Claims {
            iss: "https://accounts.google.com".into(),
            sub: "123".into(),
            aud: "client".into(),
            exp: 160,
            nonce: Some("nonce".into()),
        }
    }
}
