use dioxus::prelude::*;
use std::future::Future;
use tsunoru::{
    domain::{
        Availability, AvailabilityResponseDraft, AvailabilityResponseErrors,
        CandidateAvailabilityInput, NewAvailabilityResponseInput, ParticipantResponseMatrix,
        PreparedAvailabilityResponse, PublicCandidate, PublicEvent, RESPONDENT_NAME_MAX_CHARS,
        ResponseMatrixCandidate, ResponseMatrixRow,
    },
    server::submit_availability_response,
    ui::{
        AvailabilityResponseForm, AvailabilityResponseSuccess, ParticipantResponseMatrixView,
        PublicEventView,
    },
};

fn public_event() -> PublicEvent {
    PublicEvent {
        public_id: "7af78527-813b-4cdd-a632-058f3ce885aa".to_owned(),
        name: "秋の餃子会".to_owned(),
        organizer_note: Some("焼きたてを囲みたいです".to_owned()),
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![
            PublicCandidate {
                id: 11,
                local_date: "2026-09-18".to_owned(),
                local_time: "19:00".to_owned(),
            },
            PublicCandidate {
                id: 12,
                local_date: "2026-09-20".to_owned(),
                local_time: "14:00".to_owned(),
            },
        ],
        decision: None,
    }
}

fn choice(candidate_id: i64, availability: Availability) -> CandidateAvailabilityInput {
    CandidateAvailabilityInput {
        candidate_id,
        availability,
    }
}

fn require_http_error_contract<F>(_: F)
where
    F: Future<Output = std::result::Result<ParticipantResponseMatrix, ServerFnError>>,
{
}

#[test]
fn availability_submission_preserves_application_http_statuses() {
    let input = NewAvailabilityResponseInput {
        event_public_id: public_event().public_id,
        response_capability: "a".repeat(64),
        response: PreparedAvailabilityResponse {
            respondent_name: "ミナ".to_owned(),
            availabilities: vec![choice(11, Availability::Available)],
        },
    };

    require_http_error_contract(submit_availability_response(input));
}

#[test]
fn unknown_availability_does_not_cross_the_typed_request_boundary() {
    let json = r#"{
        "event_public_id":"7af78527-813b-4cdd-a632-058f3ce885aa",
        "response_capability":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "response":{
            "respondent_name":"ミナ",
            "availabilities":[{"candidate_id":11,"availability":"bogus"}]
        }
    }"#;

    let error = serde_json::from_str::<NewAvailabilityResponseInput>(json)
        .expect_err("unknown availability values must be rejected before persistence");
    assert!(error.to_string().contains("unknown variant `bogus`"));
}

#[test]
fn public_event_view_contains_the_complete_anonymous_response_form() {
    let html = dioxus_ssr::render_element(rsx! {
        PublicEventView { event: public_event() }
    });

    for expected in [
        "秋の餃子会",
        "焼きたてを囲みたいです",
        "あなたの名前",
        "autocomplete=\"name\"",
        "すべての候補を一つずつ選んでください",
        "○",
        "行ける",
        "△",
        "条件次第",
        "×",
        "難しい",
        "回答を送る",
    ] {
        assert!(
            html.contains(expected),
            "the shared page should expose {expected:?}: {html}"
        );
    }

    assert_eq!(
        html.matches("type=\"radio\"").count(),
        6,
        "each of two candidates should have exactly three native radios: {html}"
    );
    assert_eq!(
        html.matches("required").count(),
        7,
        "the name and every native radio should expose required semantics: {html}"
    );
    assert_eq!(
        html.matches("name=\"availability-11\"").count(),
        3,
        "one candidate must form one native radio group: {html}"
    );
    assert_eq!(
        html.matches("name=\"availability-12\"").count(),
        3,
        "different candidates must not share one radio group: {html}"
    );
    assert!(
        !html.contains("checked")
            && !html.contains("ログイン")
            && !html.contains("プロフィール")
            && !html.contains("コメントを入力"),
        "answers need an explicit choice and no later-story requirement: {html}"
    );
}

#[test]
fn response_validation_errors_are_connected_to_name_and_candidate_groups() {
    let errors = AvailabilityResponseErrors {
        respondent_name: Some("名前を入力してください。".to_owned()),
        candidate_ids: vec![12],
        request: None,
    };

    let html = dioxus_ssr::render_element(rsx! {
        AvailabilityResponseForm { event: public_event(), initial_errors: errors }
    });

    assert!(
        html.contains("id=\"response-error-summary\"")
            && html.contains("role=\"alert\"")
            && html.contains("id=\"respondent-name-error\"")
            && html.contains("aria-describedby=\"respondent-name-error\"")
            && html.contains("id=\"candidate-12-error\"")
            && html.contains("2026年9月20日 14:00 の都合を選んでください。"),
        "one alert should summarize errors while fields retain specific descriptions: {html}"
    );
    assert_eq!(
        html.matches("role=\"alert\"").count(),
        1,
        "candidate-level errors must not create an alert storm: {html}"
    );
}

#[test]
fn request_level_validation_has_a_programmatic_focus_target() {
    let errors = AvailabilityResponseErrors {
        request: Some("候補日時への回答を確認してください。".to_owned()),
        ..AvailabilityResponseErrors::default()
    };
    let html = dioxus_ssr::render_element(rsx! {
        AvailabilityResponseForm { event: public_event(), initial_errors: errors }
    });

    assert!(
        html.contains("id=\"response-heading\" tabindex=\"-1\"")
            && html.contains("候補日時への回答を確認してください。"),
        "request errors need a focusable section target: {html}"
    );
}

#[test]
fn response_draft_requires_a_name_and_every_candidate_exactly_once() {
    let draft = AvailabilityResponseDraft {
        respondent_name: "   ".to_owned(),
        candidate_ids: vec![11, 12],
        availabilities: vec![choice(11, Availability::Available)],
    };

    let errors = draft
        .prepare()
        .expect_err("an empty name and a missing candidate must be rejected");
    assert_eq!(
        errors.respondent_name.as_deref(),
        Some("名前を入力してください。")
    );
    assert_eq!(errors.candidate_ids, vec![12]);

    let duplicate = AvailabilityResponseDraft {
        respondent_name: "ミナ".to_owned(),
        candidate_ids: vec![11, 12],
        availabilities: vec![
            choice(11, Availability::Available),
            choice(11, Availability::Maybe),
            choice(12, Availability::Unavailable),
        ],
    };
    assert!(
        duplicate.prepare().is_err(),
        "a modified request must not include one candidate twice"
    );
}

#[test]
fn response_draft_accepts_all_three_values_and_normalizes_the_name() {
    let prepared = AvailabilityResponseDraft {
        respondent_name: "  ミナ  ".to_owned(),
        candidate_ids: vec![11, 12, 13],
        availabilities: vec![
            choice(13, Availability::Unavailable),
            choice(11, Availability::Available),
            choice(12, Availability::Maybe),
        ],
    }
    .prepare()
    .expect("all candidates have one typed availability");

    assert_eq!(prepared.respondent_name, "ミナ");
    assert_eq!(
        prepared.availabilities,
        vec![
            choice(11, Availability::Available),
            choice(12, Availability::Maybe),
            choice(13, Availability::Unavailable),
        ],
        "the canonical payload should follow candidate order"
    );
    assert_eq!(
        serde_json::to_string(&Availability::Available).unwrap(),
        "\"available\""
    );
    assert_eq!(
        serde_json::to_string(&Availability::Maybe).unwrap(),
        "\"maybe\""
    );
    assert_eq!(
        serde_json::to_string(&Availability::Unavailable).unwrap(),
        "\"unavailable\""
    );
}

#[test]
fn anonymous_response_name_has_a_bounded_length() {
    let errors = AvailabilityResponseDraft {
        respondent_name: "名".repeat(RESPONDENT_NAME_MAX_CHARS + 1),
        candidate_ids: vec![11],
        availabilities: vec![choice(11, Availability::Available)],
    }
    .prepare()
    .expect_err("an anonymous endpoint must not accept an unbounded name");

    assert_eq!(
        errors.respondent_name.as_deref(),
        Some("名前は100文字以内で入力してください。")
    );
}

#[test]
fn response_success_is_announced_without_exposing_later_stories() {
    let html = dioxus_ssr::render_element(rsx! {
        AvailabilityResponseSuccess {}
    });

    assert!(
        html.contains("id=\"response-success-heading\"")
            && html.contains("tabindex=\"-1\"")
            && html.contains("aria-live=\"polite\"")
            && html.contains("回答を送りました")
            && html.contains("この画面は閉じて大丈夫です"),
        "the async replacement should be focusable and clearly complete: {html}"
    );
    assert!(
        !html.contains("textarea") && !html.contains("回答サマリー") && !html.contains("回答人数"),
        "optional comment controls belong to a separate post-answer section, and aggregation belongs to later stories: {html}"
    );
}

#[test]
fn successful_answer_view_shows_every_response_as_an_accessible_matrix() {
    let matrix = ParticipantResponseMatrix {
        name: "秋の餃子会".to_owned(),
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![ResponseMatrixCandidate {
            local_date: "2026-09-18".to_owned(),
            local_time: "19:00".to_owned(),
        }],
        responses: vec![
            ResponseMatrixRow {
                respondent_name: "ミナ".to_owned(),
                availabilities: vec![Availability::Available],
            },
            ResponseMatrixRow {
                respondent_name: "ソラ".to_owned(),
                availabilities: vec![Availability::Maybe],
            },
        ],
    };
    let html = dioxus_ssr::render_element(rsx! {
        ParticipantResponseMatrixView { matrix }
    });

    for expected in [
        "みんなの回答",
        "送った時点の一覧です",
        "回答者ごとの候補日時への回答",
        "ミナ",
        "ソラ",
        "○",
        "△",
        "role=\"region\"",
    ] {
        assert!(html.contains(expected), "missing {expected:?}: {html}");
    }
    assert_eq!(html.matches("scope=\"row\"").count(), 2);
    assert!(!html.contains("capability") && !html.contains("comment"));
}

#[test]
fn public_event_before_answering_still_excludes_everyones_response_matrix() {
    let html = dioxus_ssr::render_element(rsx! {
        PublicEventView { event: public_event() }
    });
    assert!(!html.contains("みんなの回答"));
    assert!(!html.contains("回答者ごとの候補日時への回答"));
}

#[test]
fn server_input_revalidates_identifiers_capability_and_candidate_count() {
    let valid = NewAvailabilityResponseInput {
        event_public_id: "event-one".to_owned(),
        response_capability: "a1".repeat(32),
        response: PreparedAvailabilityResponse {
            respondent_name: "  ミナ  ".to_owned(),
            availabilities: vec![choice(11, Availability::Available)],
        },
    };
    assert_eq!(
        valid
            .normalized_and_validated()
            .expect("well-shaped server input")
            .response
            .respondent_name,
        "ミナ"
    );

    let mut invalid_event = valid.clone();
    invalid_event.event_public_id = "../other-event".to_owned();
    assert!(invalid_event.normalized_and_validated().is_err());

    let mut invalid_capability = valid.clone();
    invalid_capability.response_capability = "A".repeat(64);
    assert!(invalid_capability.normalized_and_validated().is_err());

    let mut duplicated_candidate = valid.clone();
    duplicated_candidate
        .response
        .availabilities
        .push(choice(11, Availability::Maybe));
    assert!(duplicated_candidate.normalized_and_validated().is_err());

    let mut too_many_candidates = valid;
    too_many_candidates.response.availabilities = (1..=21)
        .map(|candidate_id| choice(candidate_id, Availability::Available))
        .collect();
    assert!(too_many_candidates.normalized_and_validated().is_err());
}
