#![cfg(feature = "server")]

use dioxus::prelude::ServerFnError;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::future::Future;
use tsunoru::{
    domain::{
        Availability, CandidateAvailabilityInput, CandidateInput, NewEventInput,
        OrganizerDecisionInput, OrganizerEventDecision, PreparedAvailabilityResponse,
    },
    server::{get_organizer_event_decision, persist_organizer_event_decision},
    storage::{
        create_event_record, open_in_memory, record_availability_response, record_response_comment,
    },
};

const EVENT_PUBLIC_ID: &str = "80da27df-2ec9-4a18-8591-a1a108b31c1e";
const OTHER_EVENT_PUBLIC_ID: &str = "f1f10dd8-c6b1-4290-87f9-bbe60ed076c9";
const MISSING_EVENT_PUBLIC_ID: &str = "c488de09-7435-4fcc-ac3e-5b9864c52477";
const PRIVATE_EVENT_NAME: &str = "private-decision-event-sentinel";
const PRIVATE_OTHER_EVENT_NAME: &str = "private-other-decision-event-sentinel";
const PRIVATE_ORGANIZER_NOTE: &str = "private-decision-note-sentinel";
const PRIVATE_RESPONDENT_NAME: &str = "private-decision-respondent-sentinel";
const PRIVATE_COMMENT: &str = "private-decision-comment-sentinel";

fn capability(byte_pair: &str) -> String {
    assert_eq!(byte_pair.len(), 2);
    byte_pair.repeat(32)
}

fn capability_hash(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}

fn decision_input(
    event_public_id: &str,
    candidate_id: i64,
    organizer_capability: &str,
) -> OrganizerDecisionInput {
    OrganizerDecisionInput {
        event_public_id: event_public_id.to_owned(),
        candidate_id,
        organizer_capability: organizer_capability.to_owned(),
    }
}

fn require_http_error_contract<F>(_: F)
where
    F: Future<Output = std::result::Result<OrganizerEventDecision, ServerFnError>>,
{
}

fn decision_endpoint_source() -> &'static str {
    let server_source = include_str!("../src/server.rs");
    let endpoint_and_tail = server_source
        .split_once("#[post(\"/api/organizer/events/decision\")]")
        .map(|(_, tail)| tail)
        .expect("the decision endpoint must use a private POST body");

    endpoint_and_tail
        .split_once("\n/// ")
        .map(|(endpoint, _)| endpoint)
        .unwrap_or(endpoint_and_tail)
}

async fn create_event(
    pool: &sqlx::SqlitePool,
    public_id: &str,
    raw_organizer_capability: &str,
    name: &str,
) -> tsunoru::domain::PublicEvent {
    create_event_record(
        pool,
        public_id,
        &capability_hash(raw_organizer_capability),
        &NewEventInput {
            name: name.to_owned(),
            organizer_note: Some(PRIVATE_ORGANIZER_NOTE.to_owned()),
            time_zone: "Asia/Tokyo".to_owned(),
            candidates: vec![
                CandidateInput {
                    local_date: "2026-10-03".to_owned(),
                    local_time: "18:30".to_owned(),
                },
                CandidateInput {
                    local_date: "2026-10-10".to_owned(),
                    local_time: "12:00".to_owned(),
                },
            ],
        },
    )
    .await
    .expect("persist fixture event")
}

async fn add_private_response(
    pool: &sqlx::SqlitePool,
    event: &tsunoru::domain::PublicEvent,
    raw_response_capability: &str,
) {
    let response_capability_hash = capability_hash(raw_response_capability);
    record_availability_response(
        pool,
        &event.public_id,
        &response_capability_hash,
        &PreparedAvailabilityResponse {
            respondent_name: PRIVATE_RESPONDENT_NAME.to_owned(),
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
        },
    )
    .await
    .expect("persist private response");
    record_response_comment(
        pool,
        &event.public_id,
        &response_capability_hash,
        PRIVATE_COMMENT,
    )
    .await
    .expect("persist private response comment");
}

#[test]
fn decision_input_normalizes_valid_fields_and_redacts_bearer_authority() {
    let raw_capability = capability("ab");
    let input = decision_input(
        &format!("  {EVENT_PUBLIC_ID}  "),
        42,
        &format!("  {raw_capability}  "),
    );
    require_http_error_contract(get_organizer_event_decision(input.clone()));

    let debug = format!("{input:?}");
    assert!(debug.contains("OrganizerDecisionInput"));
    assert!(!debug.contains(&raw_capability));

    let normalized = input
        .normalized_and_validated()
        .expect("valid decision input should normalize");
    assert_eq!(normalized.event_public_id, EVENT_PUBLIC_ID);
    assert_eq!(normalized.candidate_id, 42);
    assert_eq!(normalized.organizer_capability, raw_capability);

    for invalid in [
        decision_input("", 42, &capability("ab")),
        decision_input("not/a/public/id", 42, &capability("ab")),
        decision_input(EVENT_PUBLIC_ID, 0, &capability("ab")),
        decision_input(EVENT_PUBLIC_ID, -1, &capability("ab")),
        decision_input(EVENT_PUBLIC_ID, 42, &"a".repeat(63)),
        decision_input(EVENT_PUBLIC_ID, 42, &"AB".repeat(32)),
    ] {
        assert!(
            invalid.normalized_and_validated().is_err(),
            "invalid decision request must be rejected: {invalid:?}"
        );
    }
}

#[test]
fn decision_endpoint_pins_post_no_store_and_explicit_http_statuses() {
    let endpoint = decision_endpoint_source();

    assert!(
        endpoint.contains("CACHE_CONTROL") && endpoint.contains("no-store"),
        "successful and failed private decision responses must opt out of storage: {endpoint}"
    );
    assert!(
        endpoint.matches("code: 422").count() >= 2,
        "input validation and candidate mismatch must each map to 422: {endpoint}"
    );
    assert!(
        endpoint.contains("code: 404"),
        "missing, wrong, and cross-event authority must map to one 404: {endpoint}"
    );
    assert!(
        endpoint.contains("code: 409"),
        "a different committed decision must map to 409: {endpoint}"
    );
    assert!(
        endpoint.contains("ServerFnError::new"),
        "database failures must map to a generic 500 response: {endpoint}"
    );
}

#[tokio::test]
async fn wrong_cross_event_and_missing_authority_share_one_non_exposing_error() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = capability("ab");
    let other_capability = capability("cd");
    let wrong_capability = capability("ef");
    let raw_capability_hash = capability_hash(&raw_capability);
    let other_capability_hash = capability_hash(&other_capability);
    let wrong_capability_hash = capability_hash(&wrong_capability);
    let event = create_event(&pool, EVENT_PUBLIC_ID, &raw_capability, PRIVATE_EVENT_NAME).await;
    create_event(
        &pool,
        OTHER_EVENT_PUBLIC_ID,
        &other_capability,
        PRIVATE_OTHER_EVENT_NAME,
    )
    .await;

    let requests = [
        decision_input(EVENT_PUBLIC_ID, event.candidates[0].id, &wrong_capability),
        decision_input(EVENT_PUBLIC_ID, event.candidates[0].id, &other_capability),
        decision_input(
            MISSING_EVENT_PUBLIC_ID,
            event.candidates[0].id,
            &raw_capability,
        ),
    ];
    let mut public_messages = Vec::new();

    for request in requests {
        let error = persist_organizer_event_decision(&pool, request)
            .await
            .expect_err("unauthorized requests must not create a decision");
        let display = error.to_string();
        let combined = format!("{display}\n{error:?}");
        for forbidden in [
            raw_capability.as_str(),
            other_capability.as_str(),
            wrong_capability.as_str(),
            raw_capability_hash.as_str(),
            other_capability_hash.as_str(),
            wrong_capability_hash.as_str(),
            PRIVATE_EVENT_NAME,
            PRIVATE_OTHER_EVENT_NAME,
            PRIVATE_ORGANIZER_NOTE,
        ] {
            assert!(
                !combined.contains(forbidden),
                "decision authorization error disclosed {forbidden}: {combined}"
            );
        }
        public_messages.push(display);
    }

    assert!(
        public_messages.windows(2).all(|pair| pair[0] == pair[1]),
        "wrong, cross-event, and missing authority must be indistinguishable: {public_messages:?}"
    );
}

#[tokio::test]
async fn a_candidate_from_another_event_is_rejected_without_private_data() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = capability("ab");
    let other_capability = capability("cd");
    let raw_capability_hash = capability_hash(&raw_capability);
    create_event(&pool, EVENT_PUBLIC_ID, &raw_capability, PRIVATE_EVENT_NAME).await;
    let other = create_event(
        &pool,
        OTHER_EVENT_PUBLIC_ID,
        &other_capability,
        PRIVATE_OTHER_EVENT_NAME,
    )
    .await;

    let error = persist_organizer_event_decision(
        &pool,
        decision_input(EVENT_PUBLIC_ID, other.candidates[0].id, &raw_capability),
    )
    .await
    .expect_err("an authorized organizer cannot decide another event's candidate");
    let combined = format!("{error}\n{error:?}");
    for forbidden in [
        raw_capability.as_str(),
        raw_capability_hash.as_str(),
        other_capability.as_str(),
        PRIVATE_EVENT_NAME,
        PRIVATE_OTHER_EVENT_NAME,
        PRIVATE_ORGANIZER_NOTE,
    ] {
        assert!(
            !combined.contains(forbidden),
            "candidate mismatch error disclosed {forbidden}: {combined}"
        );
    }
}

#[tokio::test]
async fn authorized_decision_is_minimal_idempotent_and_never_overwritten() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = capability("ab");
    let response_capability = capability("11");
    let raw_capability_hash = capability_hash(&raw_capability);
    let response_capability_hash = capability_hash(&response_capability);
    let event = create_event(&pool, EVENT_PUBLIC_ID, &raw_capability, PRIVATE_EVENT_NAME).await;
    add_private_response(&pool, &event, &response_capability).await;

    let first_input = decision_input(EVENT_PUBLIC_ID, event.candidates[0].id, &raw_capability);
    let first = persist_organizer_event_decision(&pool, first_input.clone())
        .await
        .expect("an authorized organizer can commit one candidate");
    let retry = persist_organizer_event_decision(&pool, first_input.clone())
        .await
        .expect("retrying the same candidate is an idempotent success");
    let expected = json!({
        "candidate_id": event.candidates[0].id,
        "local_date": "2026-10-03",
        "local_time": "18:30",
    });
    assert_eq!(
        serde_json::to_value(&first).expect("serialize first decision"),
        expected
    );
    assert_eq!(
        serde_json::to_value(&retry).expect("serialize retry decision"),
        expected
    );

    let conflict = persist_organizer_event_decision(
        &pool,
        decision_input(EVENT_PUBLIC_ID, event.candidates[1].id, &raw_capability),
    )
    .await
    .expect_err("a different candidate must not overwrite the first committed decision");
    let conflict_text = format!("{conflict}\n{conflict:?}");
    for forbidden in [
        raw_capability.as_str(),
        raw_capability_hash.as_str(),
        response_capability.as_str(),
        response_capability_hash.as_str(),
        PRIVATE_EVENT_NAME,
        PRIVATE_ORGANIZER_NOTE,
        PRIVATE_RESPONDENT_NAME,
        PRIVATE_COMMENT,
    ] {
        assert!(
            !conflict_text.contains(forbidden),
            "decision conflict disclosed {forbidden}: {conflict_text}"
        );
    }

    let after_conflict = persist_organizer_event_decision(&pool, first_input)
        .await
        .expect("the first decision remains after a conflicting request");
    assert_eq!(
        serde_json::to_value(after_conflict).expect("serialize retained decision"),
        expected
    );
}
