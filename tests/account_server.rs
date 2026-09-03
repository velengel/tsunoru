#![cfg(feature = "server")]

use tsunoru::{
    auth::hash_session_token,
    domain::{AccountLoginInput, AccountRegistrationInput},
    server::{
        AccountLoginError, persist_account_login, persist_account_login_replacing_session,
        persist_account_registration, persist_account_registration_replacing_session,
    },
    storage::{open_in_memory, resolve_account_session},
};

const NOW: i64 = 1_800_000_000;
const PASSWORD: &str = "correct horse battery staple";

fn source_section<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start = source.find(start).expect("source section start");
    let end = source[start..]
        .find(end)
        .map(|offset| start + offset)
        .expect("source section end");
    &source[start..end]
}

fn registration(login_id: &str) -> AccountRegistrationInput {
    AccountRegistrationInput {
        login_id: login_id.to_owned(),
        password: PASSWORD.to_owned(),
        password_confirmation: PASSWORD.to_owned(),
    }
}

#[tokio::test]
async fn registration_hashes_the_password_and_stores_only_the_session_digest() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_session = "a".repeat(64);

    let issued =
        persist_account_registration(&pool, registration("  Reader  "), raw_session.clone(), NOW)
            .await
            .expect("create and log into an account");
    assert_eq!(issued.account.login_id, "reader");
    assert_eq!(issued.session_token, raw_session);

    let stored_password: String = sqlx::query_scalar("SELECT password_hash_phc FROM accounts")
        .fetch_one(&pool)
        .await
        .expect("read password hash");
    let stored_session: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM account_sessions")
        .fetch_one(&pool)
        .await
        .expect("read token digest");
    assert!(stored_password.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
    assert!(!stored_password.contains(PASSWORD));
    assert_ne!(stored_session, issued.session_token.as_bytes());

    let debug = format!("{issued:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains(&issued.session_token));
}

#[tokio::test]
async fn wrong_password_and_unknown_login_have_one_public_failure() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    persist_account_registration(&pool, registration("known"), "b".repeat(64), NOW)
        .await
        .expect("create known account");

    let wrong = persist_account_login(
        &pool,
        AccountLoginInput {
            login_id: "known".to_owned(),
            password: "a deliberately wrong password".to_owned(),
        },
        "c".repeat(64),
        NOW + 1,
    )
    .await
    .expect_err("wrong password must fail");
    let unknown = persist_account_login(
        &pool,
        AccountLoginInput {
            login_id: "unknown".to_owned(),
            password: "a deliberately wrong password".to_owned(),
        },
        "d".repeat(64),
        NOW + 1,
    )
    .await
    .expect_err("unknown account must fail");

    assert!(matches!(wrong, AccountLoginError::InvalidCredentials));
    assert!(matches!(unknown, AccountLoginError::InvalidCredentials));
    assert_eq!(wrong.to_string(), unknown.to_string());
    assert_eq!(
        wrong.to_string(),
        "ログインIDまたはpasswordを確認してください。"
    );
}

#[tokio::test]
async fn every_successful_login_rotates_the_browser_session_token() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    persist_account_registration(&pool, registration("rotate"), "e".repeat(64), NOW)
        .await
        .expect("create account");

    let login = AccountLoginInput {
        login_id: "rotate".to_owned(),
        password: PASSWORD.to_owned(),
    };
    let first = persist_account_login(&pool, login.clone(), "f".repeat(64), NOW + 1)
        .await
        .expect("first login");
    let second = persist_account_login(&pool, login, "1".repeat(64), NOW + 2)
        .await
        .expect("second login");

    assert_ne!(first.session_token, second.session_token);
    let session_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM account_sessions WHERE account_id = (SELECT id FROM accounts WHERE login_id = 'rotate')")
            .fetch_one(&pool)
            .await
            .expect("count active device sessions");
    assert_eq!(
        session_count, 3,
        "registration and two logins are distinct sessions"
    );
}

#[tokio::test]
async fn login_replaces_the_presented_browser_session_in_one_commit() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let old_raw = "2".repeat(64);
    persist_account_registration(&pool, registration("replace-login"), old_raw.clone(), NOW)
        .await
        .expect("create account and old browser session");
    let old_hash = hash_session_token(&old_raw).unwrap();

    let issued = persist_account_login_replacing_session(
        &pool,
        AccountLoginInput {
            login_id: "replace-login".to_owned(),
            password: PASSWORD.to_owned(),
        },
        "3".repeat(64),
        Some(&old_hash),
        NOW + 1,
    )
    .await
    .expect("rotate the same browser session");

    assert!(
        resolve_account_session(&pool, &old_hash, NOW + 2)
            .await
            .unwrap()
            .is_none(),
        "the cookie that was replaced must no longer authorize"
    );
    let new_hash = hash_session_token(&issued.session_token).unwrap();
    assert!(
        resolve_account_session(&pool, &new_hash, NOW + 2)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn session_replacement_failure_rolls_back_login_and_registration() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let old_raw = "4".repeat(64);
    persist_account_registration(&pool, registration("existing"), old_raw.clone(), NOW)
        .await
        .expect("create old browser session");
    let old_hash = hash_session_token(&old_raw).unwrap();

    sqlx::query(
        r#"
        CREATE TRIGGER reject_session_replacement
        BEFORE DELETE ON account_sessions
        BEGIN
            SELECT RAISE(ABORT, 'blocked session deletion');
        END
        "#,
    )
    .execute(&pool)
    .await
    .expect("install deterministic delete failure");

    let login = persist_account_login_replacing_session(
        &pool,
        AccountLoginInput {
            login_id: "existing".to_owned(),
            password: PASSWORD.to_owned(),
        },
        "5".repeat(64),
        Some(&old_hash),
        NOW + 1,
    )
    .await;
    assert!(
        login.is_err(),
        "a failed old-session delete must fail login"
    );

    let registration = persist_account_registration_replacing_session(
        &pool,
        registration("must-not-exist"),
        "6".repeat(64),
        Some(&old_hash),
        NOW + 2,
    )
    .await;
    assert!(
        registration.is_err(),
        "a failed old-session delete must roll back account creation"
    );

    let counts: (i64, i64) = (
        sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COUNT(*) FROM account_sessions")
            .fetch_one(&pool)
            .await
            .unwrap(),
    );
    assert_eq!(counts, (1, 1));
    assert!(
        resolve_account_session(&pool, &old_hash, NOW + 3)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn a_correct_password_cannot_bypass_a_full_login_attempt_window() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let login_id = "preflight-limit";
    persist_account_registration(&pool, registration(login_id), "7".repeat(64), NOW)
        .await
        .expect("create account for login-limit integration");

    for attempt in 0..5 {
        let error = persist_account_login(
            &pool,
            AccountLoginInput {
                login_id: login_id.to_owned(),
                password: "a deliberately wrong password".to_owned(),
            },
            format!("{:064x}", attempt + 8),
            NOW + attempt,
        )
        .await
        .expect_err("the first five wrong attempts should fail generically");
        assert!(matches!(error, AccountLoginError::InvalidCredentials));
    }

    let blocked_correct = persist_account_login(
        &pool,
        AccountLoginInput {
            login_id: login_id.to_owned(),
            password: PASSWORD.to_owned(),
        },
        "f".repeat(64),
        NOW + 5,
    )
    .await
    .expect_err("the limiter must run before password verification");
    assert!(matches!(blocked_correct, AccountLoginError::RateLimited(_)));

    let server = include_str!("../src/server.rs");
    assert!(
        server.contains("registration_rate_limiter()")
            && server.contains(".record_attempt(\"account-registration\", now)"),
        "registration must reserve a bounded Argon2 attempt before hashing"
    );
}

#[test]
fn data_endpoints_do_not_open_a_session_preflight_or_mutate_the_auth_cookie() {
    let server = include_str!("../src/server.rs");

    assert!(
        !server.contains("async fn optional_write_session("),
        "event and response writes must resolve a shaped session only inside their own transaction"
    );
    assert!(
        !server.contains("fn write_session_hash(")
            && !server.contains("fn clear_inactive_write_session("),
        "public writes must not emit a stale-cookie deletion that can race a newer login response"
    );
    assert!(
        server
            .match_indices("session.presented.digest().copied()")
            .count()
            >= 2,
        "event and response writes must pass only the shaped digest to their repository transaction"
    );

    for endpoint in [
        source_section(
            server,
            "pub async fn create_event(",
            "pub async fn persist_created_event(",
        ),
        source_section(
            server,
            "pub async fn submit_availability_response(",
            "pub async fn persist_availability_response(",
        ),
        source_section(
            server,
            "pub async fn get_account_history(",
            "fn add_private_response_headers(",
        ),
    ] {
        assert!(
            !endpoint.contains("set_session_cookie(")
                && !endpoint.contains("clear_session_cookie("),
            "public data and history responses must not race explicit auth cookie changes"
        );
    }
}
