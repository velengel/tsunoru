#![cfg(feature = "server")]

use tsunoru::{
    auth::hash_session_token,
    domain::{CandidateInput, EventContinuationCreateInput, NewEventInput},
    storage::{
        EventContinuationStorageError, create_account_with_session,
        create_event_continuation_by_session, create_event_record, create_event_record_for_session,
        find_account_history_by_session, find_event_continuation_plan_by_session, open_file,
        open_in_memory,
    },
};

use std::path::{Path, PathBuf};

const NOW: i64 = 1_800_000_000;

fn event(name: &str) -> NewEventInput {
    NewEventInput {
        name: name.to_owned(),
        organizer_note: None,
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![CandidateInput {
            local_date: "2027-02-01".to_owned(),
            local_time: "19:00".to_owned(),
        }],
    }
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

fn continuation(origin: &str, tail: &str, name: &str) -> EventContinuationCreateInput {
    EventContinuationCreateInput {
        origin_event_public_id: origin.to_owned(),
        expected_tail_event_public_id: tail.to_owned(),
        event: event(name),
    }
}

fn temporary_database_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("tsunoru-{label}-{}.sqlite3", uuid::Uuid::new_v4()))
}

fn remove_temporary_database(path: &Path) {
    for candidate in [
        path.to_path_buf(),
        PathBuf::from(format!("{}-wal", path.display())),
        PathBuf::from(format!("{}-shm", path.display())),
    ] {
        match std::fs::remove_file(&candidate) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!(
                "remove temporary SQLite file {}: {error}",
                candidate.display()
            ),
        }
    }
}

#[test]
fn continuation_plan_keeps_its_series_read_outside_a_write_transaction() {
    let storage = include_str!("../src/storage.rs");
    let start = storage
        .find("pub async fn find_event_continuation_plan_by_session(")
        .expect("continuation plan repository exists");
    let end = storage[start..]
        .find("/// Create one event aggregate")
        .map(|offset| start + offset)
        .expect("continuation create follows the plan reader");
    let plan_reader = &storage[start..end];

    assert!(
        !plan_reader.contains("begin_with(\"BEGIN IMMEDIATE\")"),
        "a private plan read must not hold SQLite's single writer slot while scanning a series"
    );
    assert!(
        plan_reader.contains("pool.begin().await"),
        "owner authorization and latest-tail reads must still share one DEFERRED snapshot"
    );
}

#[tokio::test]
async fn migration_enforces_one_owned_series_and_preserves_events_on_account_delete() {
    let pool = open_in_memory().await.expect("open migrated SQLite");
    let (account_id, session) = account_session(&pool, "series-owner", &"1".repeat(64)).await;
    create_event_record_for_session(
        &pool,
        "series-origin",
        &"a".repeat(64),
        &event("ベストユニゾン #1"),
        Some(&session),
        NOW,
    )
    .await
    .unwrap();

    let series_id = sqlx::query(
        "INSERT INTO event_series (owner_account_id, display_name, created_at) VALUES (?, ?, ?)",
    )
    .bind(account_id)
    .bind("ベストユニゾン")
    .bind(NOW)
    .execute(&pool)
    .await
    .expect("series table exists")
    .last_insert_rowid();
    sqlx::query(
        "INSERT INTO event_series_members (series_id, owner_account_id, event_public_id, position) VALUES (?, ?, ?, 0)",
    )
    .bind(series_id)
    .bind(account_id)
    .bind("series-origin")
    .execute(&pool)
    .await
    .expect("membership composite foreign keys accept the matching owner");

    let duplicate = sqlx::query(
        "INSERT INTO event_series_members (series_id, owner_account_id, event_public_id, position) VALUES (?, ?, ?, 1)",
    )
    .bind(series_id)
    .bind(account_id)
    .bind("series-origin")
    .execute(&pool)
    .await;
    assert!(
        duplicate.is_err(),
        "one event must not join two positions or series"
    );

    sqlx::query("DELETE FROM accounts WHERE id = ?")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    let event_owner: Option<i64> = sqlx::query_scalar(
        "SELECT organizer_account_id FROM events WHERE public_id = 'series-origin'",
    )
    .fetch_one(&pool)
    .await
    .expect("public event remains");
    let series_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_series")
        .fetch_one(&pool)
        .await
        .unwrap();
    let member_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_series_members")
        .fetch_one(&pool)
        .await
        .unwrap();
    let violations: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!((event_owner, series_count, member_count), (None, 0, 0));
    assert!(violations.is_empty());
}

#[tokio::test]
async fn organizer_plan_and_atomic_continuation_keep_edited_names_in_one_group() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (_, session) = account_session(&pool, "continuation-owner", &"2".repeat(64)).await;
    create_event_record_for_session(
        &pool,
        "continuation-origin",
        &"b".repeat(64),
        &event("ベストユニゾン #1"),
        Some(&session),
        NOW,
    )
    .await
    .unwrap();

    let plan =
        find_event_continuation_plan_by_session(&pool, &session, "continuation-origin", NOW + 1)
            .await
            .expect("organizer receives a private plan");
    assert_eq!(plan.series_name, "ベストユニゾン");
    assert_eq!(plan.tail_event_public_id, "continuation-origin");
    assert_eq!(
        plan.suggested_event_name.as_deref(),
        Some("ベストユニゾン #2")
    );

    let created = create_event_continuation_by_session(
        &pool,
        "continuation-next",
        &"c".repeat(64),
        &continuation(
            "continuation-origin",
            "continuation-origin",
            "ベストユニゾン 夏回",
        ),
        &session,
        NOW + 2,
    )
    .await
    .expect("event and membership commit together");
    assert_eq!(created.name, "ベストユニゾン 夏回");

    let history = find_account_history_by_session(&pool, &session, NOW + 3)
        .await
        .expect("read grouped organizer history");
    assert!(history.organized_standalone.is_empty());
    assert_eq!(history.organized_series.len(), 1);
    assert_eq!(history.organized_series[0].series_name, "ベストユニゾン");
    assert_eq!(
        history.organized_series[0]
            .events
            .iter()
            .map(|event| event.name.as_str())
            .collect::<Vec<_>>(),
        vec!["ベストユニゾン 夏回", "ベストユニゾン #1"]
    );

    let counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM events), (SELECT COUNT(*) FROM event_series), (SELECT COUNT(*) FROM event_series_members)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(counts, (2, 1, 2));
}

#[tokio::test]
async fn stale_tail_and_expired_session_roll_back_every_continuation_row() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (_, session) = account_session(&pool, "stale-owner", &"3".repeat(64)).await;
    create_event_record_for_session(
        &pool,
        "stale-origin",
        &"d".repeat(64),
        &event("朗読会 #1"),
        Some(&session),
        NOW,
    )
    .await
    .unwrap();
    create_event_continuation_by_session(
        &pool,
        "stale-next",
        &"e".repeat(64),
        &continuation("stale-origin", "stale-origin", "朗読会 #2"),
        &session,
        NOW + 1,
    )
    .await
    .unwrap();

    assert!(matches!(
        create_event_continuation_by_session(
            &pool,
            "must-not-exist",
            &"f".repeat(64),
            &continuation("stale-origin", "stale-origin", "朗読会 #2"),
            &session,
            NOW + 2,
        )
        .await,
        Err(EventContinuationStorageError::Stale)
    ));
    assert!(matches!(
        create_event_continuation_by_session(
            &pool,
            "expired-must-not-exist",
            &"1".repeat(64),
            &continuation("stale-origin", "stale-next", "朗読会 #3"),
            &session,
            NOW + 31 * 24 * 60 * 60,
        )
        .await,
        Err(EventContinuationStorageError::Unauthenticated)
    ));

    let partial_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM events WHERE public_id IN ('must-not-exist', 'expired-must-not-exist')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(partial_count, 0);
}

#[tokio::test]
async fn participant_stranger_missing_and_anonymous_origins_do_not_become_series_authority() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (_, owner_session) = account_session(&pool, "scope-owner", &"4".repeat(64)).await;
    let (participant_id, participant_session) =
        account_session(&pool, "scope-participant", &"5".repeat(64)).await;
    create_event_record_for_session(
        &pool,
        "owned-origin",
        &"2".repeat(64),
        &event("所有者の会 #1"),
        Some(&owner_session),
        NOW,
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO responses (event_public_id, respondent_name, response_capability_hash, respondent_account_id) VALUES (?, ?, ?, ?)",
    )
    .bind("owned-origin")
    .bind("回答しただけの人")
    .bind("4".repeat(64))
    .bind(participant_id)
    .execute(&pool)
    .await
    .expect("link one participant-only account");
    create_event_record(
        &pool,
        "anonymous-origin",
        &"3".repeat(64),
        &event("匿名の会 #1"),
    )
    .await
    .unwrap();

    for origin in ["owned-origin", "anonymous-origin", "missing-origin"] {
        assert!(matches!(
            find_event_continuation_plan_by_session(&pool, &participant_session, origin, NOW + 1,)
                .await,
            Err(EventContinuationStorageError::NotFound)
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn simultaneous_continuations_commit_one_event_and_report_one_stale_plan() {
    let path = temporary_database_path("concurrent-event-continuation");
    let pool = open_file(&path).await.expect("open file-backed SQLite");
    let (_, session) = account_session(&pool, "concurrent-series-owner", &"6".repeat(64)).await;
    create_event_record_for_session(
        &pool,
        "concurrent-series-origin",
        &"6".repeat(64),
        &event("輪読会 #1"),
        Some(&session),
        NOW,
    )
    .await
    .unwrap();
    let first_input = continuation(
        "concurrent-series-origin",
        "concurrent-series-origin",
        "輪読会 #2 A",
    );
    let second_input = continuation(
        "concurrent-series-origin",
        "concurrent-series-origin",
        "輪読会 #2 B",
    );
    let first_capability_hash = "7".repeat(64);
    let second_capability_hash = "8".repeat(64);

    let (first, second) = tokio::join!(
        create_event_continuation_by_session(
            &pool,
            "concurrent-series-a",
            &first_capability_hash,
            &first_input,
            &session,
            NOW + 1,
        ),
        create_event_continuation_by_session(
            &pool,
            "concurrent-series-b",
            &second_capability_hash,
            &second_input,
            &session,
            NOW + 1,
        ),
    );

    let mut created = 0;
    let mut stale = 0;
    for result in [first, second] {
        match result {
            Ok(_) => created += 1,
            Err(EventContinuationStorageError::Stale) => stale += 1,
            unexpected => panic!("unexpected concurrent continuation result: {unexpected:?}"),
        }
    }
    assert_eq!((created, stale), (1, 1));

    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM event_series),
            (SELECT COUNT(*) FROM event_series_members),
            (SELECT COUNT(*) FROM events),
            (SELECT COUNT(*) FROM events
             WHERE public_id IN ('concurrent-series-a', 'concurrent-series-b'))
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("count the one committed continuation aggregate");
    assert_eq!(counts, (1, 2, 2, 1));

    pool.close().await;
    remove_temporary_database(&path);
}

#[tokio::test]
async fn candidate_constraint_failure_rolls_back_new_series_event_and_memberships() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (_, session) = account_session(&pool, "candidate-rollback-owner", &"7".repeat(64)).await;
    create_event_record_for_session(
        &pool,
        "candidate-rollback-origin",
        &"9".repeat(64),
        &event("作曲会 #1"),
        Some(&session),
        NOW,
    )
    .await
    .unwrap();
    let mut failing = continuation(
        "candidate-rollback-origin",
        "candidate-rollback-origin",
        "作曲会 #2",
    );
    failing
        .event
        .candidates
        .push(failing.event.candidates[0].clone());

    assert!(matches!(
        create_event_continuation_by_session(
            &pool,
            "candidate-rollback-next",
            &"a".repeat(64),
            &failing,
            &session,
            NOW + 1,
        )
        .await,
        Err(EventContinuationStorageError::Database(_))
    ));

    let partials: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM events
             WHERE public_id = 'candidate-rollback-next'),
            (SELECT COUNT(*) FROM candidates
             WHERE event_public_id = 'candidate-rollback-next'),
            (SELECT COUNT(*) FROM event_series),
            (SELECT COUNT(*) FROM event_series_members)
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("inspect the failed aggregate transaction");
    assert_eq!(partials, (0, 0, 0, 0));
}

#[tokio::test]
async fn malformed_series_rows_fail_closed_instead_of_returning_partial_history() {
    let empty_pool = open_in_memory().await.expect("open empty-series SQLite");
    let (empty_account, empty_session) =
        account_session(&empty_pool, "empty-series-owner", &"8".repeat(64)).await;
    sqlx::query(
        "INSERT INTO event_series (owner_account_id, display_name, created_at) VALUES (?, ?, ?)",
    )
    .bind(empty_account)
    .bind("空の活動")
    .bind(NOW)
    .execute(&empty_pool)
    .await
    .unwrap();
    assert!(matches!(
        find_account_history_by_session(&empty_pool, &empty_session, NOW + 1).await,
        Err(tsunoru::storage::AccountHistoryStorageError::DataInvariantViolation)
    ));

    let one_pool = open_in_memory().await.expect("open one-member SQLite");
    let (one_account, one_session) =
        account_session(&one_pool, "one-series-owner", &"9".repeat(64)).await;
    create_event_record_for_session(
        &one_pool,
        "one-series-origin",
        &"b".repeat(64),
        &event("一件だけ #1"),
        Some(&one_session),
        NOW,
    )
    .await
    .unwrap();
    let one_series = sqlx::query(
        "INSERT INTO event_series (owner_account_id, display_name, created_at) VALUES (?, ?, ?)",
    )
    .bind(one_account)
    .bind("一件だけ")
    .bind(NOW)
    .execute(&one_pool)
    .await
    .unwrap()
    .last_insert_rowid();
    sqlx::query(
        "INSERT INTO event_series_members (series_id, owner_account_id, event_public_id, position) VALUES (?, ?, ?, 0)",
    )
    .bind(one_series)
    .bind(one_account)
    .bind("one-series-origin")
    .execute(&one_pool)
    .await
    .unwrap();
    assert!(matches!(
        find_account_history_by_session(&one_pool, &one_session, NOW + 1).await,
        Err(tsunoru::storage::AccountHistoryStorageError::DataInvariantViolation)
    ));

    let missing_pool = open_in_memory().await.expect("open missing-event SQLite");
    let (_, missing_session) =
        account_session(&missing_pool, "missing-series-owner", &"a".repeat(64)).await;
    create_event_record_for_session(
        &missing_pool,
        "missing-series-origin",
        &"c".repeat(64),
        &event("欠損検査 #1"),
        Some(&missing_session),
        NOW,
    )
    .await
    .unwrap();
    create_event_continuation_by_session(
        &missing_pool,
        "missing-series-next",
        &"d".repeat(64),
        &continuation(
            "missing-series-origin",
            "missing-series-origin",
            "欠損検査 #2",
        ),
        &missing_session,
        NOW + 1,
    )
    .await
    .unwrap();
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&missing_pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM events WHERE public_id = 'missing-series-next'")
        .execute(&missing_pool)
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&missing_pool)
        .await
        .unwrap();
    assert!(matches!(
        find_account_history_by_session(&missing_pool, &missing_session, NOW + 2).await,
        Err(tsunoru::storage::AccountHistoryStorageError::DataInvariantViolation)
    ));

    let owner_pool = open_in_memory().await.expect("open owner-mismatch SQLite");
    let (_, owner_session) =
        account_session(&owner_pool, "mismatch-series-owner", &"b".repeat(64)).await;
    let (other_account, _) =
        account_session(&owner_pool, "mismatch-series-other", &"c".repeat(64)).await;
    create_event_record_for_session(
        &owner_pool,
        "mismatch-series-origin",
        &"e".repeat(64),
        &event("所有者検査 #1"),
        Some(&owner_session),
        NOW,
    )
    .await
    .unwrap();
    create_event_continuation_by_session(
        &owner_pool,
        "mismatch-series-next",
        &"f".repeat(64),
        &continuation(
            "mismatch-series-origin",
            "mismatch-series-origin",
            "所有者検査 #2",
        ),
        &owner_session,
        NOW + 1,
    )
    .await
    .unwrap();
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&owner_pool)
        .await
        .unwrap();
    sqlx::query("UPDATE event_series_members SET owner_account_id = ? WHERE event_public_id = ?")
        .bind(other_account)
        .bind("mismatch-series-next")
        .execute(&owner_pool)
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&owner_pool)
        .await
        .unwrap();
    assert!(matches!(
        find_account_history_by_session(&owner_pool, &owner_session, NOW + 2).await,
        Err(tsunoru::storage::AccountHistoryStorageError::DataInvariantViolation)
    ));
}
