use tsunoru::domain::{
    AccountEventContinuationState, CandidateInput, EventContinuationCreateInput,
    EventContinuationPlan, EventContinuationPlanInput, NewEventInput, suggest_next_event_name,
};

fn event(name: &str) -> NewEventInput {
    NewEventInput {
        name: name.to_owned(),
        organizer_note: None,
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![CandidateInput {
            local_date: "2027-02-01".to_owned(),
            local_time: "19:00".to_owned(),
        }],
    }
}

#[test]
fn strict_trailing_ascii_number_is_the_only_name_suggestion() {
    assert_eq!(
        suggest_next_event_name("ベストユニゾン #1"),
        Some("ベストユニゾン #2".to_owned())
    );
    assert_eq!(
        suggest_next_event_name("ベストユニゾン #41"),
        Some("ベストユニゾン #42".to_owned())
    );

    for value in [
        "飲み会",
        "飲み会を17回開催",
        "飲み会#17",
        "飲み会  #17",
        "飲み会 ＃17",
        "飲み会 #１７",
        "飲み会 #0",
        "飲み会 #01",
        "#17",
        "飲み会 #17 次回",
        "飲み会 #-1",
        "飲み会 #18446744073709551615",
    ] {
        assert_eq!(
            suggest_next_event_name(value),
            None,
            "ambiguous name must not be suggested: {value:?}"
        );
    }

    let too_long = format!("{} #1", "長".repeat(98));
    assert_eq!(suggest_next_event_name(&too_long), None);
}

#[test]
fn continuation_inputs_normalize_public_ids_and_revalidate_the_event() {
    let plan = EventContinuationPlanInput {
        origin_event_public_id: "  origin-event  ".to_owned(),
    }
    .normalized_and_validated()
    .expect("one bounded origin id is valid");
    assert_eq!(plan.origin_event_public_id, "origin-event");

    let create = EventContinuationCreateInput {
        origin_event_public_id: " origin-event ".to_owned(),
        expected_tail_event_public_id: " origin-event ".to_owned(),
        event: event("  ベストユニゾン #2  "),
    }
    .normalized_and_validated()
    .expect("continuation revalidates its nested event");
    assert_eq!(create.origin_event_public_id, "origin-event");
    assert_eq!(create.expected_tail_event_public_id, "origin-event");
    assert_eq!(create.event.name, "ベストユニゾン #2");

    for malformed in ["", "https://example.test/events/one", "event?id=1"] {
        assert!(
            EventContinuationPlanInput {
                origin_event_public_id: malformed.to_owned(),
            }
            .normalized_and_validated()
            .is_err()
        );
    }
}

#[test]
fn private_plan_serializes_display_facts_without_authority_or_internal_series_ids() {
    let state = AccountEventContinuationState::Authenticated(EventContinuationPlan {
        origin_event_public_id: "origin-event".to_owned(),
        origin_event_name: "ベストユニゾン #1".to_owned(),
        series_name: "ベストユニゾン".to_owned(),
        tail_event_public_id: "origin-event".to_owned(),
        suggested_event_name: Some("ベストユニゾン #2".to_owned()),
    });
    let encoded = serde_json::to_string(&state).expect("serialize private continuation plan");

    for expected in ["origin-event", "ベストユニゾン", "#2"] {
        assert!(encoded.contains(expected));
    }
    for forbidden in [
        "series_id",
        "account_id",
        "session",
        "capability",
        "token",
        "hash",
        "password",
        "position",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "private plan must not serialize {forbidden:?}: {encoded}"
        );
    }
}
