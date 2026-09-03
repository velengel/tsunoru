#![cfg(feature = "server")]

use sha2::{Digest, Sha256};
use tsunoru::{
    domain::{
        Availability, CandidateAvailabilityInput, CandidateInput, NewEventInput,
        PreparedAvailabilityResponse, PublicEventDecision,
    },
    storage::{
        EventDecisionWriteOutcome, PublicEventStorageError, ResponseStorageError,
        ResponseWriteOutcome, create_event_record, find_public_event, open_in_memory,
        record_availability_response, record_event_decision,
    },
};

fn event_input() -> NewEventInput {
    NewEventInput {
        name: "秋の餃子会".to_owned(),
        organizer_note: Some("焼きたてを囲みたいです".to_owned()),
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

fn response(candidate_ids: &[i64]) -> PreparedAvailabilityResponse {
    PreparedAvailabilityResponse {
        respondent_name: "ミナ".to_owned(),
        availabilities: candidate_ids
            .iter()
            .copied()
            .map(|candidate_id| CandidateAvailabilityInput {
                candidate_id,
                availability: Availability::Available,
            })
            .collect(),
    }
}

#[tokio::test]
async fn public_projection_reveals_only_the_selected_public_candidate() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = "a".repeat(64);
    let event = create_event_record(
        &pool,
        "public-decision-one",
        &organizer_hash,
        &event_input(),
    )
    .await
    .expect("create event");
    let selected = event.candidates[1].clone();

    let (outcome, _) = record_event_decision(&pool, &event.public_id, &organizer_hash, selected.id)
        .await
        .expect("decide event");
    assert_eq!(outcome, EventDecisionWriteOutcome::Created);

    let loaded = find_public_event(&pool, &event.public_id)
        .await
        .expect("load public projection")
        .expect("event exists");
    assert_eq!(
        loaded.decision,
        Some(PublicEventDecision {
            candidate_id: selected.id,
            local_date: selected.local_date,
            local_time: selected.local_time,
        })
    );

    let json = serde_json::to_string(&loaded).expect("serialize public projection");
    for private_name in [
        "decided_at",
        "organizer_capability",
        "organizer_capability_hash",
        "respondent_name",
        "availability",
        "comment",
    ] {
        assert!(
            !json.contains(private_name),
            "public JSON must not expose {private_name}: {json}"
        );
    }
}

#[tokio::test]
async fn public_projection_rejects_a_decision_with_no_joinable_candidate() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = "b".repeat(64);
    let event = create_event_record(
        &pool,
        "public-decision-two",
        &organizer_hash,
        &event_input(),
    )
    .await
    .expect("create event");
    let selected_id = event.candidates[0].id;
    record_event_decision(&pool, &event.public_id, &organizer_hash, selected_id)
        .await
        .expect("decide event");

    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .expect("disable constraints for corrupt fixture");
    sqlx::query("DELETE FROM candidates WHERE id = ?")
        .bind(selected_id)
        .execute(&pool)
        .await
        .expect("remove selected candidate");
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("restore constraints");

    let error = find_public_event(&pool, &event.public_id)
        .await
        .expect_err("a dangling public decision must not be projected");
    assert!(matches!(
        error,
        PublicEventStorageError::DataInvariantViolation
    ));
}

#[tokio::test]
async fn a_new_answer_loses_to_an_already_committed_decision() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = "c".repeat(64);
    let event = create_event_record(
        &pool,
        "public-decision-three",
        &organizer_hash,
        &event_input(),
    )
    .await
    .expect("create event");
    let candidate_ids = event
        .candidates
        .iter()
        .map(|candidate| candidate.id)
        .collect::<Vec<_>>();
    record_event_decision(&pool, &event.public_id, &organizer_hash, candidate_ids[0])
        .await
        .expect("decide event");

    let result = record_availability_response(
        &pool,
        &event.public_id,
        &format!("{:x}", Sha256::digest(b"new-response-after-decision")),
        &response(&candidate_ids),
    )
    .await;
    assert!(matches!(result, Err(ResponseStorageError::EventDecided)));
}

#[tokio::test]
async fn a_committed_answer_retry_remains_successful_after_the_decision() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = "d".repeat(64);
    let event = create_event_record(
        &pool,
        "public-decision-four",
        &organizer_hash,
        &event_input(),
    )
    .await
    .expect("create event");
    let candidate_ids = event
        .candidates
        .iter()
        .map(|candidate| candidate.id)
        .collect::<Vec<_>>();
    let response_hash = format!("{:x}", Sha256::digest(b"committed-before-decision"));

    assert_eq!(
        record_availability_response(
            &pool,
            &event.public_id,
            &response_hash,
            &response(&candidate_ids),
        )
        .await
        .expect("save response"),
        ResponseWriteOutcome::Created
    );
    record_event_decision(&pool, &event.public_id, &organizer_hash, candidate_ids[0])
        .await
        .expect("decide event");

    assert_eq!(
        record_availability_response(
            &pool,
            &event.public_id,
            &response_hash,
            &response(&candidate_ids),
        )
        .await
        .expect("replay committed response"),
        ResponseWriteOutcome::AlreadyRecorded
    );
}
