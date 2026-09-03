#![cfg(feature = "server")]

use sha2::{Digest, Sha256};
use sqlx::{AssertSqlSafe, Row};
use std::path::{Path, PathBuf};
use tsunoru::{
    domain::{CandidateInput, NewEventInput, OrganizerEventDecision, PublicEvent},
    storage::{
        EventDecisionWriteOutcome, OrganizerDecisionStorageError, create_event_record,
        find_organizer_event_summary, open_file, open_in_memory, record_event_decision,
    },
};

fn event_input(name: &str) -> NewEventInput {
    NewEventInput {
        name: name.to_owned(),
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

fn capability_hash(seed: &str) -> String {
    format!("{:x}", Sha256::digest(seed.as_bytes()))
}

async fn create_event(
    pool: &sqlx::SqlitePool,
    public_id: &str,
    organizer_capability_hash: &str,
    name: &str,
) -> PublicEvent {
    create_event_record(
        pool,
        public_id,
        organizer_capability_hash,
        &event_input(name),
    )
    .await
    .expect("persist fixture event")
}

fn expected_decision(event: &PublicEvent, candidate_position: usize) -> OrganizerEventDecision {
    let candidate = &event.candidates[candidate_position];
    OrganizerEventDecision {
        candidate_id: candidate.id,
        local_date: candidate.local_date.clone(),
        local_time: candidate.local_time.clone(),
    }
}

fn temporary_database_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("tsunoru-{label}-{}.sqlite3", uuid::Uuid::new_v4()))
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
async fn authorized_organizer_can_decide_an_owned_candidate_without_any_responses() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = capability_hash("decision-zero-responses");
    let event = create_event(
        &pool,
        "decision-zero-responses",
        &organizer_hash,
        "回答前に決める会",
    )
    .await;
    let response_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM responses WHERE event_public_id = ?")
            .bind(&event.public_id)
            .fetch_one(&pool)
            .await
            .expect("count fixture responses");
    assert_eq!(response_count, 0);
    let undecided_summary = find_organizer_event_summary(&pool, &event.public_id, &organizer_hash)
        .await
        .expect("load the undecided organizer snapshot");
    assert_eq!(
        undecided_summary.decision, None,
        "the existing private summary request must project an undecided event without another mount request"
    );

    let (outcome, decision) = record_event_decision(
        &pool,
        &event.public_id,
        &organizer_hash,
        event.candidates[1].id,
    )
    .await
    .expect("authorize and persist the organizer's explicit decision");

    assert_eq!(outcome, EventDecisionWriteOutcome::Created);
    assert_eq!(decision, expected_decision(&event, 1));
    let decided_summary = find_organizer_event_summary(&pool, &event.public_id, &organizer_hash)
        .await
        .expect("load the decided organizer snapshot");
    assert_eq!(
        decided_summary.decision,
        Some(decision.clone()),
        "the same private summary snapshot must include the committed decision"
    );
    assert_eq!(
        serde_json::to_value(&decision).expect("serialize private decision projection"),
        serde_json::json!({
            "candidate_id": event.candidates[1].id,
            "local_date": "2026-09-20",
            "local_time": "14:00",
        }),
        "the browser projection must exclude decided_at, capabilities, hashes, and responses"
    );

    let stored: (String, i64) = sqlx::query_as(
        "SELECT event_public_id, candidate_id FROM event_decisions WHERE event_public_id = ?",
    )
    .bind(&event.public_id)
    .fetch_one(&pool)
    .await
    .expect("load stored event decision");
    assert_eq!(stored, (event.public_id, event.candidates[1].id));
}

#[tokio::test]
async fn same_candidate_retry_is_idempotent_but_a_different_candidate_conflicts() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let organizer_hash = capability_hash("decision-idempotency");
    let event = create_event(
        &pool,
        "decision-idempotency",
        &organizer_hash,
        "再試行する会",
    )
    .await;

    let (created_outcome, created_decision) = record_event_decision(
        &pool,
        &event.public_id,
        &organizer_hash,
        event.candidates[0].id,
    )
    .await
    .expect("create the first decision");
    let (retry_outcome, retry_decision) = record_event_decision(
        &pool,
        &event.public_id,
        &organizer_hash,
        event.candidates[0].id,
    )
    .await
    .expect("replay the already committed decision");

    assert_eq!(created_outcome, EventDecisionWriteOutcome::Created);
    assert_eq!(retry_outcome, EventDecisionWriteOutcome::AlreadyDecided);
    assert_eq!(created_decision, expected_decision(&event, 0));
    assert_eq!(retry_decision, created_decision);
    assert!(matches!(
        record_event_decision(
            &pool,
            &event.public_id,
            &organizer_hash,
            event.candidates[1].id,
        )
        .await,
        Err(OrganizerDecisionStorageError::Conflict)
    ));

    let stored_candidates: Vec<i64> =
        sqlx::query_scalar("SELECT candidate_id FROM event_decisions WHERE event_public_id = ?")
            .bind(&event.public_id)
            .fetch_all(&pool)
            .await
            .expect("load the immutable decision row");
    assert_eq!(stored_candidates, vec![event.candidates[0].id]);
}

#[tokio::test]
async fn wrong_missing_and_cross_event_authority_are_indistinguishable_not_found() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let first_hash = capability_hash("decision-first-organizer");
    let other_hash = capability_hash("decision-other-organizer");
    let first = create_event(&pool, "decision-private", &first_hash, "非公開の決定").await;
    create_event(
        &pool,
        "decision-private-other",
        &other_hash,
        "別主催者の決定",
    )
    .await;

    for (event_public_id, presented_hash) in [
        (
            first.public_id.as_str(),
            capability_hash("decision-wrong-organizer"),
        ),
        (first.public_id.as_str(), other_hash),
        ("decision-missing", first_hash),
    ] {
        assert!(matches!(
            record_event_decision(&pool, event_public_id, &presented_hash, i64::MAX,).await,
            Err(OrganizerDecisionStorageError::NotFound)
        ));
    }

    let decision_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_decisions")
        .fetch_one(&pool)
        .await
        .expect("count decisions after rejected authorization");
    assert_eq!(decision_count, 0);
}

#[tokio::test]
async fn a_candidate_from_another_event_is_rejected_after_authorization() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let first_hash = capability_hash("decision-candidate-first");
    let other_hash = capability_hash("decision-candidate-other");
    let first = create_event(
        &pool,
        "decision-candidate-owner",
        &first_hash,
        "候補を選ぶ会",
    )
    .await;
    let other = create_event(&pool, "decision-candidate-other", &other_hash, "別候補の会").await;

    assert!(matches!(
        record_event_decision(&pool, &first.public_id, &first_hash, other.candidates[0].id,).await,
        Err(OrganizerDecisionStorageError::CandidateMismatch)
    ));

    let decision_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_decisions")
        .fetch_one(&pool)
        .await
        .expect("count decisions after candidate mismatch");
    assert_eq!(decision_count, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn simultaneous_different_candidates_commit_exactly_one_immutable_decision() {
    let path = temporary_database_path("concurrent-event-decision");
    let pool = open_file(&path).await.expect("open file-backed SQLite");
    let organizer_hash = capability_hash("decision-concurrent");
    let event = create_event(
        &pool,
        "decision-concurrent",
        &organizer_hash,
        "同時に決める会",
    )
    .await;

    let (first_result, second_result) = tokio::join!(
        record_event_decision(
            &pool,
            &event.public_id,
            &organizer_hash,
            event.candidates[0].id,
        ),
        record_event_decision(
            &pool,
            &event.public_id,
            &organizer_hash,
            event.candidates[1].id,
        ),
    );

    let mut created_candidate_id = None;
    let mut created_count = 0;
    let mut conflict_count = 0;
    for result in [first_result, second_result] {
        match result {
            Ok((EventDecisionWriteOutcome::Created, decision)) => {
                created_count += 1;
                created_candidate_id = Some(decision.candidate_id);
            }
            Err(OrganizerDecisionStorageError::Conflict) => conflict_count += 1,
            unexpected => panic!("unexpected concurrent decision result: {unexpected:?}"),
        }
    }
    assert_eq!(created_count, 1);
    assert_eq!(conflict_count, 1);

    let stored_candidates: Vec<i64> =
        sqlx::query_scalar("SELECT candidate_id FROM event_decisions WHERE event_public_id = ?")
            .bind(&event.public_id)
            .fetch_all(&pool)
            .await
            .expect("load the one concurrent winner");
    assert_eq!(stored_candidates, vec![created_candidate_id.unwrap()]);

    pool.close().await;
    remove_temporary_database(&path);
}

#[tokio::test]
async fn a_committed_decision_survives_closing_and_reopening_the_database() {
    let path = temporary_database_path("persistent-event-decision");
    let pool = open_file(&path).await.expect("open file-backed SQLite");
    let organizer_hash = capability_hash("decision-persistent");
    let event = create_event(
        &pool,
        "decision-persistent",
        &organizer_hash,
        "再起動後も残る会",
    )
    .await;
    let (_, created_decision) = record_event_decision(
        &pool,
        &event.public_id,
        &organizer_hash,
        event.candidates[1].id,
    )
    .await
    .expect("commit decision before closing SQLite");
    pool.close().await;

    let reopened = open_file(&path).await.expect("reopen file-backed SQLite");
    let (outcome, reopened_decision) = record_event_decision(
        &reopened,
        &event.public_id,
        &organizer_hash,
        event.candidates[1].id,
    )
    .await
    .expect("read the committed decision through an idempotent retry");
    assert_eq!(outcome, EventDecisionWriteOutcome::AlreadyDecided);
    assert_eq!(reopened_decision, created_decision);

    let persisted_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM event_decisions WHERE event_public_id = ? AND candidate_id = ?",
    )
    .bind(&event.public_id)
    .bind(event.candidates[1].id)
    .fetch_one(&reopened)
    .await
    .expect("count the persisted decision after reopening SQLite");
    assert_eq!(persisted_count, 1);

    reopened.close().await;
    remove_temporary_database(&path);
}

#[tokio::test]
async fn decision_schema_enforces_primary_and_composite_keys_with_expected_delete_actions() {
    let pool = open_in_memory().await.expect("open isolated SQLite");
    let first_hash = capability_hash("decision-schema-first");
    let other_hash = capability_hash("decision-schema-other");
    let mismatch_hash = capability_hash("decision-schema-mismatch");
    let first = create_event(
        &pool,
        "decision-schema-first",
        &first_hash,
        "schemaを確認する会",
    )
    .await;
    let other = create_event(
        &pool,
        "decision-schema-other",
        &other_hash,
        "別candidateの会",
    )
    .await;
    let mismatch_target = create_event(
        &pool,
        "decision-schema-mismatch",
        &mismatch_hash,
        "composite FKの会",
    )
    .await;

    let columns = sqlx::query("PRAGMA table_info(event_decisions)")
        .fetch_all(&pool)
        .await
        .expect("inspect decision table columns");
    assert_eq!(columns.len(), 3);
    let column_names = columns
        .iter()
        .map(|column| column.get::<String, _>("name"))
        .collect::<Vec<_>>();
    assert_eq!(
        column_names,
        vec!["event_public_id", "candidate_id", "decided_at"]
    );
    let event_public_id_column = &columns[0];
    assert_eq!(event_public_id_column.get::<i64, _>("notnull"), 1);
    assert_eq!(event_public_id_column.get::<i64, _>("pk"), 1);
    assert_eq!(columns[1].get::<i64, _>("notnull"), 1);
    assert_eq!(columns[2].get::<i64, _>("notnull"), 1);
    assert_eq!(
        columns[2].get::<Option<String>, _>("dflt_value").as_deref(),
        Some("CURRENT_TIMESTAMP")
    );

    let candidate_indexes = sqlx::query("PRAGMA index_list(candidates)")
        .fetch_all(&pool)
        .await
        .expect("inspect candidate indexes");
    let mut has_composite_parent_key = false;
    for index in candidate_indexes {
        if index.get::<i64, _>("unique") != 1 {
            continue;
        }
        let index_name = index.get::<String, _>("name");
        let escaped_index_name = index_name.replace('\'', "''");
        let index_info_sql = format!("PRAGMA index_info('{escaped_index_name}')");
        let index_columns = sqlx::query(AssertSqlSafe(index_info_sql))
            .fetch_all(&pool)
            .await
            .expect("inspect one unique candidate index")
            .into_iter()
            .map(|column| column.get::<String, _>("name"))
            .collect::<Vec<_>>();
        if index_columns == ["id", "event_public_id"] {
            has_composite_parent_key = true;
            break;
        }
    }
    assert!(
        has_composite_parent_key,
        "the candidate composite FK requires a UNIQUE (id, event_public_id) parent key"
    );

    let foreign_keys = sqlx::query("PRAGMA foreign_key_list(event_decisions)")
        .fetch_all(&pool)
        .await
        .expect("inspect decision table foreign keys");
    assert_eq!(foreign_keys.len(), 3);
    assert!(foreign_keys.iter().any(|foreign_key| {
        foreign_key.get::<String, _>("table") == "events"
            && foreign_key.get::<String, _>("from") == "event_public_id"
            && foreign_key.get::<String, _>("to") == "public_id"
            && foreign_key.get::<String, _>("on_delete") == "CASCADE"
    }));
    let candidate_foreign_key_id = foreign_keys
        .iter()
        .find(|foreign_key| {
            foreign_key.get::<String, _>("table") == "candidates"
                && foreign_key.get::<String, _>("from") == "candidate_id"
                && foreign_key.get::<String, _>("to") == "id"
                && foreign_key.get::<String, _>("on_delete") == "RESTRICT"
        })
        .map(|foreign_key| foreign_key.get::<i64, _>("id"))
        .expect("candidate composite foreign key includes candidate_id");
    assert!(foreign_keys.iter().any(|foreign_key| {
        foreign_key.get::<i64, _>("id") == candidate_foreign_key_id
            && foreign_key.get::<String, _>("from") == "event_public_id"
            && foreign_key.get::<String, _>("to") == "event_public_id"
            && foreign_key.get::<String, _>("on_delete") == "RESTRICT"
    }));

    sqlx::query("INSERT INTO event_decisions (event_public_id, candidate_id) VALUES (?, ?)")
        .bind(&first.public_id)
        .bind(first.candidates[0].id)
        .execute(&pool)
        .await
        .expect("insert a schema-valid decision");

    let decided_at: String =
        sqlx::query_scalar("SELECT decided_at FROM event_decisions WHERE event_public_id = ?")
            .bind(&first.public_id)
            .fetch_one(&pool)
            .await
            .expect("load database-generated decision timestamp");
    assert_eq!(decided_at.len(), 19);
    let utc_age_seconds: i64 = sqlx::query_scalar(
        r#"
        SELECT abs(unixepoch('now') - unixepoch(decided_at))
        FROM event_decisions
        WHERE event_public_id = ?
        "#,
    )
    .bind(&first.public_id)
    .fetch_one(&pool)
    .await
    .expect("compare decided_at with SQLite UTC now");
    assert!(
        utc_age_seconds <= 5,
        "CURRENT_TIMESTAMP must record the database's current UTC time"
    );

    let duplicate_event =
        sqlx::query("INSERT INTO event_decisions (event_public_id, candidate_id) VALUES (?, ?)")
            .bind(&first.public_id)
            .bind(first.candidates[1].id)
            .execute(&pool)
            .await;
    assert!(
        duplicate_event.is_err(),
        "event_public_id PRIMARY KEY must keep one decision per event"
    );

    let cross_event_candidate =
        sqlx::query("INSERT INTO event_decisions (event_public_id, candidate_id) VALUES (?, ?)")
            .bind(&mismatch_target.public_id)
            .bind(other.candidates[0].id)
            .execute(&pool)
            .await;
    assert!(
        cross_event_candidate.is_err(),
        "the composite FK must reject a candidate owned by another event"
    );

    let delete_decided_candidate = sqlx::query("DELETE FROM candidates WHERE id = ?")
        .bind(first.candidates[0].id)
        .execute(&pool)
        .await;
    assert!(
        delete_decided_candidate.is_err(),
        "a decided candidate cannot be deleted independently"
    );

    sqlx::query("DELETE FROM events WHERE public_id = ?")
        .bind(&first.public_id)
        .execute(&pool)
        .await
        .expect("deleting the owning event cascades its decision");
    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM event_decisions WHERE event_public_id = ?")
            .bind(&first.public_id)
            .fetch_one(&pool)
            .await
            .expect("count decisions after event cascade");
    assert_eq!(remaining, 0);
}
