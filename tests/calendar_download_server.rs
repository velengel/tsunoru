#![cfg(feature = "server")]

use dioxus::server::axum::{
    body::to_bytes,
    http::{StatusCode, header},
};
use tsunoru::{
    domain::{CandidateInput, NewEventInput},
    server::public_calendar_download_response,
    storage::{create_event_record, open_in_memory, record_event_decision},
};

const DECIDED_ID: &str = "7af78527-813b-4cdd-a632-058f3ce885aa";
const UNDECIDED_ID: &str = "ca90ec98-20a6-4e54-b0a8-fb820175b5a4";
const GENERATED_AT: &str = "20260902T123456Z";

fn event_input(name: &str) -> NewEventInput {
    NewEventInput {
        name: name.to_owned(),
        organizer_note: None,
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![CandidateInput {
            local_date: "2026-09-18".to_owned(),
            local_time: "19:00".to_owned(),
        }],
    }
}

async fn response_body(response: dioxus::server::axum::response::Response) -> String {
    String::from_utf8(
        to_bytes(response.into_body(), 128 * 1024)
            .await
            .expect("read bounded response body")
            .to_vec(),
    )
    .expect("response body should be UTF-8")
}

#[tokio::test]
async fn decided_download_returns_a_raw_calendar_with_safe_headers() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = "a".repeat(64);
    let event = create_event_record(&pool, DECIDED_ID, &organizer_hash, &event_input("餃子会"))
        .await
        .expect("create event");
    record_event_decision(
        &pool,
        &event.public_id,
        &organizer_hash,
        event.candidates[0].id,
    )
    .await
    .expect("decide event");

    let response = public_calendar_download_response(&pool, DECIDED_ID, GENERATED_AT).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/calendar; charset=utf-8"
    );
    assert_eq!(
        response.headers().get(header::CONTENT_DISPOSITION).unwrap(),
        "attachment; filename=\"tsunoru-7af78527-813b-4cdd-a632-058f3ce885aa.ics\""
    );
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-store"
    );
    assert_eq!(
        response
            .headers()
            .get(header::X_CONTENT_TYPE_OPTIONS)
            .unwrap(),
        "nosniff"
    );
    let body = response_body(response).await;
    assert!(body.starts_with("BEGIN:VCALENDAR\r\n"));
    assert!(body.contains("SUMMARY:餃子会"));
    assert!(body.ends_with("END:VCALENDAR\r\n"));
}

#[tokio::test]
async fn undecided_missing_and_invalid_ids_never_return_a_partial_calendar() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    create_event_record(&pool, UNDECIDED_ID, &"b".repeat(64), &event_input("未決定"))
        .await
        .expect("create undecided event");

    for (public_id, expected_status) in [
        (UNDECIDED_ID, StatusCode::CONFLICT),
        (
            "fdd1b759-f792-45ba-b351-10cbf42316f5",
            StatusCode::NOT_FOUND,
        ),
        ("../private", StatusCode::NOT_FOUND),
    ] {
        let response = public_calendar_download_response(&pool, public_id, GENERATED_AT).await;
        assert_eq!(response.status(), expected_status, "public_id={public_id}");
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        assert_eq!(
            response
                .headers()
                .get(header::X_CONTENT_TYPE_OPTIONS)
                .unwrap(),
            "nosniff"
        );
        assert!(
            response
                .headers()
                .get(header::CONTENT_DISPOSITION)
                .is_none(),
            "errors must not look downloadable: public_id={public_id}"
        );
        let body = response_body(response).await;
        assert!(!body.contains("BEGIN:VCALENDAR"), "public_id={public_id}");
    }
}

#[tokio::test]
async fn corrupt_decision_returns_a_generic_non_calendar_error() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = "c".repeat(64);
    let event = create_event_record(&pool, DECIDED_ID, &organizer_hash, &event_input("壊す予定"))
        .await
        .expect("create event");
    let candidate_id = event.candidates[0].id;
    record_event_decision(&pool, DECIDED_ID, &organizer_hash, candidate_id)
        .await
        .expect("decide event");
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .expect("disable constraints for corrupt fixture");
    sqlx::query("DELETE FROM candidates WHERE id = ?")
        .bind(candidate_id)
        .execute(&pool)
        .await
        .expect("remove selected candidate");
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("restore constraints");

    let response = public_calendar_download_response(&pool, DECIDED_ID, GENERATED_AT).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .is_none()
    );
    let body = response_body(response).await;
    assert!(!body.contains("壊す予定") && !body.contains("BEGIN:VCALENDAR"));
}

#[test]
fn the_fullstack_server_registers_calendar_as_a_non_html_route() {
    let main_source = include_str!("../src/main.rs");
    let server_source = include_str!("../src/server.rs");
    assert!(
        main_source.contains("dioxus::serve")
            && main_source.contains("dioxus::server::router(App)")
            && main_source.contains("/api/events/{public_id}/calendar.ics")
            && main_source.contains("download_public_calendar"),
        "the raw calendar route must take priority over the SSR fallback: {main_source}"
    );
    assert!(
        server_source.contains("Result<Path<String>, PathRejection>")
            && server_source.contains("calendar_not_found_response"),
        "path extraction failures must use the same 404 and safe headers as invalid UUIDs: {server_source}"
    );
}
