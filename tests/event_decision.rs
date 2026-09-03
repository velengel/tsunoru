use dioxus::prelude::*;
use tsunoru::{
    domain::{CandidateResponseSummary, OrganizerEventDecision},
    ui::{OrganizerDecisionForm, OrganizerDecisionSubmitCallback, OrganizerDecisionView},
};

const RAW_ORGANIZER_CAPABILITY: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn candidate(id: i64, local_date: &str, local_time: &str) -> CandidateResponseSummary {
    CandidateResponseSummary {
        id,
        local_date: local_date.to_owned(),
        local_time: local_time.to_owned(),
        available_count: 0,
        maybe_count: 0,
        unavailable_count: 0,
        fact: None,
    }
}

fn authored_candidates() -> Vec<CandidateResponseSummary> {
    vec![
        candidate(11, "2026-09-20", "14:00"),
        candidate(12, "2026-09-18", "19:00"),
        candidate(13, "2026-09-21", "12:30"),
    ]
}

fn render_form(
    initial_selected_candidate_id: Option<i64>,
    initial_error: Option<String>,
    submitting: bool,
) -> String {
    let capability_kept_in_callback = RAW_ORGANIZER_CAPABILITY.to_owned();
    let on_submit = OrganizerDecisionSubmitCallback::from(move |candidate_id: i64| {
        let _authority_stays_outside_markup = &capability_kept_in_callback;
        let _selected_candidate = candidate_id;
    });

    dioxus_ssr::render_element(rsx! {
        OrganizerDecisionForm {
            candidates: authored_candidates(),
            time_zone: "Asia/Tokyo".to_owned(),
            initial_selected_candidate_id,
            initial_error,
            submitting,
            on_submit,
        }
    })
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

fn source_region_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start = source
        .find(start)
        .unwrap_or_else(|| panic!("missing source start {start:?}"));
    let rest = &source[start..];
    let end = rest
        .find(end)
        .unwrap_or_else(|| panic!("missing source end {end:?}"));
    &rest[..end]
}

#[test]
fn undecided_form_uses_unselected_native_radios_in_authored_order() {
    let html = render_form(None, None, false);

    for expected in [
        "<form",
        "<fieldset",
        "<legend",
        "候補日時から一つ選ぶ",
        "Asia/Tokyo の時刻",
        "type=\"radio\"",
        "name=\"organizer-decision-candidate\"",
        "required",
    ] {
        assert!(
            html.contains(expected),
            "the undecided form should contain {expected:?}: {html}"
        );
    }
    assert_eq!(
        html.matches("type=\"radio\"").count(),
        3,
        "every authored candidate needs one native radio: {html}"
    );
    assert_eq!(
        html.matches("name=\"organizer-decision-candidate\"")
            .count(),
        3,
        "candidate radios must form one native single-selection group: {html}"
    );
    for candidate_id in [11, 12, 13] {
        let input_id = format!("organizer-decision-candidate-{candidate_id}");
        assert!(
            html.contains(&format!("id=\"{input_id}\""))
                && html.contains(&format!("for=\"{input_id}\"")),
            "each native radio needs an associated visible label: candidate={candidate_id}; html={html}"
        );
    }
    assert!(
        !html.contains("checked"),
        "no candidate may be preselected from counts or facts: {html}"
    );

    let first = html.find("2026年9月20日 14:00").unwrap();
    let second = html.find("2026年9月18日 19:00").unwrap();
    let third = html.find("2026年9月21日 12:30").unwrap();
    assert!(
        first < second && second < third,
        "decision choices must retain authored order rather than chronological order: {html}"
    );

    let submit = opening_tag_with_id(&html, "organizer-decision-submit");
    assert!(
        submit.contains("type=\"submit\"") && submit.contains("disabled"),
        "confirmation must stay disabled until the organizer explicitly selects a candidate: {submit}"
    );
    assert!(
        !html.contains(RAW_ORGANIZER_CAPABILITY),
        "authority captured by the parent callback must never enter decision markup: {html}"
    );
}

#[test]
fn selected_date_is_confirmed_in_context_before_a_separate_submit() {
    let html = render_form(Some(12), None, false);
    let submit = opening_tag_with_id(&html, "organizer-decision-submit");
    let selected_radio = opening_tag_with_id(&html, "organizer-decision-candidate-12");

    assert_eq!(
        html.matches("checked").count(),
        1,
        "exactly the organizer's explicit choice should be checked: {html}"
    );
    assert!(
        selected_radio.contains("checked"),
        "the checked radio must correspond to the selected candidate: {selected_radio}"
    );
    assert!(
        html.contains("id=\"organizer-decision-selection\"")
            && html.contains("role=\"status\"")
            && html.contains("選択中: 2026年9月18日 19:00")
            && html.contains("この日時に確定する"),
        "the choice needs a separate confirmation context and submit action: {html}"
    );
    assert!(
        !submit.contains("disabled"),
        "an explicitly selected candidate enables confirmation: {submit}"
    );
    assert!(
        !submit.contains("2026年") && !submit.contains("Asia/Tokyo"),
        "the selected date belongs in context, not inside the action label: {submit}"
    );
}

#[test]
fn submitting_and_failure_states_keep_the_selected_radio_for_retry() {
    let submitting = render_form(Some(12), None, true);
    let submitting_button = opening_tag_with_id(&submitting, "organizer-decision-submit");
    assert!(
        (submitting.contains("aria-busy=true") || submitting.contains("aria-busy=\"true\""))
            && submitting.contains("checked")
            && submitting_button.contains("disabled")
            && submitting.contains("確定中…"),
        "saving must prevent duplicate submit without losing the chosen candidate: {submitting}"
    );

    let failed = render_form(
        Some(12),
        Some("日程を確定できませんでした。選択は残っています。".to_owned()),
        false,
    );
    let retry_button = opening_tag_with_id(&failed, "organizer-decision-submit");
    assert!(
        failed.contains("role=\"alert\"")
            && failed.contains("id=\"organizer-decision-error\"")
            && failed
                .contains("aria-describedby=\"organizer-decision-help organizer-decision-error\"",)
            && failed.contains("日程を確定できませんでした。選択は残っています。")
            && failed.contains("checked")
            && failed.contains("選択中: 2026年9月18日 19:00")
            && failed.contains("もう一度確定する"),
        "a failed request needs an associated retry state with the choice retained: {failed}"
    );
    assert!(
        !retry_button.contains("disabled"),
        "the retained selection should allow a non-loading retry: {retry_button}"
    );
}

#[test]
fn decided_view_announces_the_immutable_result_without_an_edit_control() {
    let html = dioxus_ssr::render_element(rsx! {
        OrganizerDecisionView {
            decision: OrganizerEventDecision {
                candidate_id: 12,
                local_date: "2026-09-18".to_owned(),
                local_time: "19:00".to_owned(),
            },
            time_zone: "Asia/Tokyo".to_owned(),
        }
    });

    assert!(
        html.contains("role=\"status\"")
            && html.contains("id=\"organizer-decision-heading\"")
            && html.contains("tabindex=\"-1\"")
            && html.contains("日程を確定しました")
            && html.contains("2026年9月18日 19:00")
            && html.contains("Asia/Tokyo の時刻"),
        "the organizer needs one explicit, focusable confirmation of the saved date: {html}"
    );
    for forbidden in ["<form", "type=\"radio\"", "<button", "変更する", "決め直す"] {
        assert!(
            !html.contains(forbidden),
            "the first saved decision is immutable in Story 6 and must exclude {forbidden:?}: {html}"
        );
    }
}

#[test]
fn decision_ui_does_not_recommend_or_preempt_story_seven_sharing() {
    let undecided = render_form(None, None, false);
    let decided = dioxus_ssr::render_element(rsx! {
        OrganizerDecisionView {
            decision: OrganizerEventDecision {
                candidate_id: 12,
                local_date: "2026-09-18".to_owned(),
                local_time: "19:00".to_owned(),
            },
            time_zone: "Asia/Tokyo".to_owned(),
        }
    });

    for forbidden in [
        "おすすめ",
        "自動",
        "最多",
        "最適",
        "順位",
        "スコア",
        "回答者に共有",
        "カレンダー",
        "iCalendar",
        ".ics",
        "Google Calendar",
        "Outlook",
    ] {
        assert!(
            !undecided.contains(forbidden) && !decided.contains(forbidden),
            "Story 6 decision UI must not expose {forbidden:?}: undecided={undecided}; decided={decided}"
        );
    }
}

#[test]
fn client_submits_privately_then_updates_or_refreshes_without_clearing_a_failed_choice() {
    let ui_source = include_str!("../src/ui.rs");
    let server_source = include_str!("../src/server.rs");
    let client = source_region_between(
        ui_source,
        "fn OrganizerSummaryClient",
        "/// The organizer-facing projection",
    );

    assert!(
        server_source.contains("#[post(\"/api/organizer/events/decision\")]")
            && client.contains("get_organizer_event_decision")
            && client.contains("OrganizerDecisionInput")
            && client.contains("read_organizer_capability"),
        "a decision must be sent only through the private POST using a local capability"
    );
    assert!(
        !client.contains("use_server_future(get_organizer_event_decision")
            && !client.contains("use_resource(get_organizer_event_decision"),
        "the irreversible private POST must run only after explicit form submission"
    );

    let updates_summary = client.contains(".decision = Some(decision)")
        || client.contains("decision: Some(decision)");
    assert!(
        updates_summary
            && client.contains("summary.set(Some(")
            && client.contains("OrganizerDecidedEventHandoff")
            && client.contains("OrganizerDecisionForm"),
        "success must replace the undecided form by updating the existing summary state"
    );
    assert!(
        client.contains("code: 409")
            && client.contains("load_saved_organizer_summary")
            && client.contains("decision_error.set(Some(")
            && client.contains("decision_submitting.set(false)"),
        "a conflict must refresh the private summary, while an ordinary failure returns to retry"
    );
    assert!(
        client
            .matches("begin_summary_request(&mut summary_request_epoch)")
            .count()
            >= 5,
        "decision submission must supersede an older summary read so stale decision: null cannot replace a committed result"
    );
    let refresh_panel = source_region_between(
        client,
        "class: \"summary-refresh-panel\"",
        "if recovery_visible()",
    );
    assert!(
        refresh_panel.contains("decision_submitting()"),
        "manual summary refresh must be disabled while an irreversible decision request is pending"
    );
    assert!(
        client.contains("refresh_request_epoch")
            && client.contains("supersede_summary_refresh")
            && refresh_panel.contains("finish_summary_refresh"),
        "only the request that owns the refresh pending state may clear it after a newer operation supersedes that request"
    );
    let decision_flow = source_region_between(
        client,
        "let decision_public_id",
        "if let Some(current_summary)",
    );
    let decision_guard = source_region_between(
        decision_flow,
        "OrganizerDecisionSubmitCallback::from",
        "decision_error.set(None)",
    );
    assert!(
        !decision_guard.contains("refreshing()")
            && decision_flow.contains("supersede_summary_refresh"),
        "an explicit decision must supersede a stuck summary read instead of remaining blocked by its stale pending flag"
    );

    assert!(
        ui_source.contains("pub fn OrganizerDecisionForm")
            && ui_source.contains("use_signal")
            && ui_source.contains("initial_selected_candidate_id"),
        "the form must own its selected radio across parent error rerenders"
    );
}
