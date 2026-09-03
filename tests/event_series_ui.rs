use dioxus::prelude::*;
use tsunoru::{
    domain::{
        AccountHistory, EventContinuationPlan, OrganizedEventHistoryItem,
        OrganizedEventSeriesHistory,
    },
    ui::{
        AccountEventContinuationFailure, AccountEventContinuationGuest,
        AccountEventContinuationLoading, AccountEventContinuationMissing, AccountHistoryView,
        EventContinuationView, EventCreationForm, event_continuation_failure_message,
    },
};

fn history_event(public_id: &str, name: &str) -> OrganizedEventHistoryItem {
    OrganizedEventHistoryItem {
        public_id: public_id.to_owned(),
        name: name.to_owned(),
        time_zone: "Asia/Tokyo".to_owned(),
        decision: None,
        response_count: 0,
    }
}

fn plan(suggestion: Option<&str>) -> EventContinuationPlan {
    EventContinuationPlan {
        origin_event_public_id: "series-origin".to_owned(),
        origin_event_name: "ベストユニゾン #1".to_owned(),
        series_name: "ベストユニゾン".to_owned(),
        tail_event_public_id: "series-origin".to_owned(),
        suggested_event_name: suggestion.map(ToOwned::to_owned),
    }
}

#[test]
fn continuation_states_are_distinct_and_never_claim_an_empty_plan() {
    let loading = dioxus_ssr::render_element(rsx! { AccountEventContinuationLoading {} });
    let guest = dioxus_ssr::render_element(rsx! {
        AccountEventContinuationGuest { session_expired: false }
    });
    let expired = dioxus_ssr::render_element(rsx! {
        AccountEventContinuationGuest { session_expired: true }
    });
    let missing = dioxus_ssr::render_element(rsx! { AccountEventContinuationMissing {} });
    let failure = dioxus_ssr::render_element(rsx! { AccountEventContinuationFailure {} });

    assert!(loading.contains("aria-busy=\"true\"") && loading.contains("読み込んでいます"));
    assert!(guest.contains("ログイン") && !guest.contains("有効期限が切れました"));
    assert!(expired.contains("有効期限が切れました"));
    assert!(missing.contains("見つかりません"));
    assert!(failure.contains("role=\"alert\"") && failure.contains("もう一度試す"));
}

#[test]
fn suggested_name_is_editable_and_no_suggestion_remains_a_valid_empty_input() {
    let suggested = dioxus_ssr::render_element(
        rsx! { EventContinuationView { plan: plan(Some("ベストユニゾン #2")) } },
    );
    for expected in [
        "同じ活動の次回をつのる",
        "value=\"ベストユニゾン #2\"",
        "過去の末尾名",
        "名前を変更しても同じ活動としてまとまります",
        "href=\"/history/events/series-origin\"",
        "href=\"/\"",
        "このイベントの続きにしないで通常作成へ",
        "value=\"19:00\"",
        "カレンダーを準備しています",
    ] {
        assert!(
            suggested.contains(expected),
            "missing {expected:?}: {suggested}"
        );
    }
    assert!(suggested.contains("required=true"));
    assert!(!suggested.contains("readonly"));
    assert!(!suggested.contains("type=\"time\""));

    let absent = dioxus_ssr::render_element(rsx! { EventContinuationView { plan: plan(None) } });
    assert!(absent.contains("次回名は提案できませんでした"));
    assert!(absent.contains("value=\"\""));
    assert!(absent.contains("required=true"));
}

#[test]
fn grouped_history_uses_one_native_disclosure_and_keeps_participation_flat() {
    let html = dioxus_ssr::render_element(rsx! {
        AccountHistoryView {
            history: AccountHistory {
                login_id: "series-reader".to_owned(),
                organized_standalone: vec![history_event("single", "単発の会")],
                organized_series: vec![OrganizedEventSeriesHistory {
                    series_name: "ベストユニゾン".to_owned(),
                    events: vec![
                        history_event("series-two", "ベストユニゾン 夏回"),
                        history_event("series-one", "ベストユニゾン #1"),
                    ],
                }],
                participated: Vec::new(),
            }
        }
    });

    for expected in [
        "継続している活動",
        "その他の主催イベント",
        "<details",
        "<summary",
        "ベストユニゾン",
        "2件",
        "href=\"/history/events/series-two\"",
        "href=\"/history/events/series-one\"",
        "単発の会",
        "参加したイベント",
    ] {
        assert!(
            html.contains(expected),
            "missing grouped history {expected:?}: {html}"
        );
    }
    let summary_start = html.find("<summary").unwrap();
    let summary_end = html[summary_start..]
        .find("</summary>")
        .map(|offset| summary_start + offset)
        .unwrap();
    let summary = &html[summary_start..summary_end];
    assert!(!summary.contains("<a") && !summary.contains("<button"));
    assert!(html.find("series-two").unwrap() < html.find("series-one").unwrap());
}

#[test]
fn stale_failure_guidance_keeps_the_draft_and_does_not_silently_fallback() {
    let stale = dioxus::prelude::ServerFnError::ServerError {
        message: "private detail".to_owned(),
        code: 409,
        details: None,
    };
    let expired = dioxus::prelude::ServerFnError::ServerError {
        message: "private detail".to_owned(),
        code: 401,
        details: None,
    };
    assert!(event_continuation_failure_message(&stale).contains("最新の続き情報"));
    assert!(event_continuation_failure_message(&stale).contains("入力内容は残っています"));
    assert!(event_continuation_failure_message(&expired).contains("ログイン"));

    let ui = include_str!("../src/ui.rs");
    assert!(ui.contains("create_account_event_continuation"));
    assert!(ui.contains("get_account_event_continuation_plan"));
    assert!(ui.contains("expected_tail_event_public_id"));
    assert!(ui.contains("continuation-latest-suggestion"));
    assert!(ui.contains("最新の候補を使う"));
    assert!(ui.contains("name.set(latest_suggestion"));
    assert!(ui.contains("login_required.set(status_code == 401)"));
    assert!(ui.contains("別タブでログインする"));
    assert!(ui.contains("target: \"_blank\""));
    assert!(!ui.contains("create_event(input).await.or_else"));
}

#[test]
fn continuation_validation_moves_focus_to_the_first_related_field() {
    let ui = include_str!("../src/ui.rs");
    assert!(
        ui.contains("first_continuation_error_target(&next_errors)"),
        "continuation validation must choose a field-specific focus target"
    );
    assert!(
        ui.contains("focus_element_after_render(focus_target).await"),
        "the chosen validation target must receive focus after error markup renders"
    );
}

#[test]
fn a_transient_retry_failure_keeps_the_latest_suggestion_recovery_hint() {
    let ui = include_str!("../src/ui.rs");
    let submit_start = ui
        .find("let event_input = match draft.prepare()")
        .expect("continuation submit validation exists");
    let request_start = ui[submit_start..]
        .find("match create_account_event_continuation(input).await")
        .map(|offset| submit_start + offset)
        .expect("continuation create request exists");
    let before_request = &ui[submit_start..request_start];
    assert!(
        !before_request.contains("latest_suggestion_state.set(None)"),
        "starting a retry must not discard a recovery hint before its outcome is known"
    );

    let error_start = ui[request_start..]
        .find("Err(error) =>")
        .map(|offset| request_start + offset)
        .expect("continuation error branch exists");
    let error_end = ui[error_start..]
        .find("submitting.set(false)")
        .map(|offset| error_start + offset)
        .expect("continuation submit cleanup follows the error branch");
    let error_branch = &ui[error_start..error_end];
    assert!(
        error_branch.contains("if status_code == 409")
            && error_branch.contains("latest_suggestion_state.set(None)"),
        "only a newer stale-tail result should invalidate the currently displayed suggestion"
    );

    let suggestion_start = ui
        .find("if let Some(latest_suggestion) = rendered_latest_suggestion")
        .expect("latest suggestion panel exists");
    let suggestion_end = ui[suggestion_start..]
        .find("if !plan_status().is_empty()")
        .map(|offset| suggestion_start + offset)
        .expect("plan status follows the latest suggestion panel");
    let suggestion_panel = &ui[suggestion_start..suggestion_end];
    assert!(
        suggestion_panel.contains("disabled: submitting() || refreshing_plan()"),
        "a suggestion must not change the visible name after the submitted request is fixed"
    );

    let context_start = ui
        .find("section { class: \"continuation-context\"")
        .expect("continuation context exists");
    let context_end = ui[context_start..]
        .find("nav { class: \"continuation-exit-links\"")
        .map(|offset| context_start + offset)
        .expect("continuation exit links follow the context");
    let context = &ui[context_start..context_end];
    let stale_message = context
        .find("if stale_plan()")
        .expect("stale context guidance exists");
    let latest_message = context
        .find("if has_latest_suggestion")
        .expect("latest suggestion context exists");
    assert!(
        stale_message < latest_message,
        "a new stale result must take precedence over an earlier suggestion explanation"
    );
}

#[test]
fn ordinary_creation_form_contains_no_series_or_name_suggestion_controls() {
    let html = dioxus_ssr::render_element(rsx! {
        EventCreationForm { initial_errors: Default::default() }
    });
    for forbidden in ["同じ活動", "次回名", "シリーズ", "series", "continuation"] {
        assert!(
            !html.contains(forbidden),
            "ordinary creation gained {forbidden:?}: {html}"
        );
    }
}
