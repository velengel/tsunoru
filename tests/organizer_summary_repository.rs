#![cfg(feature = "server")]

use sha2::{Digest, Sha256};
use tsunoru::{
    domain::{
        Availability, CandidateAvailabilityInput, CandidateInput, CandidateResponseSummary,
        CandidateSummaryFact, NewEventInput, OrganizerEventSummary, PreparedAvailabilityResponse,
    },
    storage::{
        OrganizerSummaryStorageError, create_event_record, find_organizer_event_summary,
        find_public_event, open_in_memory, record_availability_response, record_response_comment,
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
async fn zero_responses_returns_every_candidate_in_authored_order_with_zero_counts() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = capability_hash("organizer-zero");
    let event = create_event(&pool, "summary-zero", &organizer_hash).await;

    let summary: OrganizerEventSummary =
        find_organizer_event_summary(&pool, &event.public_id, &organizer_hash)
            .await
            .expect("load organizer summary");

    assert_eq!(summary.public_id, event.public_id);
    assert_eq!(summary.name, "秋の餃子会");
    assert_eq!(
        summary.organizer_note.as_deref(),
        Some("駅の近くで集まりたいです")
    );
    assert_eq!(summary.time_zone, "Asia/Tokyo");
    assert_eq!(summary.response_count, 0);
    assert_eq!(summary.comment_count, 0);
    assert!(summary.comment_previews.is_empty());
    assert_eq!(summary.candidates.len(), 3);

    let _: &CandidateResponseSummary = &summary.candidates[0];
    assert_eq!(
        summary
            .candidates
            .iter()
            .map(|candidate| (
                candidate.local_date.as_str(),
                candidate.local_time.as_str(),
                candidate.available_count,
                candidate.maybe_count,
                candidate.unavailable_count,
            ))
            .collect::<Vec<_>>(),
        vec![
            ("2026-09-20", "14:00", 0, 0, 0),
            ("2026-09-18", "19:00", 0, 0, 0),
            ("2026-09-21", "12:30", 0, 0, 0),
        ]
    );
    assert!(
        summary
            .candidates
            .iter()
            .all(|candidate| candidate.fact.is_none()),
        "an empty response set must not be described as unanimous"
    );
}

#[tokio::test]
async fn counts_all_three_values_and_treats_equal_display_names_as_distinct_responses() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = capability_hash("organizer-counts");
    let event = create_event(&pool, "summary-counts", &organizer_hash).await;

    add_response(
        &pool,
        &event,
        "response-one",
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
        "response-two",
        "ミナ",
        [
            Availability::Available,
            Availability::Available,
            Availability::Maybe,
        ],
        None,
    )
    .await;
    add_response(
        &pool,
        &event,
        "response-three",
        "ソラ",
        [
            Availability::Maybe,
            Availability::Unavailable,
            Availability::Available,
        ],
        None,
    )
    .await;

    let summary = find_organizer_event_summary(&pool, &event.public_id, &organizer_hash)
        .await
        .expect("load organizer summary");

    assert_eq!(summary.response_count, 3, "a display name is not identity");
    assert_eq!(
        summary
            .candidates
            .iter()
            .map(|candidate| (
                candidate.id,
                candidate.available_count,
                candidate.maybe_count,
                candidate.unavailable_count,
            ))
            .collect::<Vec<_>>(),
        vec![
            (event.candidates[0].id, 2, 1, 0),
            (event.candidates[1].id, 1, 1, 1),
            (event.candidates[2].id, 1, 1, 1),
        ],
        "candidate order and the exact tri-state counts must be preserved"
    );
    assert_eq!(
        summary.candidates[0].fact.as_ref(),
        Some(&CandidateSummaryFact::EveryoneAvailableIncludingMaybe)
    );
    assert_eq!(
        summary.candidates[1].fact.as_ref(),
        Some(&CandidateSummaryFact::OneUnavailable)
    );
    assert_eq!(
        summary.candidates[2].fact.as_ref(),
        Some(&CandidateSummaryFact::OneUnavailable)
    );
}

#[tokio::test]
async fn returns_the_total_comment_count_but_only_three_deterministic_previews() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = capability_hash("organizer-comments");
    let event = create_event(&pool, "summary-comments", &organizer_hash).await;
    let fixtures = [
        ("comment-one", "一人目", "最初のコメント"),
        ("comment-two", "二人目", "二番目のコメント"),
        ("comment-three", "三人目", "三番目のコメント"),
        ("comment-four", "四人目", "四番目のコメント"),
        (
            "comment-five",
            "五人目",
            "<strong>plain textのまま返す</strong>",
        ),
    ];

    for (capability_seed, respondent_name, comment) in fixtures {
        add_response(
            &pool,
            &event,
            capability_seed,
            respondent_name,
            [
                Availability::Available,
                Availability::Maybe,
                Availability::Unavailable,
            ],
            Some(comment),
        )
        .await;
    }

    let summary = find_organizer_event_summary(&pool, &event.public_id, &organizer_hash)
        .await
        .expect("load organizer summary");

    assert_eq!(summary.response_count, 5);
    assert_eq!(summary.comment_count, 5);
    assert_eq!(summary.comment_previews.len(), 3);
    assert_eq!(
        summary
            .comment_previews
            .iter()
            .map(|preview| (preview.respondent_name.as_str(), preview.comment.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("五人目", "<strong>plain textのまま返す</strong>"),
            ("四人目", "四番目のコメント"),
            ("三人目", "三番目のコメント"),
        ],
        "preview selection follows descending response id, not an invented comment timestamp"
    );
}

#[tokio::test]
async fn wrong_missing_and_cross_event_hashes_are_indistinguishable_not_found() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let first_hash = capability_hash("organizer-first");
    let other_hash = capability_hash("organizer-other");
    let first = create_event(&pool, "summary-private", &first_hash).await;
    create_event(&pool, "summary-private-other", &other_hash).await;

    for (event_public_id, presented_hash) in [
        (first.public_id.as_str(), capability_hash("wrong-secret")),
        (first.public_id.as_str(), other_hash),
        ("missing-summary", first_hash),
    ] {
        assert!(matches!(
            find_organizer_event_summary(&pool, event_public_id, &presented_hash).await,
            Err(OrganizerSummaryStorageError::NotFound)
        ));
    }
}

#[tokio::test]
async fn public_event_projection_does_not_expose_summary_or_comment_data() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = capability_hash("organizer-public-boundary");
    let event = create_event(&pool, "summary-public-boundary", &organizer_hash).await;
    add_response(
        &pool,
        &event,
        "private-response",
        "公開してはいけない回答者名",
        [
            Availability::Available,
            Availability::Maybe,
            Availability::Unavailable,
        ],
        Some("公開してはいけないコメント"),
    )
    .await;

    let public_event = find_public_event(&pool, &event.public_id)
        .await
        .expect("load public event")
        .expect("fixture event exists");
    let public_json = serde_json::to_string(&public_event).expect("serialize public projection");

    for private_fragment in [
        "response_count",
        "available_count",
        "comment_count",
        "comment_previews",
        "公開してはいけない回答者名",
        "公開してはいけないコメント",
    ] {
        assert!(
            !public_json.contains(private_fragment),
            "public projection leaked {private_fragment}"
        );
    }
}

#[tokio::test]
async fn incomplete_response_aggregate_is_rejected_as_an_invariant_violation() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = capability_hash("organizer-invariant");
    let event = create_event(&pool, "summary-invariant", &organizer_hash).await;
    add_response(
        &pool,
        &event,
        "incomplete-response",
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
        find_organizer_event_summary(&pool, &event.public_id, &organizer_hash).await,
        Err(OrganizerSummaryStorageError::DataInvariantViolation)
    ));
}
