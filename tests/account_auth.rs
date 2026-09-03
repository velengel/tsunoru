#![cfg(feature = "server")]

use tsunoru::auth::{
    LoginRateLimiter, PresentedSession, SessionCookiePolicy, hash_password, hash_session_token,
    issue_session_token, request_origin_is_allowed, verify_password,
};
use tsunoru::server::protect_private_api_response;

const VALID_PASSWORD: &str = "correct horse battery staple";

#[tokio::test]
async fn password_hashes_are_argon2id_phc_with_independent_salts() {
    let first = hash_password(VALID_PASSWORD)
        .await
        .expect("hash a valid password");
    let second = hash_password(VALID_PASSWORD)
        .await
        .expect("hash the same password with another salt");

    for hash in [&first, &second] {
        assert!(
            hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
            "the reviewed algorithm and cost must be explicit: {hash}"
        );
        assert!(verify_password(VALID_PASSWORD, hash).await.unwrap());
        assert!(!verify_password("wrong password value", hash).await.unwrap());
    }
    assert_ne!(
        first, second,
        "each password needs an independent random salt"
    );
}

#[test]
fn session_tokens_are_random_and_only_their_digest_is_storage_shaped() {
    let first = issue_session_token();
    let second = issue_session_token();

    assert_eq!(first.len(), 64);
    assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_ne!(first, second);

    let digest = hash_session_token(&first).expect("a generated token is valid");
    assert_eq!(digest.len(), 32);
    assert_ne!(digest.as_slice(), first.as_bytes());
    assert!(hash_session_token("malformed").is_none());
}

#[test]
fn production_and_loopback_cookies_keep_separate_security_profiles() {
    let production = SessionCookiePolicy::for_origin("https://tsunoru.example")
        .expect("an explicit HTTPS origin is production-safe");
    let local_8081 = SessionCookiePolicy::for_origin("http://127.0.0.1:8081")
        .expect("loopback HTTP is allowed for local development");
    let local_8082 = SessionCookiePolicy::for_origin("http://127.0.0.1:8082")
        .expect("a second local validation server is isolated");

    let production_header = production.set_cookie_header("a".repeat(64).as_str());
    assert!(production_header.starts_with("__Host-tsunoru-session="));
    for attribute in [
        "Secure",
        "HttpOnly",
        "SameSite=Lax",
        "Path=/",
        "Max-Age=2592000",
    ] {
        assert!(
            production_header.contains(attribute),
            "missing {attribute}: {production_header}"
        );
    }
    assert!(!production_header.contains("Domain="));

    assert_ne!(local_8081.name(), local_8082.name());
    assert!(local_8081.name().contains("8081"));
    assert!(
        !local_8081
            .set_cookie_header("b".repeat(64).as_str())
            .contains("Secure")
    );
    assert!(
        SessionCookiePolicy::for_origin("http://192.0.2.5:8080").is_err(),
        "insecure auth must not spread from loopback to the LAN"
    );
    let clear = local_8081.clear_cookie_header();
    assert!(clear.contains("Max-Age=0") && clear.contains("HttpOnly"));
}

#[test]
fn dioxus_development_proxy_uses_the_external_port_for_origin_and_cookie_isolation() {
    let policy = SessionCookiePolicy::for_request_host("127.0.0.1:63152", Some(8081))
        .expect("the trusted Dioxus dev-server port replaces its ephemeral backend port");

    assert_eq!(policy.origin(), "http://127.0.0.1:8081");
    assert_eq!(policy.name(), "tsunoru-session-local-8081");
}

#[test]
fn cookie_parsing_distinguishes_absence_from_malformed_session_material() {
    let policy = SessionCookiePolicy::for_origin("http://127.0.0.1:8081").unwrap();
    let raw = "a".repeat(64);

    assert!(matches!(
        policy.presented_session_from_cookie_header(None),
        PresentedSession::Absent
    ));
    assert!(matches!(
        policy.presented_session_from_cookie_header(Some("theme=dark")),
        PresentedSession::Absent
    ));
    assert!(matches!(
        policy.presented_session_from_cookie_header(Some("tsunoru-session-local-8081=malformed")),
        PresentedSession::Invalid
    ));
    assert!(matches!(
        policy.presented_session_from_cookie_header(Some(&format!(
            "theme=dark; tsunoru-session-local-8081={raw}"
        ))),
        PresentedSession::Digest(_)
    ));
}

#[test]
fn unsafe_api_requests_require_the_exact_target_origin() {
    let expected = "https://tsunoru.example";
    assert!(request_origin_is_allowed(
        "POST",
        "/api/events/create",
        Some(expected),
        None,
        expected,
    ));
    assert!(request_origin_is_allowed(
        "POST",
        "/api/auth/login",
        None,
        Some("https://tsunoru.example/history"),
        expected,
    ));
    for (origin, referer) in [
        (Some("https://attacker.example"), None),
        (None, None),
        (Some("null"), Some("https://tsunoru.example/history")),
    ] {
        assert!(
            !request_origin_is_allowed("POST", "/api/auth/login", origin, referer, expected,),
            "cross-site or unverifiable writes must fail closed"
        );
    }
    assert!(request_origin_is_allowed(
        "GET",
        "/api/account/history",
        None,
        None,
        expected,
    ));
}

#[test]
fn login_throttle_is_bounded_per_normalized_identifier_without_storing_it() {
    let limiter = LoginRateLimiter::new(5, 15 * 60);
    for second in 0..5 {
        assert!(limiter.record_attempt("reader", second).is_ok());
    }
    let retry_after = limiter
        .record_attempt("reader", 5)
        .expect_err("the sixth failure in one window must be delayed");
    assert!(retry_after > 0 && retry_after <= 15 * 60);
    assert!(limiter.record_failure("another-reader", 5).is_ok());

    limiter.record_success("reader");
    assert!(limiter.record_attempt("reader", 6).is_ok());
    assert!(
        !format!("{limiter:?}").contains("reader"),
        "raw login IDs must not remain in diagnostics"
    );
}

#[test]
fn attempt_tracking_has_a_fixed_identifier_budget_and_expires_old_windows() {
    let limiter = LoginRateLimiter::with_capacity(5, 15 * 60, 2);
    assert!(limiter.record_attempt("first", 0).is_ok());
    assert!(limiter.record_attempt("second", 0).is_ok());
    assert!(
        limiter.record_attempt("third", 1).is_err(),
        "cycling identifiers must not grow process memory without a bound"
    );
    assert_eq!(limiter.tracked_identifier_count(), 2);

    assert!(
        limiter.record_attempt("third", 15 * 60 + 1).is_ok(),
        "expired windows should release capacity"
    );
    assert_eq!(limiter.tracked_identifier_count(), 1);
}

#[test]
fn private_api_headers_cover_failures_returned_before_server_function_code() {
    use dioxus::server::axum::{body::Body, http::header, response::Response};

    for path in [
        "/history",
        "/history/events/private-event",
        "/api/auth/login",
        "/api/account/history",
        "/api/account/history/event-detail",
        "/api/answers/submit",
    ] {
        let mut response = Response::builder()
            .status(400)
            .body(Body::from("decode failed"))
            .unwrap();
        protect_private_api_response(path, &mut response);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(
            response.headers()[header::X_CONTENT_TYPE_OPTIONS],
            "nosniff"
        );
    }

    let mut public = Response::builder().status(400).body(Body::empty()).unwrap();
    protect_private_api_response("/api/events/create", &mut public);
    assert!(!public.headers().contains_key(header::CACHE_CONTROL));
}
