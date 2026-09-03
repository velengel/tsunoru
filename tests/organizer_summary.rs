use dioxus::prelude::*;
use tsunoru::{
    domain::{
        CandidateResponseSummary, CandidateSummaryFact, OrganizerEventSummary,
        ResponseCommentPreview, derive_candidate_summary_facts,
    },
    ui::{
        OrganizerRecoveryForm, OrganizerRecoverySubmitCallback, OrganizerSummaryFailure,
        OrganizerSummaryLoading, OrganizerSummaryView,
    },
};

const RAW_ORGANIZER_CAPABILITY: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn candidate(
    id: i64,
    local_date: &str,
    local_time: &str,
    counts: (u64, u64, u64),
    fact: Option<CandidateSummaryFact>,
) -> CandidateResponseSummary {
    CandidateResponseSummary {
        id,
        local_date: local_date.to_owned(),
        local_time: local_time.to_owned(),
        available_count: counts.0,
        maybe_count: counts.1,
        unavailable_count: counts.2,
        fact,
    }
}

fn populated_summary() -> OrganizerEventSummary {
    OrganizerEventSummary {
        public_id: "event-one".to_owned(),
        name: "秋の餃子会".to_owned(),
        organizer_note: Some("焼きたてを囲みたいです".to_owned()),
        time_zone: "Asia/Tokyo".to_owned(),
        response_count: 6,
        candidates: vec![
            candidate(
                11,
                "2026-09-18",
                "19:00",
                (6, 0, 0),
                Some(CandidateSummaryFact::EveryoneAvailable),
            ),
            candidate(
                12,
                "2026-09-20",
                "14:00",
                (4, 2, 0),
                Some(CandidateSummaryFact::EveryoneAvailableIncludingMaybe),
            ),
            candidate(
                13,
                "2026-09-23",
                "18:30",
                (4, 1, 1),
                Some(CandidateSummaryFact::OneUnavailable),
            ),
        ],
        comment_count: 4,
        comment_previews: vec![
            ResponseCommentPreview {
                respondent_name: "ミナ".to_owned(),
                comment: "調整ありがとう！".to_owned(),
            },
            ResponseCommentPreview {
                respondent_name: "ソラ".to_owned(),
                comment: "18日なら遅くなっても大丈夫".to_owned(),
            },
            ResponseCommentPreview {
                respondent_name: "レン".to_owned(),
                comment: "<script>alert('x')</script> & 焼肉".to_owned(),
            },
        ],
        decision: None,
    }
}

fn empty_summary() -> OrganizerEventSummary {
    OrganizerEventSummary {
        public_id: "event-empty".to_owned(),
        name: "冬の読書会".to_owned(),
        organizer_note: None,
        time_zone: "Asia/Tokyo".to_owned(),
        response_count: 0,
        candidates: vec![
            candidate(21, "2026-12-05", "13:00", (0, 0, 0), None),
            candidate(22, "2026-12-12", "13:00", (0, 0, 0), None),
        ],
        comment_count: 0,
        comment_previews: Vec::new(),
        decision: None,
    }
}

fn render_summary(summary: OrganizerEventSummary) -> String {
    dioxus_ssr::render_element(rsx! { OrganizerSummaryView { summary } })
}

fn region_between<'a>(html: &'a str, start: &str, end: Option<&str>) -> &'a str {
    let start = html
        .find(start)
        .unwrap_or_else(|| panic!("missing region start {start:?}: {html}"));
    let end = end
        .and_then(|marker| {
            html[start + 1..]
                .find(marker)
                .map(|index| start + 1 + index)
        })
        .unwrap_or(html.len());
    &html[start..end]
}

fn opening_tag_with_id<'a>(html: &'a str, id: &str) -> &'a str {
    let id_marker = format!("id=\"{id}\"");
    let id_position = html
        .find(&id_marker)
        .unwrap_or_else(|| panic!("missing #{id}: {html}"));
    let start = html[..id_position]
        .rfind('<')
        .expect("the identified element should have an opening tag");
    let end = html[id_position..]
        .find('>')
        .map(|relative| id_position + relative + 1)
        .expect("the identified opening tag should be complete");
    &html[start..end]
}

#[test]
fn summary_keeps_authored_candidate_order_and_exposes_three_complete_counts() {
    let html = render_summary(populated_summary());

    for expected in [
        "主催者用",
        "秋の餃子会",
        "焼きたてを囲みたいです",
        "回答サマリー",
        "6件の回答",
        "Asia/Tokyo の時刻",
        "aria-label=\"候補日時ごとの回答サマリー\"",
    ] {
        assert!(
            html.contains(expected),
            "the organizer summary should contain {expected:?}: {html}"
        );
    }

    let first_date = html.find("2026年9月18日 19:00").unwrap();
    let second_date = html.find("2026年9月20日 14:00").unwrap();
    let third_date = html.find("2026年9月23日 18:30").unwrap();
    assert!(
        first_date < second_date && second_date < third_date,
        "candidate cards must retain authored order: {html}"
    );

    let first = region_between(
        &html,
        "id=\"summary-candidate-11\"",
        Some("id=\"summary-candidate-12\""),
    );
    for count in ["○ 行ける 6件", "△ 条件次第 0件", "× 難しい 0件"] {
        assert!(
            first.contains(count),
            "candidate 11 should contain {count:?}: {first}"
        );
    }

    let second = region_between(
        &html,
        "id=\"summary-candidate-12\"",
        Some("id=\"summary-candidate-13\""),
    );
    for count in ["○ 行ける 4件", "△ 条件次第 2件", "× 難しい 0件"] {
        assert!(
            second.contains(count),
            "candidate 12 should contain {count:?}: {second}"
        );
    }

    let third = region_between(&html, "id=\"summary-candidate-13\"", None);
    for count in ["○ 行ける 4件", "△ 条件次第 1件", "× 難しい 1件"] {
        assert!(
            third.contains(count),
            "candidate 13 should contain {count:?}: {third}"
        );
    }
}

#[test]
fn summary_uses_only_the_recorded_bounded_facts() {
    let html = render_summary(populated_summary());
    for fact in [
        "回答した全員が○です",
        "△を含めると、回答した全員が参加できそうです",
        "×が1件あります",
    ] {
        assert!(
            html.contains(fact),
            "the derived fact should be visible: {fact:?}"
        );
    }

    let unique_most = render_summary(OrganizerEventSummary {
        response_count: 6,
        candidates: vec![
            candidate(
                31,
                "2026-10-02",
                "19:00",
                (4, 0, 2),
                Some(CandidateSummaryFact::UniqueMostAvailable),
            ),
            candidate(32, "2026-10-03", "19:00", (3, 2, 1), None),
        ],
        comment_count: 0,
        comment_previews: Vec::new(),
        decision: None,
        ..empty_summary()
    });
    assert!(unique_most.contains("○が最も多い候補です"));

    for forbidden in ["おすすめ", "最適", "この日に決め", "順位", "スコア"] {
        assert!(
            !html.contains(forbidden) && !unique_most.contains(forbidden),
            "Story 4 must not turn a fact into recommendation {forbidden:?}"
        );
    }
}

#[test]
fn zero_responses_keep_every_candidate_without_an_empty_set_claim() {
    let html = render_summary(empty_summary());

    assert!(
        html.contains("まだ回答は届いていません")
            && html.contains("2026年12月5日 13:00")
            && html.contains("2026年12月12日 13:00")
            && html.matches("○ 行ける 0件").count() == 2
            && html.matches("△ 条件次第 0件").count() == 2
            && html.matches("× 難しい 0件").count() == 2,
        "an empty aggregate should retain all candidate facts as zero: {html}"
    );
    for false_claim in [
        "全員が○",
        "全員が参加できそう",
        "×が1件",
        "○が最も多い",
        "みんなから",
        "<details",
    ] {
        assert!(
            !html.contains(false_claim),
            "the empty set must not imply {false_claim:?}: {html}"
        );
    }
}

#[test]
fn comments_use_a_closed_native_disclosure_and_plain_text() {
    let html = render_summary(populated_summary());
    let details = opening_tag_with_id(&html, "summary-comments");
    let ui_source = include_str!("../src/ui.rs");

    assert!(
        details.starts_with("<details") && !details.contains(" open"),
        "comment previews should use a closed native disclosure: {details}"
    );
    for expected in [
        "<summary",
        "みんなから 4件",
        "4件中3件を表示しています",
        "ミナ",
        "調整ありがとう！",
        "ソラ",
        "18日なら遅くなっても大丈夫",
        "レン",
    ] {
        assert!(
            html.contains(expected),
            "comments should contain {expected:?}: {html}"
        );
    }
    assert_eq!(
        html.matches("class=\"comment-preview\"").count(),
        3,
        "Story 4 must cap the initial comment payload and layout: {html}"
    );
    assert!(
        !ui_source.contains("dangerous_inner_html"),
        "comments must use Dioxus text nodes instead of an HTML injection escape hatch"
    );
    assert!(
        !html.contains("<script>")
            && !html.contains("</script>")
            && (html.contains("&lt;script&gt;")
                || html.contains("&#x3C;script&#x3E;")
                || html.contains("&#60;script&#62;"))
            && (html.contains("&amp; 焼肉")
                || html.contains("&#x26; 焼肉")
                || html.contains("&#38; 焼肉")),
        "respondent comments must remain escaped plain text: {html}"
    );
}

#[test]
fn summary_does_not_render_story_five_table_or_story_six_decision_controls() {
    let html = render_summary(populated_summary());
    for forbidden in [
        "<table",
        "type=\"radio\"",
        "type=\"hidden\"",
        "回答者 × 候補日時",
        "回答者ごとの都合",
        "この日に決める",
        "日程を決定",
        "候補を選ぶ",
        "organizer_capability",
        "organizer_capability_hash",
        "response_id",
    ] {
        assert!(
            !html.contains(forbidden),
            "later-story output must stay out of the summary: {forbidden:?}: {html}"
        );
    }
}

#[test]
fn candidate_facts_use_the_conservative_priority_order() {
    let mut candidates = vec![
        candidate(1, "2026-09-18", "19:00", (4, 0, 0), None),
        candidate(2, "2026-09-19", "19:00", (3, 1, 0), None),
        candidate(3, "2026-09-20", "19:00", (3, 0, 1), None),
        candidate(4, "2026-09-21", "19:00", (2, 0, 2), None),
    ];

    derive_candidate_summary_facts(4, &mut candidates);

    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.fact)
            .collect::<Vec<_>>(),
        vec![
            Some(CandidateSummaryFact::EveryoneAvailable),
            Some(CandidateSummaryFact::EveryoneAvailableIncludingMaybe),
            Some(CandidateSummaryFact::OneUnavailable),
            None,
        ],
        "unanimity and a single unavailable response must take priority over relative counts"
    );
}

#[test]
fn candidate_facts_only_mark_a_strictly_unique_available_leader() {
    let mut unique = vec![
        candidate(1, "2026-09-18", "19:00", (3, 0, 2), None),
        candidate(2, "2026-09-19", "19:00", (2, 1, 2), None),
    ];
    derive_candidate_summary_facts(5, &mut unique);
    assert_eq!(
        unique
            .iter()
            .map(|candidate| candidate.fact)
            .collect::<Vec<_>>(),
        vec![Some(CandidateSummaryFact::UniqueMostAvailable), None]
    );

    let mut tied = vec![
        candidate(1, "2026-09-18", "19:00", (3, 0, 2), None),
        candidate(2, "2026-09-19", "19:00", (3, 0, 2), None),
    ];
    derive_candidate_summary_facts(5, &mut tied);
    assert!(
        tied.iter().all(|candidate| candidate.fact.is_none()),
        "a tie must not be presented as a recommendation or winner"
    );
}

#[test]
fn loading_failure_and_recovery_have_distinct_accessible_semantics() {
    let loading = dioxus_ssr::render_element(rsx! { OrganizerSummaryLoading {} });
    assert!(
        loading.contains("role=\"status\"")
            && (loading.contains("aria-busy=true") || loading.contains("aria-busy=\"true\""))
            && loading.contains("回答サマリーを読み込んでいます"),
        "loading should be announced without pretending to be summary content: {loading}"
    );

    let failure = dioxus_ssr::render_element(rsx! {
        OrganizerSummaryFailure { message: "回答を読み込めませんでした。".to_owned() }
    });
    assert!(
        failure.contains("role=\"alert\"")
            && failure.contains("id=\"organizer-summary-error-heading\"")
            && failure.contains("tabindex=\"-1\"")
            && failure.contains("回答を読み込めませんでした。"),
        "a failed async replacement needs an alert and focus target: {failure}"
    );

    let capability_kept_in_callback = RAW_ORGANIZER_CAPABILITY.to_owned();
    let callback = OrganizerRecoverySubmitCallback::from(move |_: String| {
        let _secret_kept_outside_markup = &capability_kept_in_callback;
    });
    let recovery = dioxus_ssr::render_element(rsx! {
        OrganizerRecoveryForm {
            initial_error: Some("主催者用の復旧キーを確認してください。".to_owned()),
            submitting: false,
            on_submit: callback,
        }
    });
    let input = opening_tag_with_id(&recovery, "organizer-recovery-key");
    assert!(
        recovery.contains("id=\"organizer-recovery-heading\"")
            && recovery.contains("tabindex=\"-1\"")
            && recovery.contains("主催者用の復旧キー")
            && recovery.contains("role=\"alert\"")
            && recovery.contains("id=\"organizer-recovery-error\"")
            && recovery.contains("aria-describedby=\"organizer-recovery-error\""),
        "recovery should explain and associate its validation state: {recovery}"
    );
    assert!(
        input.contains("type=\"password\"")
            && input.contains("autocomplete=\"off\"")
            && input.contains("maxlength=64")
            && input.contains("required")
            && input.contains("name=\"organizer-recovery-key\""),
        "the recovery key must be an explicit bounded secret input: {input}"
    );
    assert!(
        !recovery.contains(RAW_ORGANIZER_CAPABILITY)
            && !recovery.contains("type=\"hidden\"")
            && !recovery.contains("?organizer"),
        "captured organizer authority must never enter HTML or a URL: {recovery}"
    );

    let submitting = dioxus_ssr::render_element(rsx! {
        OrganizerRecoveryForm {
            initial_error: None,
            submitting: true,
            on_submit: OrganizerRecoverySubmitCallback::from(move |_: String| {}),
        }
    });
    assert!(
        submitting.contains("disabled") && submitting.contains("確認中"),
        "recovery submission should expose a non-duplicating busy state: {submitting}"
    );
}
