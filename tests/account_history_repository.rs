#![cfg(feature = "server")]

use sha2::{Digest, Sha256};
use tsunoru::{
    auth::hash_session_token,
    domain::{
        Availability, CandidateAvailabilityInput, CandidateInput, NewEventInput,
        PreparedAvailabilityResponse,
    },
    storage::{
        AccountHistoryStorageError, ResponseWriteOutcome, SessionWriteStatus,
        create_account_with_session, create_event_record, create_event_record_for_session,
        delete_account_session, find_account_history_by_session, open_in_memory,
        record_availability_response_for_session, resolve_account_session,
    },
};

const NOW: i64 = 1_800_000_000;

fn event_input(name: &str) -> NewEventInput {
    NewEventInput {
        name: name.to_owned(),
        organizer_note: Some("一覧には出さない主催者のひとこと".to_owned()),
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![CandidateInput {
            local_date: "2027-01-15".to_owned(),
            local_time: "19:00".to_owned(),
        }],
    }
}

fn response(candidate_id: i64, respondent_name: &str) -> PreparedAvailabilityResponse {
    PreparedAvailabilityResponse {
        respondent_name: respondent_name.to_owned(),
        availabilities: vec![CandidateAvailabilityInput {
            candidate_id,
            availability: Availability::Available,
        }],
    }
}

fn response_capability_hash(label: &str) -> String {
    format!("{:x}", Sha256::digest(label.as_bytes()))
}

async fn account_session(
    pool: &sqlx::SqlitePool,
    login_id: &str,
    raw_token: &str,
) -> (i64, [u8; 32]) {
    let token_hash = hash_session_token(raw_token).expect("test session token shape");
    let account = create_account_with_session(
        pool,
        login_id,
        "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA",
        &token_hash,
        NOW,
    )
    .await
    .expect("create account fixture");
    (account.id, token_hash)
}

#[tokio::test]
async fn account_migration_keeps_existing_aggregates_nullable_and_foreign_keys_valid() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("open pre-account SQLite");
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .unwrap();
    for migration in [
        include_str!("../migrations/0001_create_events.sql"),
        include_str!("../migrations/0002_create_availability_responses.sql"),
        include_str!("../migrations/0003_add_response_comment.sql"),
        include_str!("../migrations/0004_index_response_availabilities_by_event.sql"),
        include_str!("../migrations/0005_create_event_decisions.sql"),
    ] {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        r#"
        INSERT INTO events (
            public_id, name, organizer_note, time_zone, organizer_capability_hash
        ) VALUES ('pre-account-event', '以前の匿名イベント', NULL, 'Asia/Tokyo',
                  'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa');
        INSERT INTO candidates (event_public_id, position, local_date, local_time)
        VALUES ('pre-account-event', 0, '2027-01-15', '19:00');
        INSERT INTO responses (event_public_id, respondent_name, response_capability_hash)
        VALUES ('pre-account-event', '匿名回答者',
                'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb');
        INSERT INTO response_availabilities (
            response_id, candidate_id, event_public_id, availability
        ) VALUES (1, 1, 'pre-account-event', 'available');
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed rows before account migration");

    sqlx::raw_sql(include_str!(
        "../migrations/0006_create_accounts_and_history_links.sql"
    ))
    .execute(&pool)
    .await
    .expect("apply account migration to existing aggregates");

    let event_account: Option<i64> =
        sqlx::query_scalar("SELECT organizer_account_id FROM events WHERE public_id = ?")
            .bind("pre-account-event")
            .fetch_one(&pool)
            .await
            .expect("read nullable event owner");
    let response_account: Option<i64> =
        sqlx::query_scalar("SELECT respondent_account_id FROM responses WHERE event_public_id = ?")
            .bind("pre-account-event")
            .fetch_one(&pool)
            .await
            .expect("read nullable response owner");
    let violations: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_all(&pool)
        .await
        .expect("check migrated foreign keys");

    assert_eq!((event_account, response_account), (None, None));
    assert!(
        violations.is_empty(),
        "migration must retain referential integrity"
    );
}

#[tokio::test]
async fn session_storage_keeps_only_a_digest_and_enforces_expiry_and_logout() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw = "1".repeat(64);
    let (account_id, token_hash) = account_session(&pool, "reader", &raw).await;

    let stored_hash: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM account_sessions")
        .fetch_one(&pool)
        .await
        .expect("read stored digest");
    assert_eq!(stored_hash, token_hash);
    assert_ne!(stored_hash, raw.as_bytes());

    let active = resolve_account_session(&pool, &token_hash, NOW + 60)
        .await
        .expect("resolve session")
        .expect("session should be active");
    assert_eq!(
        (active.id, active.login_id.as_str()),
        (account_id, "reader")
    );

    assert!(
        resolve_account_session(&pool, &token_hash, NOW + 30 * 24 * 60 * 60 + 1)
            .await
            .expect("expired session is a normal miss")
            .is_none()
    );
    let expired_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM account_sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(expired_rows, 0, "expired sessions must be removed");

    let (_, active_hash) = account_session(&pool, "active-logout", &"7".repeat(64)).await;
    resolve_account_session(&pool, &active_hash, NOW + 60 * 60 + 1)
        .await
        .expect("touch an active session")
        .expect("the touched session remains active");
    let touched: i64 =
        sqlx::query_scalar("SELECT last_seen_at FROM account_sessions WHERE token_hash = ?")
            .bind(active_hash.as_slice())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(touched, NOW + 60 * 60 + 1);

    delete_account_session(&pool, &active_hash)
        .await
        .expect("logout invalidates the server row");
    assert!(
        resolve_account_session(&pool, &active_hash, NOW + 60 * 60 + 2)
            .await
            .expect("revoked session is a normal miss")
            .is_none()
    );
}

#[tokio::test]
async fn expired_history_session_cleanup_is_committed_on_the_unauthenticated_path() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (_, session_hash) = account_session(&pool, "expired-history", &"8".repeat(64)).await;

    assert!(matches!(
        find_account_history_by_session(&pool, &session_hash, NOW + 30 * 24 * 60 * 60 + 1).await,
        Err(AccountHistoryStorageError::Unauthenticated)
    ));
    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM account_sessions WHERE token_hash = ?")
            .bind(session_hash.as_slice())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0, "history expiry cleanup must not roll back");
}

#[tokio::test]
async fn event_creation_and_account_link_commit_together_without_changing_anonymous_creation() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (account_id, session_hash) = account_session(&pool, "organizer", &"2".repeat(64)).await;

    let linked_write = create_event_record_for_session(
        &pool,
        "account-event",
        &"b".repeat(64),
        &event_input("ログイン中の会"),
        Some(&session_hash),
        NOW + 1,
    )
    .await
    .expect("create and link an account event");
    assert_eq!(linked_write.session_status, SessionWriteStatus::Active);
    let anonymous_write = create_event_record_for_session(
        &pool,
        "anonymous-event",
        &"c".repeat(64),
        &event_input("匿名の会"),
        None,
        NOW + 2,
    )
    .await
    .expect("anonymous creation remains valid");
    assert_eq!(
        anonymous_write.session_status,
        SessionWriteStatus::NotPresented
    );

    let linked: Option<i64> =
        sqlx::query_scalar("SELECT organizer_account_id FROM events WHERE public_id = ?")
            .bind("account-event")
            .fetch_one(&pool)
            .await
            .unwrap();
    let anonymous: Option<i64> =
        sqlx::query_scalar("SELECT organizer_account_id FROM events WHERE public_id = ?")
            .bind("anonymous-event")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(linked, Some(account_id));
    assert_eq!(anonymous, None);
}

#[tokio::test]
async fn expired_session_status_is_returned_from_the_event_and_response_write_transactions() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (_, session_hash) = account_session(&pool, "expired-write", &"9".repeat(64)).await;
    let expired_at = NOW + 30 * 24 * 60 * 60 + 1;

    let event_write = create_event_record_for_session(
        &pool,
        "expired-session-event",
        &"7".repeat(64),
        &event_input("期限切れでも作れる匿名の会"),
        Some(&session_hash),
        expired_at,
    )
    .await
    .expect("an inactive account session must not block anonymous event creation");
    assert_eq!(event_write.session_status, SessionWriteStatus::Inactive);
    let event = event_write.value;

    let response_write = record_availability_response_for_session(
        &pool,
        &event.public_id,
        &response_capability_hash("expired-session-response"),
        &response(event.candidates[0].id, "匿名の回答者"),
        Some(&session_hash),
        expired_at + 1,
    )
    .await
    .expect("an inactive account session must not block an anonymous response");
    assert_eq!(response_write.session_status, SessionWriteStatus::Inactive);
    assert_eq!(response_write.value, ResponseWriteOutcome::Created);

    let links: (Option<i64>, Option<i64>, i64) = (
        sqlx::query_scalar(
            "SELECT organizer_account_id FROM events WHERE public_id = 'expired-session-event'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        sqlx::query_scalar(
            "SELECT respondent_account_id FROM responses WHERE event_public_id = 'expired-session-event'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        sqlx::query_scalar("SELECT COUNT(*) FROM account_sessions WHERE token_hash = ?")
            .bind(session_hash.as_slice())
            .fetch_one(&pool)
            .await
            .unwrap(),
    );
    assert_eq!(links, (None, None, 0));
}

#[tokio::test]
async fn committed_anonymous_response_is_not_claimed_by_a_logged_in_retry() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let event = create_event_record(
        &pool,
        "retry-event",
        &"d".repeat(64),
        &event_input("再試行の会"),
    )
    .await
    .unwrap();
    let payload = response(event.candidates[0].id, "ミナ");
    let capability = response_capability_hash("same-response");

    assert_eq!(
        record_availability_response_for_session(
            &pool,
            &event.public_id,
            &capability,
            &payload,
            None,
            NOW,
        )
        .await
        .unwrap()
        .value,
        ResponseWriteOutcome::Created
    );

    let (_, session_hash) = account_session(&pool, "late-login", &"3".repeat(64)).await;
    assert_eq!(
        record_availability_response_for_session(
            &pool,
            &event.public_id,
            &capability,
            &payload,
            Some(&session_hash),
            NOW + 1,
        )
        .await
        .unwrap()
        .value,
        ResponseWriteOutcome::AlreadyRecorded
    );

    let linked: Option<i64> =
        sqlx::query_scalar("SELECT respondent_account_id FROM responses WHERE event_public_id = ?")
            .bind(&event.public_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        linked, None,
        "a retry must not retroactively claim anonymous work"
    );
}

#[tokio::test]
async fn history_is_account_scoped_deduplicated_and_projects_only_list_fields() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (_, first_session) = account_session(&pool, "first", &"4".repeat(64)).await;
    let (_, second_session) = account_session(&pool, "second", &"5".repeat(64)).await;

    let first_event = create_event_record_for_session(
        &pool,
        "first-event",
        &"e".repeat(64),
        &event_input("最初の主催イベント"),
        Some(&first_session),
        NOW,
    )
    .await
    .unwrap()
    .value;
    let other_event = create_event_record_for_session(
        &pool,
        "second-event",
        &"f".repeat(64),
        &event_input("別accountのイベント"),
        Some(&second_session),
        NOW,
    )
    .await
    .unwrap()
    .value;

    for (index, candidate_id) in [first_event.candidates[0].id, other_event.candidates[0].id]
        .into_iter()
        .enumerate()
    {
        record_availability_response_for_session(
            &pool,
            if index == 0 {
                &first_event.public_id
            } else {
                &other_event.public_id
            },
            &response_capability_hash(&format!("first-response-{index}")),
            &response(candidate_id, "一覧に出さない名前"),
            Some(&first_session),
            NOW + index as i64 + 1,
        )
        .await
        .unwrap();
    }
    record_availability_response_for_session(
        &pool,
        &first_event.public_id,
        &response_capability_hash("first-response-again"),
        &response(first_event.candidates[0].id, "同じaccountの別回答"),
        Some(&first_session),
        NOW + 3,
    )
    .await
    .unwrap();

    let first = find_account_history_by_session(&pool, &first_session, NOW + 10)
        .await
        .expect("read first account history");
    assert_eq!(first.login_id, "first");
    assert_eq!(first.organized_standalone.len(), 1);
    assert_eq!(first.organized_standalone[0].public_id, "first-event");
    assert_eq!(first.organized_standalone[0].response_count, 2);
    assert!(first.organized_series.is_empty());
    assert_eq!(
        first
            .participated
            .iter()
            .map(|item| item.public_id.as_str())
            .collect::<Vec<_>>(),
        vec!["first-event", "second-event"]
    );

    let second = find_account_history_by_session(&pool, &second_session, NOW + 10)
        .await
        .expect("read second account history");
    assert_eq!(second.organized_standalone.len(), 1);
    assert!(second.organized_series.is_empty());
    assert!(second.participated.is_empty());

    let missing = [9_u8; 32];
    assert!(matches!(
        find_account_history_by_session(&pool, &missing, NOW + 10).await,
        Err(AccountHistoryStorageError::Unauthenticated)
    ));
}

#[tokio::test]
async fn deleting_an_account_removes_sessions_but_preserves_public_aggregates() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (account_id, session_hash) = account_session(&pool, "departing", &"6".repeat(64)).await;
    let event = create_event_record_for_session(
        &pool,
        "preserved-event",
        &"1".repeat(64),
        &event_input("残るイベント"),
        Some(&session_hash),
        NOW,
    )
    .await
    .unwrap()
    .value;
    record_availability_response_for_session(
        &pool,
        &event.public_id,
        &response_capability_hash("preserved-response"),
        &response(event.candidates[0].id, "残る回答者"),
        Some(&session_hash),
        NOW + 1,
    )
    .await
    .unwrap();

    sqlx::query("DELETE FROM accounts WHERE id = ?")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("exercise future account deletion semantics");

    let counts: (i64, i64, i64) = (
        sqlx::query_scalar("SELECT COUNT(*) FROM account_sessions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COUNT(*) FROM events WHERE public_id = 'preserved-event'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM responses WHERE event_public_id = 'preserved-event'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
    );
    assert_eq!(counts, (0, 1, 1));

    let links: (Option<i64>, Option<i64>) = (
        sqlx::query_scalar(
            "SELECT organizer_account_id FROM events WHERE public_id = 'preserved-event'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        sqlx::query_scalar(
            "SELECT respondent_account_id FROM responses WHERE event_public_id = 'preserved-event'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
    );
    assert_eq!(links, (None, None));
}
