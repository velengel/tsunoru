#![cfg(feature = "server")]

use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Barrier},
    time::{SystemTime, UNIX_EPOCH},
};
use tsunoru::{
    domain::{
        Availability, CandidateAvailabilityInput, CandidateInput, NewAvailabilityResponseInput,
        NewEventInput, PreparedAvailabilityResponse,
    },
    server::{AvailabilityResponseSubmissionError, persist_availability_response},
    storage::{
        ResponseStorageError, ResponseWriteOutcome, create_event_record, open_file, open_in_memory,
        record_availability_response,
    },
};

fn event_input() -> NewEventInput {
    NewEventInput {
        name: "秋の餃子会".to_owned(),
        organizer_note: None,
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![
            CandidateInput {
                local_date: "2026-09-18".to_owned(),
                local_time: "19:00".to_owned(),
            },
            CandidateInput {
                local_date: "2026-09-20".to_owned(),
                local_time: "14:00".to_owned(),
            },
        ],
    }
}

fn prepared(
    respondent_name: &str,
    availabilities: Vec<(i64, Availability)>,
) -> PreparedAvailabilityResponse {
    PreparedAvailabilityResponse {
        respondent_name: respondent_name.to_owned(),
        availabilities: availabilities
            .into_iter()
            .map(|(candidate_id, availability)| CandidateAvailabilityInput {
                candidate_id,
                availability,
            })
            .collect(),
    }
}

async fn create_event(pool: &sqlx::SqlitePool, public_id: &str) -> tsunoru::domain::PublicEvent {
    create_event_record(pool, public_id, &"a".repeat(64), &event_input())
        .await
        .expect("persist fixture event")
}

fn capability_hash(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}

fn temporary_database_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tsunoru-{label}-{}-{nonce}.sqlite3",
        std::process::id()
    ))
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

#[tokio::test]
async fn response_and_availabilities_round_trip_in_one_transaction() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let event = create_event(&pool, "event-one").await;
    let payload = prepared(
        "ミナ",
        vec![
            (event.candidates[0].id, Availability::Available),
            (event.candidates[1].id, Availability::Maybe),
        ],
    );

    let outcome = record_availability_response(
        &pool,
        &event.public_id,
        &capability_hash("response-capability-one"),
        &payload,
    )
    .await
    .expect("persist one complete response");
    assert_eq!(outcome, ResponseWriteOutcome::Created);

    let name: String = sqlx::query_scalar("SELECT respondent_name FROM responses")
        .fetch_one(&pool)
        .await
        .expect("read response");
    let values: Vec<String> = sqlx::query_scalar(
        "SELECT availability FROM response_availabilities ORDER BY candidate_id",
    )
    .fetch_all(&pool)
    .await
    .expect("read candidate values");
    assert_eq!(name, "ミナ");
    assert_eq!(values, vec!["available", "maybe"]);
}

#[tokio::test]
async fn invalid_candidate_rolls_back_the_whole_response() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let event = create_event(&pool, "event-two").await;
    let other = create_event(&pool, "other-event").await;
    let payload = prepared(
        "ミナ",
        vec![
            (event.candidates[0].id, Availability::Available),
            (other.candidates[0].id, Availability::Unavailable),
        ],
    );

    let result = record_availability_response(
        &pool,
        &event.public_id,
        &capability_hash("response-capability-two"),
        &payload,
    )
    .await;
    assert!(matches!(
        result,
        Err(ResponseStorageError::CandidateSetMismatch)
    ));

    let response_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    let value_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM response_availabilities")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((response_count, value_count), (0, 0));
}

#[tokio::test]
async fn missing_duplicate_and_extra_candidates_are_rejected() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let event = create_event(&pool, "event-three").await;
    let other = create_event(&pool, "event-three-other").await;
    let malformed = [
        prepared(
            "missing",
            vec![(event.candidates[0].id, Availability::Available)],
        ),
        prepared(
            "duplicate",
            vec![
                (event.candidates[0].id, Availability::Available),
                (event.candidates[0].id, Availability::Maybe),
            ],
        ),
        prepared(
            "extra",
            vec![
                (event.candidates[0].id, Availability::Available),
                (event.candidates[1].id, Availability::Maybe),
                (other.candidates[0].id, Availability::Unavailable),
            ],
        ),
    ];

    for (index, payload) in malformed.into_iter().enumerate() {
        let result = record_availability_response(
            &pool,
            &event.public_id,
            &capability_hash(&format!("malformed-{index}")),
            &payload,
        )
        .await;
        assert!(matches!(
            result,
            Err(ResponseStorageError::CandidateSetMismatch)
        ));
    }
}

#[tokio::test]
async fn same_capability_and_payload_is_idempotent_after_commit() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let event = create_event(&pool, "event-four").await;
    let payload = prepared(
        "ミナ",
        vec![
            (event.candidates[0].id, Availability::Available),
            (event.candidates[1].id, Availability::Unavailable),
        ],
    );
    let hash = capability_hash("idempotent-capability");

    assert_eq!(
        record_availability_response(&pool, &event.public_id, &hash, &payload)
            .await
            .unwrap(),
        ResponseWriteOutcome::Created
    );
    assert_eq!(
        record_availability_response(&pool, &event.public_id, &hash, &payload)
            .await
            .unwrap(),
        ResponseWriteOutcome::AlreadyRecorded
    );

    let response_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    let value_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM response_availabilities")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((response_count, value_count), (1, 2));
}

#[tokio::test]
async fn same_capability_with_changed_payload_conflicts_without_overwriting() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let event = create_event(&pool, "event-five").await;
    let original = prepared(
        "ミナ",
        vec![
            (event.candidates[0].id, Availability::Available),
            (event.candidates[1].id, Availability::Maybe),
        ],
    );
    let changed = prepared(
        "別の名前",
        vec![
            (event.candidates[0].id, Availability::Unavailable),
            (event.candidates[1].id, Availability::Maybe),
        ],
    );
    let hash = capability_hash("conflicting-capability");

    record_availability_response(&pool, &event.public_id, &hash, &original)
        .await
        .unwrap();
    assert!(matches!(
        record_availability_response(&pool, &event.public_id, &hash, &changed).await,
        Err(ResponseStorageError::CapabilityConflict)
    ));

    let saved_name: String = sqlx::query_scalar("SELECT respondent_name FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    let first_value: String = sqlx::query_scalar(
        "SELECT availability FROM response_availabilities ORDER BY candidate_id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (saved_name.as_str(), first_value.as_str()),
        ("ミナ", "available")
    );
}

#[tokio::test]
async fn same_name_with_different_capabilities_creates_two_responses() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let event = create_event(&pool, "event-six").await;
    let payload = prepared(
        "ミナ",
        vec![
            (event.candidates[0].id, Availability::Available),
            (event.candidates[1].id, Availability::Maybe),
        ],
    );

    for raw in ["same-name-one", "same-name-two"] {
        record_availability_response(&pool, &event.public_id, &capability_hash(raw), &payload)
            .await
            .unwrap();
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2, "a display name is not anonymous identity");
}

#[tokio::test]
async fn raw_response_capability_is_never_persisted() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let event = create_event(&pool, "event-seven").await;
    let raw_capability = "1a".repeat(32);
    let input = NewAvailabilityResponseInput {
        event_public_id: event.public_id.clone(),
        response_capability: raw_capability.clone(),
        response: prepared(
            "ミナ",
            vec![
                (event.candidates[0].id, Availability::Available),
                (event.candidates[1].id, Availability::Maybe),
            ],
        ),
    };

    persist_availability_response(&pool, input)
        .await
        .expect("the service hashes and commits the capability");

    let stored: String = sqlx::query_scalar("SELECT response_capability_hash FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, capability_hash(&raw_capability));
    assert_ne!(stored, raw_capability);

    let raw_occurrences: i64 = sqlx::query_scalar(
        r#"
        SELECT
            (SELECT COUNT(*) FROM responses WHERE response_capability_hash = ?)
            + (SELECT COUNT(*) FROM events WHERE organizer_capability_hash = ?)
        "#,
    )
    .bind(&raw_capability)
    .bind(&raw_capability)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(raw_occurrences, 0);
}

#[tokio::test]
async fn database_rejects_a_cross_event_availability_outside_the_repository() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let event = create_event(&pool, "event-eight").await;
    let other = create_event(&pool, "event-eight-other").await;
    let payload = prepared(
        "ミナ",
        vec![
            (event.candidates[0].id, Availability::Available),
            (event.candidates[1].id, Availability::Maybe),
        ],
    );
    record_availability_response(
        &pool,
        &event.public_id,
        &capability_hash("cross-event-proof"),
        &payload,
    )
    .await
    .unwrap();

    let response_id: i64 = sqlx::query_scalar("SELECT id FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    let result = sqlx::query(
        r#"
        INSERT INTO response_availabilities (
            response_id,
            candidate_id,
            event_public_id,
            availability
        ) VALUES (?, ?, ?, 'available')
        "#,
    )
    .bind(response_id)
    .bind(other.candidates[0].id)
    .bind(&event.public_id)
    .execute(&pool)
    .await;

    assert!(
        result.is_err(),
        "composite foreign keys must reject a candidate from another event"
    );
}

#[tokio::test]
async fn service_distinguishes_an_unknown_event_from_a_database_failure() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let input = NewAvailabilityResponseInput {
        event_public_id: "missing-event".to_owned(),
        response_capability: "ab".repeat(32),
        response: prepared("ミナ", vec![(1, Availability::Available)]),
    };

    assert!(matches!(
        persist_availability_response(&pool, input).await,
        Err(AvailabilityResponseSubmissionError::Storage(
            ResponseStorageError::EventNotFound
        ))
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn simultaneous_identical_retries_create_one_response() {
    let path = temporary_database_path("concurrent-response");
    let pool = open_file(&path).await.expect("open file-backed SQLite");
    let event = create_event(&pool, "event-concurrent").await;
    let payload = prepared(
        "ミナ",
        vec![
            (event.candidates[0].id, Availability::Available),
            (event.candidates[1].id, Availability::Maybe),
        ],
    );
    let hash = capability_hash("simultaneous-capability");
    let barrier = Arc::new(Barrier::new(2));

    let first_pool = pool.clone();
    let first_barrier = Arc::clone(&barrier);
    let first_event_id = event.public_id.clone();
    let first_payload = payload.clone();
    let first_hash = hash.clone();
    let first = tokio::spawn(async move {
        first_barrier.wait();
        record_availability_response(&first_pool, &first_event_id, &first_hash, &first_payload)
            .await
    });

    let second_pool = pool.clone();
    let second_barrier = Arc::clone(&barrier);
    let second_event_id = event.public_id.clone();
    let second = tokio::spawn(async move {
        second_barrier.wait();
        record_availability_response(&second_pool, &second_event_id, &hash, &payload).await
    });

    let outcomes = [
        first.await.expect("first task completes").unwrap(),
        second.await.expect("second task completes").unwrap(),
    ];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == ResponseWriteOutcome::Created)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == ResponseWriteOutcome::AlreadyRecorded)
            .count(),
        1
    );

    let response_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    let availability_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM response_availabilities")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((response_count, availability_count), (1, 2));

    pool.close().await;
    remove_temporary_database(&path);
}

#[tokio::test]
async fn response_schema_enforces_checks_cascade_and_foreign_key_integrity() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let event = create_event(&pool, "event-schema").await;

    let empty_name = sqlx::query(
        r#"
        INSERT INTO responses (event_public_id, respondent_name, response_capability_hash)
        VALUES (?, '   ', ?)
        "#,
    )
    .bind(&event.public_id)
    .bind("a".repeat(64))
    .execute(&pool)
    .await;
    assert!(
        empty_name.is_err(),
        "trimmed empty names must fail the DB CHECK"
    );

    let invalid_hash = sqlx::query(
        r#"
        INSERT INTO responses (event_public_id, respondent_name, response_capability_hash)
        VALUES (?, 'ミナ', ?)
        "#,
    )
    .bind(&event.public_id)
    .bind("G".repeat(64))
    .execute(&pool)
    .await;
    assert!(
        invalid_hash.is_err(),
        "non-lowercase-hex hashes must fail the DB CHECK"
    );

    let inserted = sqlx::query(
        r#"
        INSERT INTO responses (event_public_id, respondent_name, response_capability_hash)
        VALUES (?, 'ミナ', ?)
        "#,
    )
    .bind(&event.public_id)
    .bind("b".repeat(64))
    .execute(&pool)
    .await
    .expect("insert a valid response parent");
    let invalid_availability = sqlx::query(
        r#"
        INSERT INTO response_availabilities (
            response_id,
            candidate_id,
            event_public_id,
            availability
        ) VALUES (?, ?, ?, 'unknown')
        "#,
    )
    .bind(inserted.last_insert_rowid())
    .bind(event.candidates[0].id)
    .bind(&event.public_id)
    .execute(&pool)
    .await;
    assert!(
        invalid_availability.is_err(),
        "unknown values must fail the availability CHECK"
    );

    let violation: Option<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_optional(&pool)
        .await
        .expect("run SQLite foreign-key audit");
    assert_eq!(violation, None);

    sqlx::query("DELETE FROM events WHERE public_id = ?")
        .bind(&event.public_id)
        .execute(&pool)
        .await
        .expect("delete event fixture");
    let response_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    let availability_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM response_availabilities")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((response_count, availability_count), (0, 0));
}
