use tsunoru::domain::{
    AccountEventTrace, AccountEventTraceCandidate, AccountEventTraceInput,
    AccountEventTraceRelationship, AccountEventTraceResponse, AccountEventTraceState, Availability,
    HistoryDecision,
};

fn trace() -> AccountEventTrace {
    AccountEventTrace {
        public_id: "event-trace-01".to_owned(),
        name: "秋の餃子会".to_owned(),
        organizer_note: Some("いつもの店で".to_owned()),
        time_zone: "Asia/Tokyo".to_owned(),
        relationship: AccountEventTraceRelationship::OrganizedAndParticipated,
        candidates: vec![
            AccountEventTraceCandidate {
                local_date: "2027-01-15".to_owned(),
                local_time: "19:00".to_owned(),
            },
            AccountEventTraceCandidate {
                local_date: "2027-01-16".to_owned(),
                local_time: "20:00".to_owned(),
            },
        ],
        decision: Some(HistoryDecision {
            local_date: "2027-01-15".to_owned(),
            local_time: "19:00".to_owned(),
        }),
        responses: vec![AccountEventTraceResponse {
            respondent_name: "ミナ".to_owned(),
            comment: Some("楽しみです".to_owned()),
            availabilities: vec![Availability::Available, Availability::Maybe],
            is_current_account: true,
        }],
    }
}

#[test]
fn event_trace_input_accepts_only_one_normalized_public_identifier() {
    let normalized = AccountEventTraceInput {
        event_public_id: "  event-trace-01  ".to_owned(),
    }
    .normalized_and_validated()
    .expect("a bounded public id is valid");
    assert_eq!(normalized.event_public_id, "event-trace-01");

    let errors = AccountEventTraceInput {
        event_public_id: "https://example.test/events/private?account_id=4".to_owned(),
    }
    .normalized_and_validated()
    .expect_err("a URL and account id must not become request authority");
    assert!(!errors.to_string().is_empty());
}

#[test]
fn private_trace_serialization_contains_display_facts_without_authority_material() {
    let json = serde_json::to_value(AccountEventTraceState::Authenticated(trace()))
        .expect("serialize private projection");
    let encoded = json.to_string();

    for expected in [
        "event-trace-01",
        "秋の餃子会",
        "organized_and_participated",
        "ミナ",
        "楽しみです",
        "available",
        "maybe",
    ] {
        assert!(
            encoded.contains(expected),
            "trace should contain {expected:?}"
        );
    }
    for forbidden in [
        "account_id",
        "login_id",
        "candidate_id",
        "response_id",
        "decided_at",
        "organizer_capability",
        "response_capability",
        "session",
        "token",
        "hash",
        "password",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "trace must not serialize authority field {forbidden:?}: {encoded}"
        );
    }
}

#[test]
fn relationship_is_a_closed_server_decided_state() {
    assert_ne!(
        AccountEventTraceRelationship::Organized,
        AccountEventTraceRelationship::Participated
    );
    assert_ne!(
        AccountEventTraceRelationship::Participated,
        AccountEventTraceRelationship::OrganizedAndParticipated
    );
}
