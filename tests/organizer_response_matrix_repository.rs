#![cfg(feature = "server")]

use sha2::{Digest, Sha256};
use tsunoru::{
    domain::{
        Availability, CandidateAvailabilityInput, CandidateInput, NewEventInput,
        OrganizerResponseMatrix, PreparedAvailabilityResponse,
    },
    storage::{
        OrganizerResponseMatrixStorageError, ResponseMatrixStorageError, create_event_record,
        find_organizer_response_matrix, find_participant_response_matrix, open_in_memory,
        record_availability_response, record_response_comment,
    },
};

fn event_input() -> NewEventInput {
    NewEventInput {
        name: "秋の餃子会".to_owned(),
        organizer_note: Some("駅の近くで集まりたいです".to_owned()),
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![
            CandidateInput {
                local_date: "2026-09-20".to_owned(),
                local_time: "14:00".to_owned(),
            },
            CandidateInput {
                local_date: "2026-09-18".to_owned(),
                local_time: "19:00".to_owned(),
            },
            CandidateInput {
                local_date: "2026-09-21".to_owned(),
                local_time: "12:30".to_owned(),
            },
        ],
    }
}

#[tokio::test]
async fn an_answer_capability_reads_the_complete_matrix_only_for_its_own_event() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = capability_hash("participant-matrix-organizer");
    let event = create_event(&pool, "participant-matrix", &organizer_hash).await;
    add_response(
        &pool,
        &event,
        "participant-matrix-one",
        "ミナ",
        [
            Availability::Available,
            Availability::Maybe,
            Availability::Unavailable,
        ],
        None,
    )
    .await;
    add_response(
        &pool,
        &event,
        "participant-matrix-two",
        "ソラ",
        [
            Availability::Maybe,
            Availability::Available,
            Availability::Unavailable,
        ],
        None,
    )
    .await;

    let matrix = find_participant_response_matrix(
        &pool,
        &event.public_id,
        &capability_hash("participant-matrix-one"),
    )
    .await
    .expect("one recorded participant may read the complete matrix");
    assert_eq!(matrix.responses.len(), 2);
    assert_eq!(matrix.responses[0].respondent_name, "ミナ");
    assert_eq!(matrix.responses[1].respondent_name, "ソラ");

    assert!(matches!(
        find_participant_response_matrix(
            &pool,
            &event.public_id,
            &capability_hash("not-an-answer"),
        )
        .await,
        Err(ResponseMatrixStorageError::NotFound)
    ));
    assert!(matches!(
        find_participant_response_matrix(
            &pool,
            "different-event",
            &capability_hash("participant-matrix-one"),
        )
        .await,
        Err(ResponseMatrixStorageError::NotFound)
    ));
}

fn capability_hash(seed: &str) -> String {
    format!("{:x}", Sha256::digest(seed.as_bytes()))
}

async fn create_event(
    pool: &sqlx::SqlitePool,
    public_id: &str,
    organizer_capability_hash: &str,
) -> tsunoru::domain::PublicEvent {
    create_event_record(pool, public_id, organizer_capability_hash, &event_input())
        .await
        .expect("persist fixture event")
}

async fn add_response(
    pool: &sqlx::SqlitePool,
    event: &tsunoru::domain::PublicEvent,
    capability_seed: &str,
    respondent_name: &str,
    availabilities: [Availability; 3],
    comment: Option<&str>,
) {
    let response_capability_hash = capability_hash(capability_seed);
    let response = PreparedAvailabilityResponse {
        respondent_name: respondent_name.to_owned(),
        availabilities: event
            .candidates
            .iter()
            .zip(availabilities)
            .map(|(candidate, availability)| CandidateAvailabilityInput {
                candidate_id: candidate.id,
                availability,
            })
            .collect(),
    };

    record_availability_response(pool, &event.public_id, &response_capability_hash, &response)
        .await
        .expect("persist fixture response");
    if let Some(comment) = comment {
        record_response_comment(pool, &event.public_id, &response_capability_hash, comment)
            .await
            .expect("persist fixture comment");
    }
}

#[tokio::test]
async fn reconstructs_every_cell_in_candidate_and_response_order_without_merging_names() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = capability_hash("matrix-organizer");
    let event = create_event(&pool, "matrix-complete", &organizer_hash).await;

    add_response(
        &pool,
        &event,
        "matrix-one",
        "ミナ",
        [
            Availability::Available,
            Availability::Maybe,
            Availability::Unavailable,
        ],
        Some("表には返してはいけないコメント"),
    )
    .await;
    add_response(
        &pool,
        &event,
        "matrix-two",
        "ミナ",
        [
            Availability::Unavailable,
            Availability::Available,
            Availability::Maybe,
        ],
        None,
    )
    .await;
    add_response(
        &pool,
        &event,
        "matrix-three",
        "ソラ",
        [
            Availability::Maybe,
            Availability::Unavailable,
            Availability::Available,
        ],
        None,
    )
    .await;

    let matrix: OrganizerResponseMatrix =
        find_organizer_response_matrix(&pool, &event.public_id, &organizer_hash)
            .await
            .expect("load organizer response matrix");

    assert_eq!(matrix.name, "秋の餃子会");
    assert_eq!(matrix.time_zone, "Asia/Tokyo");
    assert_eq!(
        matrix
            .candidates
            .iter()
            .map(|candidate| (candidate.local_date.as_str(), candidate.local_time.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("2026-09-20", "14:00"),
            ("2026-09-18", "19:00"),
            ("2026-09-21", "12:30"),
        ],
        "candidate columns retain authored position, not chronological order"
    );
    assert_eq!(matrix.responses.len(), 3);
    assert_eq!(matrix.responses[0].respondent_name, "ミナ");
    assert_eq!(
        matrix.responses[0].availabilities,
        vec![
            Availability::Available,
            Availability::Maybe,
            Availability::Unavailable,
        ]
    );
    assert_eq!(matrix.responses[1].respondent_name, "ミナ");
    assert_eq!(
        matrix.responses[1].availabilities,
        vec![
            Availability::Unavailable,
            Availability::Available,
            Availability::Maybe,
        ]
    );
    assert_eq!(matrix.responses[2].respondent_name, "ソラ");
    assert_eq!(
        matrix.responses[2].availabilities,
        vec![
            Availability::Maybe,
            Availability::Unavailable,
            Availability::Available,
        ],
        "response-id order is deterministic and equal display names remain distinct"
    );

    let json = serde_json::to_string(&matrix).expect("serialize matrix projection");
    for forbidden in [
        "表には返してはいけないコメント",
        "comment",
        "public_id",
        "candidate_id",
        "response_id",
        "capability",
        "hash",
    ] {
        assert!(
            !json.contains(forbidden),
            "matrix leaked {forbidden}: {json}"
        );
    }
}

#[tokio::test]
async fn zero_responses_returns_authored_candidates_and_an_empty_row_vector() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = capability_hash("matrix-zero");
    let event = create_event(&pool, "matrix-zero", &organizer_hash).await;

    let matrix = find_organizer_response_matrix(&pool, &event.public_id, &organizer_hash)
        .await
        .expect("zero responses is a valid matrix");

    assert_eq!(matrix.candidates.len(), 3);
    assert!(matrix.responses.is_empty());
}

#[tokio::test]
async fn wrong_missing_and_cross_event_hashes_are_indistinguishable_not_found() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let first_hash = capability_hash("matrix-first");
    let other_hash = capability_hash("matrix-other");
    let first = create_event(&pool, "matrix-private", &first_hash).await;
    create_event(&pool, "matrix-private-other", &other_hash).await;

    for (event_public_id, presented_hash) in [
        (first.public_id.as_str(), capability_hash("wrong-secret")),
        (first.public_id.as_str(), other_hash),
        ("missing-matrix", first_hash),
    ] {
        assert!(matches!(
            find_organizer_response_matrix(&pool, event_public_id, &presented_hash).await,
            Err(OrganizerResponseMatrixStorageError::NotFound)
        ));
    }
}

#[tokio::test]
async fn incomplete_response_matrix_is_rejected_instead_of_returning_a_partial_row() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = capability_hash("matrix-invariant");
    let event = create_event(&pool, "matrix-invariant", &organizer_hash).await;
    add_response(
        &pool,
        &event,
        "matrix-incomplete",
        "ミナ",
        [
            Availability::Available,
            Availability::Maybe,
            Availability::Unavailable,
        ],
        None,
    )
    .await;

    sqlx::query(
        r#"
        DELETE FROM response_availabilities
        WHERE response_id = (SELECT id FROM responses WHERE event_public_id = ? LIMIT 1)
          AND candidate_id = ?
        "#,
    )
    .bind(&event.public_id)
    .bind(event.candidates[1].id)
    .execute(&pool)
    .await
    .expect("create an otherwise schema-valid incomplete aggregate fixture");

    assert!(matches!(
        find_organizer_response_matrix(&pool, &event.public_id, &organizer_hash).await,
        Err(OrganizerResponseMatrixStorageError::DataInvariantViolation)
    ));
}

#[tokio::test]
async fn an_extra_cell_with_an_unknown_response_id_is_not_filtered_before_validation() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = capability_hash("matrix-unknown-response");
    let event = create_event(&pool, "matrix-unknown-response", &organizer_hash).await;
    add_response(
        &pool,
        &event,
        "matrix-known-response",
        "ミナ",
        [
            Availability::Available,
            Availability::Maybe,
            Availability::Unavailable,
        ],
        None,
    )
    .await;

    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .expect("allow an isolated corrupted-row fixture");
    sqlx::query(
        r#"
        INSERT INTO response_availabilities (
            response_id,
            candidate_id,
            event_public_id,
            availability
        ) VALUES (?, ?, ?, 'available')
        "#,
    )
    .bind(9_999_999_i64)
    .bind(event.candidates[0].id)
    .bind(&event.public_id)
    .execute(&pool)
    .await
    .expect("insert a cell whose response does not exist");
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("restore foreign-key enforcement");

    assert!(matches!(
        find_organizer_response_matrix(&pool, &event.public_id, &organizer_hash).await,
        Err(OrganizerResponseMatrixStorageError::DataInvariantViolation)
    ));
}

#[tokio::test]
async fn matrix_cell_lookup_uses_an_event_scoped_index() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let plan = sqlx::query_as::<_, (i64, i64, i64, String)>(
        r#"
        EXPLAIN QUERY PLAN
        SELECT response_id, candidate_id, availability
        FROM response_availabilities
        WHERE event_public_id = ?
        "#,
    )
    .bind("matrix-query-plan")
    .fetch_all(&pool)
    .await
    .expect("inspect matrix cell lookup plan");

    assert!(
        plan.iter().any(|(_, _, _, detail)| {
            detail.contains("response_availabilities_event_public_id_idx")
        }),
        "matrix reads must not scan cells belonging to every event: {plan:?}"
    );
}
