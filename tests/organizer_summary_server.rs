#![cfg(feature = "server")]

use dioxus::prelude::ServerFnError;
use sha2::{Digest, Sha256};
use std::future::Future;
use tsunoru::{
    domain::{
        Availability, CandidateAvailabilityInput, CandidateInput, NewAvailabilityResponseInput,
        NewEventInput, NewResponseCommentInput, OrganizerEventSummary, OrganizerSummaryInput,
        PreparedAvailabilityResponse,
    },
    server::{
        get_organizer_event_summary, persist_availability_response,
        persist_organizer_event_summary, persist_response_comment,
    },
    storage::{create_event_record, open_in_memory},
};

const EVENT_PUBLIC_ID: &str = "7af78527-813b-4cdd-a632-058f3ce885aa";
const OTHER_EVENT_PUBLIC_ID: &str = "5d70514a-575f-4079-9be9-5bca4563f84c";
const PRIVATE_NAME: &str = "private-name-sentinel";
const PRIVATE_COMMENT: &str = "private-comment-sentinel";

fn event_input(name: &str) -> NewEventInput {
    NewEventInput {
        name: name.to_owned(),
        organizer_note: Some("みんなで集まりたいです".to_owned()),
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

fn capability(byte_pair: &str) -> String {
    assert_eq!(byte_pair.len(), 2);
    byte_pair.repeat(32)
}

fn capability_hash(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}

fn summary_input(event_public_id: &str, organizer_capability: &str) -> OrganizerSummaryInput {
    OrganizerSummaryInput {
        event_public_id: event_public_id.to_owned(),
        organizer_capability: organizer_capability.to_owned(),
    }
}

fn require_http_error_contract<F>(_: F)
where
    F: Future<Output = std::result::Result<OrganizerEventSummary, ServerFnError>>,
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
        &event_input(name),
    )
    .await
    .expect("persist fixture event")
}

async fn add_private_response(
    pool: &sqlx::SqlitePool,
    event: &tsunoru::domain::PublicEvent,
    raw_response_capability: &str,
) {
    persist_availability_response(
        pool,
        NewAvailabilityResponseInput {
            event_public_id: event.public_id.clone(),
            response_capability: raw_response_capability.to_owned(),
            response: PreparedAvailabilityResponse {
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
        },
    )
    .await
    .expect("persist fixture response");

    persist_response_comment(
        pool,
        NewResponseCommentInput {
            event_public_id: event.public_id.clone(),
            response_capability: raw_response_capability.to_owned(),
            comment: PRIVATE_COMMENT.to_owned(),
        },
    )
    .await
    .expect("persist fixture comment");
}

#[test]
fn organizer_summary_input_normalizes_only_well_shaped_authority() {
    let raw_capability = capability("ab");
    let normalized = summary_input(
        &format!("  {EVENT_PUBLIC_ID}  "),
        &format!("  {raw_capability}  "),
    )
    .normalized_and_validated()
    .expect("a copied organizer capability may have surrounding whitespace");

    assert_eq!(normalized.event_public_id, EVENT_PUBLIC_ID);
    assert_eq!(normalized.organizer_capability, raw_capability);

    for invalid in [
        summary_input("../other-event", &capability("ab")),
        summary_input(&"a".repeat(65), &capability("ab")),
        summary_input(EVENT_PUBLIC_ID, &"a".repeat(63)),
        summary_input(EVENT_PUBLIC_ID, &capability("AB")),
        summary_input(EVENT_PUBLIC_ID, &capability("gg")),
    ] {
        assert!(
            invalid.normalized_and_validated().is_err(),
            "path-like public ids and non-64-character lowercase hexadecimal capabilities must not cross the server boundary"
        );
    }
}

#[test]
fn organizer_capability_is_redacted_from_debug_but_serialized_in_the_post_body() {
    let raw_capability = capability("cd");
    let input = summary_input(EVENT_PUBLIC_ID, &raw_capability);
    let debug = format!("{input:?}");

    assert!(debug.contains("OrganizerSummaryInput"));
    assert!(debug.contains("organizer_capability"));
    assert!(
        !debug.contains(&raw_capability),
        "routine debug output must redact organizer authority: {debug}"
    );

    let json = serde_json::to_value(&input).expect("serialize the typed POST body");
    assert_eq!(json["event_public_id"], EVENT_PUBLIC_ID);
    assert_eq!(json["organizer_capability"], raw_capability);
    assert!(json.get("organizer_capability_hash").is_none());
}

#[test]
fn organizer_summary_is_a_post_server_function_with_explicit_http_errors() {
    let input = summary_input(EVENT_PUBLIC_ID, &capability("ab"));
    require_http_error_contract(get_organizer_event_summary(input));

    let server_source = include_str!("../src/server.rs");
    assert!(
        server_source.contains("#[post(\"/api/organizer/events/summary\")]"),
        "private organizer data must cross an explicit POST body rather than a GET query"
    );
}

#[tokio::test]
async fn wrong_cross_event_and_missing_event_authority_share_one_non_exposing_error() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = capability("ab");
    let other_capability = capability("cd");
    let wrong_capability = capability("ef");
    let raw_response_capability = capability("11");
    let raw_capability_hash = capability_hash(&raw_capability);
    let raw_response_capability_hash = capability_hash(&raw_response_capability);
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
    add_private_response(&pool, &event, &raw_response_capability).await;

    let requests = [
        summary_input(EVENT_PUBLIC_ID, &wrong_capability),
        summary_input(EVENT_PUBLIC_ID, &other_capability),
        summary_input("0c3d7820-7542-4f89-ad38-0ef5c1c18a93", &raw_capability),
    ];
    let mut public_messages = Vec::new();

    for request in requests {
        let error = persist_organizer_event_summary(&pool, request)
            .await
            .expect_err("unauthorized reads must not return any private projection");
        let display = error.to_string();
        let debug = format!("{error:?}");
        let combined = format!("{display}\n{debug}");

        for forbidden in [
            raw_capability.as_str(),
            other_capability.as_str(),
            wrong_capability.as_str(),
            raw_response_capability.as_str(),
            raw_capability_hash.as_str(),
            raw_response_capability_hash.as_str(),
            PRIVATE_NAME,
            PRIVATE_COMMENT,
            "private-event-sentinel",
            "other-private-event-sentinel",
        ] {
            assert!(
                !combined.contains(forbidden),
                "not-found errors must not disclose organizer authority or private event data: {combined}"
            );
        }
        public_messages.push(display);
    }

    assert!(
        public_messages.windows(2).all(|pair| pair[0] == pair[1]),
        "wrong, cross-event, and missing-event authority must be indistinguishable: {public_messages:?}"
    );
}

#[tokio::test]
async fn authorized_summary_json_excludes_every_authority_and_internal_response_identifier() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = capability("ab");
    let raw_response_capability = capability("11");
    let raw_capability_hash = capability_hash(&raw_capability);
    let raw_response_capability_hash = capability_hash(&raw_response_capability);
    let event = create_event(&pool, EVENT_PUBLIC_ID, &raw_capability, "秋の餃子会").await;
    add_private_response(&pool, &event, &raw_response_capability).await;

    let summary =
        persist_organizer_event_summary(&pool, summary_input(EVENT_PUBLIC_ID, &raw_capability))
            .await
            .expect("valid organizer authority should return the private projection");
    let json = serde_json::to_string(&summary).expect("serialize the server-function response");

    assert!(
        json.contains("秋の餃子会")
            && json.contains(PRIVATE_NAME)
            && json.contains(PRIVATE_COMMENT),
        "the fixture must prove this is the authorized private projection: {json}"
    );
    for forbidden in [
        raw_capability.as_str(),
        raw_capability_hash.as_str(),
        raw_response_capability.as_str(),
        raw_response_capability_hash.as_str(),
        "organizer_capability",
        "organizer_capability_hash",
        "response_capability",
        "response_capability_hash",
        "response_id",
    ] {
        assert!(
            !json.contains(forbidden),
            "the organizer response must not echo credentials, hashes, or internal response ids: {json}"
        );
    }
}
