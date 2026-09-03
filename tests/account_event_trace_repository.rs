#![cfg(feature = "server")]

use sha2::{Digest, Sha256};
use tsunoru::{
    auth::hash_session_token,
    domain::{
        AccountEventTraceRelationship, Availability, CandidateAvailabilityInput, CandidateInput,
        NewEventInput, PreparedAvailabilityResponse,
    },
    storage::{
        AccountEventTraceStorageError, create_account_with_session,
        create_event_record_for_session, find_account_event_trace_by_session, open_in_memory,
        record_availability_response_for_session,
    },
};

const NOW: i64 = 1_800_000_000;

async fn account_session(
    pool: &sqlx::SqlitePool,
    login_id: &str,
    raw_token: &str,
) -> (i64, [u8; 32]) {
    let token_hash = hash_session_token(raw_token).expect("test session token shape");
    let account = create_account_with_session(
        pool,
        login_id,
        "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA",
        &token_hash,
        NOW,
    )
    .await
    .expect("create account fixture");
    (account.id, token_hash)
}

fn event_input() -> NewEventInput {
    NewEventInput {
        name: "秋の餃子会".to_owned(),
        organizer_note: Some("いつもの店で".to_owned()),
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![
            CandidateInput {
                local_date: "2027-01-15".to_owned(),
                local_time: "19:00".to_owned(),
            },
            CandidateInput {
                local_date: "2027-01-16".to_owned(),
                local_time: "20:00".to_owned(),
            },
        ],
    }
}

fn response(
    candidate_ids: [i64; 2],
    respondent_name: &str,
    availabilities: [Availability; 2],
) -> PreparedAvailabilityResponse {
    PreparedAvailabilityResponse {
        respondent_name: respondent_name.to_owned(),
        availabilities: candidate_ids
            .into_iter()
            .zip(availabilities)
            .map(|(candidate_id, availability)| CandidateAvailabilityInput {
                candidate_id,
                availability,
            })
            .collect(),
    }
}

fn response_capability_hash(label: &str) -> String {
    format!("{:x}", Sha256::digest(label.as_bytes()))
}

async fn set_comment(pool: &sqlx::SqlitePool, capability_hash: &str, comment: &str) {
    sqlx::query("UPDATE responses SET respondent_comment = ? WHERE response_capability_hash = ?")
        .bind(comment)
        .bind(capability_hash)
        .execute(pool)
        .await
        .expect("add fixture comment");
}

#[tokio::test]
async fn organizer_and_participant_receive_different_views_of_one_snapshot() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (_, organizer_session) = account_session(&pool, "organizer", &"1".repeat(64)).await;
    let (_, participant_session) = account_session(&pool, "participant", &"2".repeat(64)).await;
    let (_, other_session) = account_session(&pool, "other", &"3".repeat(64)).await;

    let event = create_event_record_for_session(
        &pool,
        "trace-event",
        &"a".repeat(64),
        &event_input(),
        Some(&organizer_session),
        NOW,
    )
    .await
    .unwrap()
    .value;
    let candidate_ids = [event.candidates[0].id, event.candidates[1].id];

    for (label, session, name, answers, comment) in [
        (
            "participant-answer",
            &participant_session,
            "ミナ",
            [Availability::Available, Availability::Maybe],
            "楽しみです",
        ),
        (
            "other-answer",
            &other_session,
            "ソラ",
            [Availability::Unavailable, Availability::Available],
            "二人で行きます",
        ),
    ] {
        let capability_hash = response_capability_hash(label);
        record_availability_response_for_session(
            &pool,
            &event.public_id,
            &capability_hash,
            &response(candidate_ids, name, answers),
            Some(session),
            NOW + 1,
        )
        .await
        .unwrap();
        set_comment(&pool, &capability_hash, comment).await;
    }
    sqlx::query(
        "INSERT INTO event_decisions (event_public_id, candidate_id, decided_at) VALUES (?, ?, ?)",
    )
    .bind(&event.public_id)
    .bind(candidate_ids[0])
    .bind(NOW + 2)
    .execute(&pool)
    .await
    .unwrap();

    let organizer =
        find_account_event_trace_by_session(&pool, &organizer_session, &event.public_id, NOW + 10)
            .await
            .expect("organizer reads all event traces");
    assert_eq!(
        organizer.relationship,
        AccountEventTraceRelationship::Organized
    );
    assert_eq!(organizer.organizer_note.as_deref(), Some("いつもの店で"));
    assert_eq!(organizer.candidates.len(), 2);
    assert_eq!(
        organizer.decision.as_ref().unwrap().local_date,
        "2027-01-15"
    );
    assert_eq!(
        organizer
            .responses
            .iter()
            .map(|response| response.respondent_name.as_str())
            .collect::<Vec<_>>(),
        vec!["ミナ", "ソラ"]
    );
    assert!(
        organizer
            .responses
            .iter()
            .all(|response| !response.is_current_account)
    );

    let participant = find_account_event_trace_by_session(
        &pool,
        &participant_session,
        &event.public_id,
        NOW + 10,
    )
    .await
    .expect("participant reads only directly linked responses");
    assert_eq!(
        participant.relationship,
        AccountEventTraceRelationship::Participated
    );
    assert_eq!(participant.responses.len(), 1);
    assert_eq!(participant.responses[0].respondent_name, "ミナ");
    assert_eq!(
        participant.responses[0].comment.as_deref(),
        Some("楽しみです")
    );
    assert!(participant.responses[0].is_current_account);
    assert!(!format!("{participant:?}").contains("ソラ"));
    assert!(!format!("{participant:?}").contains("二人で行きます"));
}

#[tokio::test]
async fn both_role_and_multiple_own_responses_are_preserved_without_deduplication() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (_, session) = account_session(&pool, "both", &"4".repeat(64)).await;
    let event = create_event_record_for_session(
        &pool,
        "both-role-event",
        &"b".repeat(64),
        &event_input(),
        Some(&session),
        NOW,
    )
    .await
    .unwrap()
    .value;
    let candidate_ids = [event.candidates[0].id, event.candidates[1].id];

    for label in ["first-own", "second-own"] {
        record_availability_response_for_session(
            &pool,
            &event.public_id,
            &response_capability_hash(label),
            &response(
                candidate_ids,
                "同じ表示名",
                [Availability::Available, Availability::Maybe],
            ),
            Some(&session),
            NOW + 1,
        )
        .await
        .unwrap();
    }

    let trace = find_account_event_trace_by_session(&pool, &session, &event.public_id, NOW + 10)
        .await
        .expect("read both-role trace");
    assert_eq!(
        trace.relationship,
        AccountEventTraceRelationship::OrganizedAndParticipated
    );
    assert_eq!(trace.responses.len(), 2);
    assert!(
        trace
            .responses
            .iter()
            .all(|response| response.is_current_account)
    );
}

#[tokio::test]
async fn both_role_keeps_all_scopes_and_maps_cells_by_authored_candidate_position() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (_, both_session) = account_session(&pool, "both-scopes", &"7".repeat(64)).await;
    let (_, other_session) = account_session(&pool, "other-scope", &"8".repeat(64)).await;
    let event = create_event_record_for_session(
        &pool,
        "both-scope-event",
        &"d".repeat(64),
        &event_input(),
        Some(&both_session),
        NOW,
    )
    .await
    .unwrap()
    .value;
    let candidate_ids = [event.candidates[0].id, event.candidates[1].id];

    for (label, session, name, answers) in [
        (
            "both-own",
            Some(&both_session),
            "本人",
            [Availability::Available, Availability::Maybe],
        ),
        (
            "both-other",
            Some(&other_session),
            "別account",
            [Availability::Unavailable, Availability::Available],
        ),
        (
            "both-anonymous",
            None,
            "anonymous",
            [Availability::Maybe, Availability::Unavailable],
        ),
    ] {
        record_availability_response_for_session(
            &pool,
            &event.public_id,
            &response_capability_hash(label),
            &response(candidate_ids, name, answers),
            session,
            NOW + 1,
        )
        .await
        .unwrap();
    }

    sqlx::query("UPDATE candidates SET position = 99 WHERE id = ?")
        .bind(candidate_ids[0])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE candidates SET position = 0 WHERE id = ?")
        .bind(candidate_ids[1])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE candidates SET position = 1 WHERE id = ?")
        .bind(candidate_ids[0])
        .execute(&pool)
        .await
        .unwrap();

    let trace =
        find_account_event_trace_by_session(&pool, &both_session, &event.public_id, NOW + 10)
            .await
            .expect("both role sees all event responses once");
    assert_eq!(
        trace.relationship,
        AccountEventTraceRelationship::OrganizedAndParticipated
    );
    assert_eq!(
        trace
            .candidates
            .iter()
            .map(|candidate| candidate.local_date.as_str())
            .collect::<Vec<_>>(),
        vec!["2027-01-16", "2027-01-15"]
    );
    assert_eq!(
        trace
            .responses
            .iter()
            .map(|response| (
                response.respondent_name.as_str(),
                response.availabilities.clone(),
                response.is_current_account,
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "本人",
                vec![Availability::Maybe, Availability::Available],
                true,
            ),
            (
                "別account",
                vec![Availability::Available, Availability::Unavailable],
                false,
            ),
            (
                "anonymous",
                vec![Availability::Unavailable, Availability::Maybe],
                false,
            ),
        ]
    );
}

#[tokio::test]
async fn missing_and_unrelated_events_share_not_found_while_invalid_cells_fail_closed() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (_, owner_session) = account_session(&pool, "owner", &"5".repeat(64)).await;
    let (_, stranger_session) = account_session(&pool, "stranger", &"6".repeat(64)).await;
    let event = create_event_record_for_session(
        &pool,
        "private-trace-event",
        &"c".repeat(64),
        &event_input(),
        Some(&owner_session),
        NOW,
    )
    .await
    .unwrap()
    .value;

    for public_id in [&event.public_id, "does-not-exist"] {
        assert!(matches!(
            find_account_event_trace_by_session(&pool, &stranger_session, public_id, NOW + 10)
                .await,
            Err(AccountEventTraceStorageError::NotFound)
        ));
    }

    let capability_hash = response_capability_hash("broken-cell");
    record_availability_response_for_session(
        &pool,
        &event.public_id,
        &capability_hash,
        &response(
            [event.candidates[0].id, event.candidates[1].id],
            "欠損回答",
            [Availability::Available, Availability::Maybe],
        ),
        Some(&owner_session),
        NOW + 1,
    )
    .await
    .unwrap();
    sqlx::query(
        "DELETE FROM response_availabilities WHERE response_id = (SELECT id FROM responses WHERE response_capability_hash = ?) AND candidate_id = ?",
    )
    .bind(&capability_hash)
    .bind(event.candidates[1].id)
    .execute(&pool)
    .await
    .unwrap();

    assert!(matches!(
        find_account_event_trace_by_session(&pool, &owner_session, &event.public_id, NOW + 10)
            .await,
        Err(AccountEventTraceStorageError::DataInvariantViolation)
    ));
}

#[tokio::test]
async fn organizer_trace_rejects_an_event_cell_with_an_unknown_response() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let (_, owner_session) = account_session(&pool, "orphan-owner", &"9".repeat(64)).await;
    let event = create_event_record_for_session(
        &pool,
        "orphan-trace-event",
        &"e".repeat(64),
        &event_input(),
        Some(&owner_session),
        NOW,
    )
    .await
    .unwrap()
    .value;

    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"
        INSERT INTO response_availabilities (
            response_id, candidate_id, event_public_id, availability
        ) VALUES (?, ?, ?, 'available')
        "#,
    )
    .bind(999_999_i64)
    .bind(event.candidates[0].id)
    .bind(&event.public_id)
    .execute(&pool)
    .await
    .expect("seed one orphan cell with foreign keys disabled");
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .unwrap();

    assert!(matches!(
        find_account_event_trace_by_session(&pool, &owner_session, &event.public_id, NOW + 10)
            .await,
        Err(AccountEventTraceStorageError::DataInvariantViolation)
    ));
}
