#![cfg(feature = "server")]

use dioxus::prelude::ServerFnError;
use sha2::{Digest, Sha256};
use std::future::Future;
use tsunoru::{
    domain::{
        Availability, CandidateAvailabilityInput, CandidateInput, NewEventInput,
        OrganizerResponseMatrix, OrganizerSummaryInput, PreparedAvailabilityResponse,
    },
    server::{get_organizer_response_matrix, persist_organizer_response_matrix},
    storage::{
        create_event_record, open_in_memory, record_availability_response, record_response_comment,
    },
};

const EVENT_PUBLIC_ID: &str = "7af78527-813b-4cdd-a632-058f3ce885aa";
const OTHER_EVENT_PUBLIC_ID: &str = "5d70514a-575f-4079-9be9-5bca4563f84c";
const PRIVATE_NAME: &str = "private-matrix-name-sentinel";
const PRIVATE_COMMENT: &str = "private-matrix-comment-sentinel";

fn capability(byte_pair: &str) -> String {
    assert_eq!(byte_pair.len(), 2);
    byte_pair.repeat(32)
}

fn capability_hash(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}

fn matrix_input(event_public_id: &str, organizer_capability: &str) -> OrganizerSummaryInput {
    OrganizerSummaryInput {
        event_public_id: event_public_id.to_owned(),
        organizer_capability: organizer_capability.to_owned(),
    }
}

fn require_http_error_contract<F>(_: F)
where
    F: Future<Output = std::result::Result<OrganizerResponseMatrix, ServerFnError>>,
{
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
            organizer_note: Some("表へは含めない主催者メモ".to_owned()),
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
    let response_hash = capability_hash(raw_response_capability);
    record_availability_response(
        pool,
        &event.public_id,
        &response_hash,
        &PreparedAvailabilityResponse {
            respondent_name: PRIVATE_NAME.to_owned(),
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
    record_response_comment(pool, &event.public_id, &response_hash, PRIVATE_COMMENT)
        .await
        .expect("persist private comment");
}

#[test]
fn organizer_matrix_reuses_the_redacted_post_input_and_explicit_http_error_contract() {
    let raw_capability = capability("ab");
    let input = matrix_input(EVENT_PUBLIC_ID, &raw_capability);
    require_http_error_contract(get_organizer_response_matrix(input.clone()));

    let debug = format!("{input:?}");
    assert!(debug.contains("OrganizerSummaryInput"));
    assert!(!debug.contains(&raw_capability));

    let server_source = include_str!("../src/server.rs");
    let endpoint = server_source
        .split_once("#[post(\"/api/organizer/events/matrix\")]")
        .map(|(_, tail)| tail)
        .expect("the matrix must use a private POST body");
    assert!(
        endpoint.contains("CACHE_CONTROL") && endpoint.contains("no-store"),
        "successful and failed private matrix responses must opt out of storage"
    );
}

#[tokio::test]
async fn wrong_cross_event_and_missing_authority_share_one_non_exposing_error() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = capability("ab");
    let other_capability = capability("cd");
    let wrong_capability = capability("ef");
    let response_capability = capability("11");
    let event = create_event(
        &pool,
        EVENT_PUBLIC_ID,
        &raw_capability,
        "private-event-sentinel",
    )
    .await;
    create_event(
        &pool,
        OTHER_EVENT_PUBLIC_ID,
        &other_capability,
        "other-private-event-sentinel",
    )
    .await;
    add_private_response(&pool, &event, &response_capability).await;

    let requests = [
        matrix_input(EVENT_PUBLIC_ID, &wrong_capability),
        matrix_input(EVENT_PUBLIC_ID, &other_capability),
        matrix_input("0c3d7820-7542-4f89-ad38-0ef5c1c18a93", &raw_capability),
    ];
    let mut public_messages = Vec::new();

    for request in requests {
        let error = persist_organizer_response_matrix(&pool, request)
            .await
            .expect_err("unauthorized reads must not return a private matrix");
        let display = error.to_string();
        let combined = format!("{display}\n{error:?}");
        for forbidden in [
            raw_capability.as_str(),
            other_capability.as_str(),
            wrong_capability.as_str(),
            response_capability.as_str(),
            PRIVATE_NAME,
            PRIVATE_COMMENT,
            "private-event-sentinel",
            "other-private-event-sentinel",
        ] {
            assert!(
                !combined.contains(forbidden),
                "matrix authorization errors disclosed {forbidden}: {combined}"
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
async fn authorized_json_contains_only_the_minimal_matrix_projection() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = capability("ab");
    let response_capability = capability("11");
    let organizer_capability_hash = capability_hash(&raw_capability);
    let response_capability_hash = capability_hash(&response_capability);
    let event = create_event(&pool, EVENT_PUBLIC_ID, &raw_capability, "秋の餃子会").await;
    add_private_response(&pool, &event, &response_capability).await;

    let matrix =
        persist_organizer_response_matrix(&pool, matrix_input(EVENT_PUBLIC_ID, &raw_capability))
            .await
            .expect("valid organizer authority should return the private matrix");
    let json = serde_json::to_string(&matrix).expect("serialize matrix response");

    assert!(
        json.contains("秋の餃子会")
            && json.contains(PRIVATE_NAME)
            && json.contains("available")
            && json.contains("maybe"),
        "the fixture should prove this is the complete authorized matrix: {json}"
    );
    for forbidden in [
        raw_capability.as_str(),
        organizer_capability_hash.as_str(),
        response_capability.as_str(),
        response_capability_hash.as_str(),
        PRIVATE_COMMENT,
        "表へは含めない主催者メモ",
        "organizer_capability",
        "organizer_capability_hash",
        "response_capability",
        "response_capability_hash",
        "public_id",
        "candidate_id",
        "response_id",
        "comment",
        "count",
        "fact",
        "decision",
    ] {
        assert!(
            !json.contains(forbidden),
            "matrix leaked {forbidden}: {json}"
        );
    }
}
