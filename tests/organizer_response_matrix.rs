use dioxus::prelude::*;
use tsunoru::{
    domain::{Availability, OrganizerResponseMatrix, ResponseMatrixCandidate, ResponseMatrixRow},
    ui::{
        OrganizerResponseMatrixFailure, OrganizerResponseMatrixLoading,
        OrganizerResponseMatrixRetryCallback, OrganizerResponseMatrixView,
    },
};

fn populated_matrix() -> OrganizerResponseMatrix {
    OrganizerResponseMatrix {
        name: "秋の餃子会".to_owned(),
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![
            ResponseMatrixCandidate {
                local_date: "2026-09-20".to_owned(),
                local_time: "14:00".to_owned(),
            },
            ResponseMatrixCandidate {
                local_date: "2026-09-18".to_owned(),
                local_time: "19:00".to_owned(),
            },
            ResponseMatrixCandidate {
                local_date: "2026-09-21".to_owned(),
                local_time: "12:30".to_owned(),
            },
        ],
        responses: vec![
            ResponseMatrixRow {
                respondent_name: "ミナ".to_owned(),
                availabilities: vec![
                    Availability::Available,
                    Availability::Maybe,
                    Availability::Unavailable,
                ],
            },
            ResponseMatrixRow {
                respondent_name: "ミナ".to_owned(),
                availabilities: vec![
                    Availability::Unavailable,
                    Availability::Available,
                    Availability::Maybe,
                ],
            },
        ],
    }
}

fn render_matrix(matrix: OrganizerResponseMatrix) -> String {
    dioxus_ssr::render_element(rsx! { OrganizerResponseMatrixView { matrix } })
}

#[test]
fn matrix_is_a_semantic_table_with_authored_candidates_and_distinct_response_rows() {
    let html = render_matrix(populated_matrix());

    for expected in [
        "回答者ごとの候補日時への回答",
        "秋の餃子会",
        "Asia/Tokyo の時刻",
        "<table",
        "<caption",
        "role=\"region\"",
        "tabindex=\"0\"",
        "aria-labelledby=\"response-matrix-caption\"",
        "aria-describedby=\"response-matrix-scroll-help\"",
    ] {
        assert!(
            html.contains(expected),
            "the response matrix should contain {expected:?}: {html}"
        );
    }
    assert_eq!(
        html.matches("scope=\"col\"").count(),
        4,
        "the respondent heading and every candidate need column scope: {html}"
    );
    assert_eq!(
        html.matches("scope=\"row\"").count(),
        2,
        "equal display names must remain separate response rows: {html}"
    );
    assert_eq!(
        html.matches(">ミナ<").count(),
        2,
        "a display name is not an identity or deduplication key: {html}"
    );

    let first = html.find("2026年9月20日 14:00").unwrap();
    let second = html.find("2026年9月18日 19:00").unwrap();
    let third = html.find("2026年9月21日 12:30").unwrap();
    assert!(
        first < second && second < third,
        "candidate columns must retain authored order rather than chronological order: {html}"
    );

    for (symbol, meaning, count) in [("○", "行ける", 2), ("△", "条件次第", 2), ("×", "難しい", 2)]
    {
        assert_eq!(
            html.matches(symbol).count(),
            count,
            "every recorded symbol must remain in its cell: {html}"
        );
        assert_eq!(
            html.matches(meaning).count(),
            count,
            "symbol meaning must also be available as text: {html}"
        );
    }
}

#[test]
fn matrix_projection_and_markup_exclude_authority_comments_ids_and_decision_controls() {
    let matrix = populated_matrix();
    let json = serde_json::to_string(&matrix).expect("serialize matrix projection");
    let html = render_matrix(matrix);

    for forbidden in [
        "organizer_capability",
        "organizer_capability_hash",
        "response_capability",
        "response_capability_hash",
        "public_id",
        "candidate_id",
        "response_id",
        "comment",
        "type=\"radio\"",
        "この日に決める",
        "日程を決定",
        "おすすめ",
        "順位",
        "スコア",
    ] {
        assert!(
            !json.contains(forbidden) && !html.contains(forbidden),
            "Story 5 must not expose or invent {forbidden:?}: json={json}; html={html}"
        );
    }
}

#[test]
fn zero_responses_uses_an_explanatory_state_instead_of_an_empty_table() {
    let html = render_matrix(OrganizerResponseMatrix {
        responses: Vec::new(),
        ..populated_matrix()
    });

    assert!(
        html.contains("まだ詳細回答はありません") && html.contains("role=\"status\""),
        "zero responses should be an announced successful state: {html}"
    );
    assert!(
        !html.contains("<table"),
        "an empty matrix should not expose a structurally empty table: {html}"
    );
}

#[test]
fn loading_failure_retry_and_lazy_disclosure_have_distinct_semantics() {
    let loading = dioxus_ssr::render_element(rsx! { OrganizerResponseMatrixLoading {} });
    assert!(
        loading.contains("role=\"status\"")
            && (loading.contains("aria-busy=true") || loading.contains("aria-busy=\"true\""))
            && loading.contains("集計表を読み込んでいます"),
        "matrix loading must be announced without replacing the summary: {loading}"
    );

    let failure = dioxus_ssr::render_element(rsx! {
        OrganizerResponseMatrixFailure {
            message: "集計表を読み込めませんでした。".to_owned(),
            on_retry: OrganizerResponseMatrixRetryCallback::from(move |_: ()| {}),
        }
    });
    assert!(
        failure.contains("role=\"alert\"")
            && failure.contains("集計表を読み込めませんでした。")
            && failure.contains("もう一度読み込む")
            && failure.contains("type=\"button\""),
        "matrix failure needs a non-submitting retry path: {failure}"
    );

    let ui_source = include_str!("../src/ui.rs");
    assert!(
        ui_source.contains("use_action(")
            && ui_source.contains("get_organizer_response_matrix")
            && ui_source.contains("aria_expanded")
            && ui_source.contains("aria_controls: \"organizer-response-matrix\"")
            && ui_source.contains("matrix_action.reset()"),
        "the client must fetch only after explicit disclosure and invalidate the payload after a successful summary refresh"
    );
    assert!(
        !ui_source.contains("use_server_future(get_organizer_response_matrix")
            && !ui_source.contains("use_resource(get_organizer_response_matrix"),
        "the private O(R x C) payload must not load on mount or during SSR"
    );
    assert!(
        ui_source.contains("summary_request_epoch")
            && ui_source.matches("summary_request_is_current").count() >= 3
            && ui_source.matches("summary_request_epoch.peek()").count() >= 2,
        "recovery, refresh, and retry must ignore an older summary request that finishes later"
    );
}
