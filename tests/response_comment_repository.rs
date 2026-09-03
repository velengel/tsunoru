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
        NewEventInput, NewResponseCommentInput, PreparedAvailabilityResponse,
        RESPONDENT_COMMENT_MAX_CHARS,
    },
    server::{
        ResponseCommentSubmissionError, persist_availability_response, persist_response_comment,
    },
    storage::{
        ResponseCommentStorageError, ResponseCommentWriteOutcome, create_event_record,
        find_public_event, open_file, open_in_memory, record_response_comment,
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

fn capability_hash(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}

async fn create_answer(
    pool: &sqlx::SqlitePool,
    event_public_id: &str,
    raw_capability: &str,
) -> tsunoru::domain::PublicEvent {
    let event = create_event_record(pool, event_public_id, &"a".repeat(64), &event_input())
        .await
        .expect("persist fixture event");
    let response = PreparedAvailabilityResponse {
        respondent_name: "ミナ".to_owned(),
        availabilities: vec![
            CandidateAvailabilityInput {
                candidate_id: event.candidates[0].id,
                availability: Availability::Available,
            },
            CandidateAvailabilityInput {
                candidate_id: event.candidates[1].id,
                availability: Availability::Maybe,
            },
        ],
    };
    persist_availability_response(
        pool,
        NewAvailabilityResponseInput {
            event_public_id: event.public_id.clone(),
            response_capability: raw_capability.to_owned(),
            response,
        },
    )
    .await
    .expect("persist fixture response");
    event
}

fn comment_input(
    event_public_id: &str,
    raw_capability: &str,
    comment: &str,
) -> NewResponseCommentInput {
    NewResponseCommentInput {
        event_public_id: event_public_id.to_owned(),
        response_capability: raw_capability.to_owned(),
        comment: comment.to_owned(),
    }
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
async fn valid_capability_adds_one_normalized_comment_without_changing_the_answer() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = "1a".repeat(32);
    let event = create_answer(&pool, "comment-event-one", &raw_capability).await;
    let before_response_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    let before_availability_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM response_availabilities")
            .fetch_one(&pool)
            .await
            .unwrap();

    let outcome = persist_response_comment(
        &pool,
        comment_input(
            &event.public_id,
            &raw_capability,
            "  調整ありがとう！\n楽しみです  ",
        ),
    )
    .await
    .expect("authorize and persist the optional comment");

    assert_eq!(outcome, ResponseCommentWriteOutcome::Created);
    let stored: String = sqlx::query_scalar("SELECT respondent_comment FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, "調整ありがとう！\n楽しみです");

    let after_response_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    let after_availability_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM response_availabilities")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        (after_response_count, after_availability_count),
        (before_response_count, before_availability_count),
        "adding a comment must not create or replace answer rows"
    );

    let stored_hash: String = sqlx::query_scalar("SELECT response_capability_hash FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored_hash, capability_hash(&raw_capability));
    assert_ne!(stored_hash, raw_capability);
    let raw_occurrences: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM responses
        WHERE response_capability_hash = ? OR respondent_comment = ?
        "#,
    )
    .bind(&raw_capability)
    .bind(&raw_capability)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(raw_occurrences, 0, "the raw bearer secret must not persist");

    let public_projection = find_public_event(&pool, &event.public_id)
        .await
        .expect("read the public-by-link projection")
        .expect("fixture event remains public-by-link");
    let public_json = serde_json::to_string(&public_projection).unwrap();
    assert!(
        !public_json.contains("respondent_comment")
            && !public_json.contains("調整ありがとう")
            && !public_json.contains("ミナ"),
        "Story 3 must not publish responses or comments before Story 4 decides read access"
    );
}

#[tokio::test]
async fn another_event_or_capability_cannot_select_a_response() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = "2b".repeat(32);
    let other_capability = "3c".repeat(32);
    let event = create_answer(&pool, "comment-event-two", &raw_capability).await;
    let other = create_answer(&pool, "comment-event-other", &other_capability).await;

    for input in [
        comment_input(&other.public_id, &raw_capability, "別eventには付けられない"),
        comment_input(&event.public_id, &"4d".repeat(32), "秘密が違う"),
    ] {
        assert!(matches!(
            persist_response_comment(&pool, input).await,
            Err(ResponseCommentSubmissionError::Storage(
                ResponseCommentStorageError::ResponseNotFound
            ))
        ));
    }

    let comments: Vec<Option<String>> =
        sqlx::query_scalar("SELECT respondent_comment FROM responses ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(comments, vec![None, None]);
}

#[tokio::test]
async fn identical_retry_succeeds_but_changed_text_conflicts_without_overwriting() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = "5e".repeat(32);
    let event = create_answer(&pool, "comment-event-three", &raw_capability).await;

    assert_eq!(
        persist_response_comment(
            &pool,
            comment_input(&event.public_id, &raw_capability, "  楽しみ！  "),
        )
        .await
        .unwrap(),
        ResponseCommentWriteOutcome::Created
    );
    assert_eq!(
        persist_response_comment(
            &pool,
            comment_input(&event.public_id, &raw_capability, "楽しみ！"),
        )
        .await
        .unwrap(),
        ResponseCommentWriteOutcome::AlreadyRecorded
    );
    assert!(matches!(
        persist_response_comment(
            &pool,
            comment_input(&event.public_id, &raw_capability, "別のひとこと"),
        )
        .await,
        Err(ResponseCommentSubmissionError::Storage(
            ResponseCommentStorageError::CommentConflict
        ))
    ));

    let stored: String = sqlx::query_scalar("SELECT respondent_comment FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, "楽しみ！");
}

#[tokio::test]
async fn invalid_comment_requests_are_rejected_before_the_database_changes() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = "6f".repeat(32);
    let event = create_answer(&pool, "comment-event-four", &raw_capability).await;

    let invalid_inputs = [
        comment_input(&event.public_id, &raw_capability, "   \n  "),
        comment_input(
            &event.public_id,
            &raw_capability,
            &"長".repeat(RESPONDENT_COMMENT_MAX_CHARS + 1),
        ),
        comment_input(&event.public_id, &raw_capability, "途中\0にNUL"),
        comment_input("../other-event", &raw_capability, "不正なevent"),
        comment_input(&event.public_id, &"A".repeat(64), "不正なcapability"),
    ];

    for input in invalid_inputs {
        assert!(matches!(
            persist_response_comment(&pool, input).await,
            Err(ResponseCommentSubmissionError::Validation(_))
        ));
    }

    let stored: Option<String> = sqlx::query_scalar("SELECT respondent_comment FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, None);
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
    assert!(matches!(
        persist_response_comment(
            &pool,
            comment_input(&event.public_id, &raw_capability, "   "),
        )
        .await,
        Err(ResponseCommentSubmissionError::Validation(_))
    ));
}

#[tokio::test]
async fn response_comment_schema_enforces_nullable_bounds_and_event_cascade() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = "7a".repeat(32);
    let event = create_answer(&pool, "comment-event-five", &raw_capability).await;

    let initial: Option<String> = sqlx::query_scalar("SELECT respondent_comment FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(initial, None, "existing answers start without a comment");

    for invalid in [
        "   ".to_owned(),
        "長".repeat(RESPONDENT_COMMENT_MAX_CHARS + 1),
        format!("a\0{}", "b".repeat(RESPONDENT_COMMENT_MAX_CHARS + 1)),
    ] {
        let result = sqlx::query("UPDATE responses SET respondent_comment = ?")
            .bind(invalid)
            .execute(&pool)
            .await;
        assert!(result.is_err(), "the DB CHECK must reject invalid text");
    }

    sqlx::query("UPDATE responses SET respondent_comment = ?")
        .bind("肉！")
        .execute(&pool)
        .await
        .expect("the DB accepts a bounded nonblank comment");
    sqlx::query("DELETE FROM events WHERE public_id = ?")
        .bind(&event.public_id)
        .execute(&pool)
        .await
        .expect("delete fixture event");

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn simultaneous_identical_comments_create_one_value() {
    let path = temporary_database_path("concurrent-response-comment");
    let pool = open_file(&path).await.expect("open file-backed SQLite");
    let raw_capability = "8b".repeat(32);
    let event = create_answer(&pool, "comment-event-concurrent", &raw_capability).await;
    let hash = capability_hash(&raw_capability);
    let barrier = Arc::new(Barrier::new(2));

    let first_pool = pool.clone();
    let first_barrier = Arc::clone(&barrier);
    let first_event_id = event.public_id.clone();
    let first_hash = hash.clone();
    let first = tokio::spawn(async move {
        first_barrier.wait();
        record_response_comment(
            &first_pool,
            &first_event_id,
            &first_hash,
            "調整ありがとう！",
        )
        .await
    });

    let second_pool = pool.clone();
    let second_barrier = Arc::clone(&barrier);
    let second_event_id = event.public_id.clone();
    let second = tokio::spawn(async move {
        second_barrier.wait();
        record_response_comment(&second_pool, &second_event_id, &hash, "調整ありがとう！").await
    });

    let outcomes = [
        first.await.expect("first task completes").unwrap(),
        second.await.expect("second task completes").unwrap(),
    ];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == ResponseCommentWriteOutcome::Created)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == ResponseCommentWriteOutcome::AlreadyRecorded)
            .count(),
        1
    );
    let stored: String = sqlx::query_scalar("SELECT respondent_comment FROM responses")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, "調整ありがとう！");

    pool.close().await;
    remove_temporary_database(&path);
}
