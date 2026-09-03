#![cfg(feature = "server")]

use tsunoru::{
    domain::{CandidateInput, NewEventInput},
    server::persist_created_event,
    storage::{create_event_record, find_public_event, open_in_memory},
};

fn new_event(candidates: Vec<CandidateInput>) -> NewEventInput {
    NewEventInput {
        name: "秋の餃子会".to_owned(),
        organizer_note: None,
        time_zone: "Asia/Tokyo".to_owned(),
        candidates,
    }
}

fn candidate(date: &str, time: &str) -> CandidateInput {
    CandidateInput {
        local_date: date.to_owned(),
        local_time: time.to_owned(),
    }
}

#[tokio::test]
async fn event_and_candidates_round_trip_in_the_authored_order() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let input = new_event(vec![
        candidate("2026-09-20", "14:00"),
        candidate("2026-09-18", "19:00"),
    ]);

    let created = create_event_record(
        &pool,
        "public-event-one",
        "hashed-organizer-capability",
        &input,
    )
    .await
    .expect("persist an event and all candidates");

    assert_eq!(
        created
            .candidates
            .iter()
            .map(|item| (item.local_date.as_str(), item.local_time.as_str()))
            .collect::<Vec<_>>(),
        vec![("2026-09-20", "14:00"), ("2026-09-18", "19:00")],
        "the committed aggregate should be returned without a post-commit reload"
    );

    let loaded = find_public_event(&pool, "public-event-one")
        .await
        .expect("query the event")
        .expect("event should exist");

    assert_eq!(loaded.name, "秋の餃子会");
    assert_eq!(loaded.organizer_note, None);
    assert_eq!(loaded.time_zone, "Asia/Tokyo");
    assert_eq!(
        loaded
            .candidates
            .iter()
            .map(|item| (item.local_date.as_str(), item.local_time.as_str()))
            .collect::<Vec<_>>(),
        vec![("2026-09-20", "14:00"), ("2026-09-18", "19:00")]
    );
}

#[tokio::test]
async fn candidate_failure_rolls_back_the_whole_event() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let duplicated = candidate("2026-09-18", "19:00");

    let result = create_event_record(
        &pool,
        "public-event-two",
        "hashed-organizer-capability",
        &new_event(vec![duplicated.clone(), duplicated]),
    )
    .await;

    assert!(result.is_err(), "the database should reject duplicates");
    assert_eq!(
        find_public_event(&pool, "public-event-two")
            .await
            .expect("query after rollback"),
        None,
        "the parent event must not survive a failed candidate insert"
    );
}

#[tokio::test]
async fn raw_organizer_capability_is_never_persisted() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let raw_capability = "raw-organizer-capability-for-test";

    let created = persist_created_event(
        &pool,
        new_event(vec![candidate("2026-09-18", "19:00")]),
        "public-event-three".to_owned(),
        raw_capability.to_owned(),
    )
    .await
    .expect("service should hash and persist an event");

    let stored: String =
        sqlx::query_scalar("SELECT organizer_capability_hash FROM events WHERE public_id = ?")
            .bind("public-event-three")
            .fetch_one(&pool)
            .await
            .expect("read the stored capability hash");

    use sha2::{Digest, Sha256};
    let expected_hash = format!("{:x}", Sha256::digest(raw_capability.as_bytes()));
    assert_eq!(created.organizer_capability, raw_capability);
    assert_eq!(stored, expected_hash);
    assert_ne!(stored, raw_capability);
}
