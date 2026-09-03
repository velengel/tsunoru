use dioxus::prelude::*;
use tsunoru::{
    domain::{
        AccountEventTrace, AccountEventTraceCandidate, AccountEventTraceRelationship,
        AccountEventTraceResponse, Availability, HistoryDecision,
    },
    ui::{
        AccountEventTraceFailure, AccountEventTraceGuest, AccountEventTraceLoading,
        AccountEventTraceMissing, AccountEventTraceView,
    },
};

fn trace() -> AccountEventTrace {
    AccountEventTrace {
        public_id: "event-trace-01".to_owned(),
        name: "餃子会 <script>alert(1)</script>".to_owned(),
        organizer_note: Some("改行を含む\n主催者のひとこと".to_owned()),
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
        responses: vec![
            AccountEventTraceResponse {
                respondent_name: "ミナ".to_owned(),
                comment: Some("楽しみです <b>太字にしない</b>".to_owned()),
                availabilities: vec![Availability::Available, Availability::Maybe],
                is_current_account: true,
            },
            AccountEventTraceResponse {
                respondent_name: "ソラ".to_owned(),
                comment: None,
                availabilities: vec![Availability::Unavailable, Availability::Available],
                is_current_account: false,
            },
        ],
    }
}

#[test]
fn trace_view_separates_own_and_other_responses_without_edit_actions() {
    let html = dioxus_ssr::render_element(rsx! { AccountEventTraceView { trace: trace() } });

    for expected in [
        "日程調整の記録",
        "餃子会 &#60;script&#62;alert(1)&#60;/script&#62;",
        "決まった日時",
        "2027年1月15日 19:00",
        "あなたが送った回答",
        "ほかに届いた回答",
        "ミナ",
        "ソラ",
        "○ 行ける",
        "△ 条件次第",
        "× 難しい",
        "楽しみです &#60;b&#62;太字にしない&#60;/b&#62;",
        "ひとことなし",
        "href=\"/history\"",
        "href=\"/events/event-trace-01\"",
        "<details",
    ] {
        assert!(
            html.contains(expected),
            "trace should contain {expected:?}: {html}"
        );
    }
    for forbidden in [
        "<script>",
        "<b>太字",
        "<input",
        "<textarea",
        "type=\"submit\"",
    ] {
        assert!(
            !html.contains(forbidden),
            "trace must not contain {forbidden:?}: {html}"
        );
    }
}

#[test]
fn trace_states_distinguish_loading_guest_expired_missing_and_failure() {
    let loading = dioxus_ssr::render_element(rsx! { AccountEventTraceLoading {} });
    let guest = dioxus_ssr::render_element(rsx! {
        AccountEventTraceGuest { session_expired: false }
    });
    let expired = dioxus_ssr::render_element(rsx! {
        AccountEventTraceGuest { session_expired: true }
    });
    let missing = dioxus_ssr::render_element(rsx! { AccountEventTraceMissing {} });
    let failure = dioxus_ssr::render_element(rsx! { AccountEventTraceFailure {} });

    assert!(loading.contains("aria-busy=\"true\"") && loading.contains("記録を読み込んでいます"));
    assert!(guest.contains("ログイン") && !guest.contains("有効期限が切れました"));
    assert!(expired.contains("セッションの有効期限が切れました"));
    assert!(missing.contains("記録が見つかりません"));
    assert!(failure.contains("role=\"alert\"") && failure.contains("もう一度読み込む"));
}

#[test]
fn repeated_own_responses_have_stable_visible_ordinals() {
    let mut repeated = trace();
    repeated.responses = vec![
        AccountEventTraceResponse {
            respondent_name: "同じ表示名".to_owned(),
            comment: None,
            availabilities: vec![Availability::Available, Availability::Maybe],
            is_current_account: true,
        },
        AccountEventTraceResponse {
            respondent_name: "同じ表示名".to_owned(),
            comment: None,
            availabilities: vec![Availability::Available, Availability::Maybe],
            is_current_account: true,
        },
    ];

    let html = dioxus_ssr::render_element(rsx! { AccountEventTraceView { trace: repeated } });
    assert_eq!(html.matches("同じ表示名").count(), 2);
    assert!(
        html.contains("回答 1 / 2"),
        "first response needs an ordinal: {html}"
    );
    assert!(
        html.contains("回答 2 / 2"),
        "second response needs an ordinal: {html}"
    );
}

#[test]
fn private_trace_route_is_csr_only_and_history_list_stays_compact() {
    let routes = include_str!("../src/lib.rs");
    assert!(routes.contains("/history/events/:public_id"));

    let ui = include_str!("../src/ui.rs");
    assert!(ui.contains("noindex,nofollow"));
    assert!(ui.contains("get_account_event_trace(input).await"));
    assert!(ui.contains("use_effect(use_reactive((&public_id,)"));
    assert!(ui.contains("AccountEventTraceRouteContent { key: \"{public_id}\", public_id }"));
    assert!(!ui.contains("use_server_future(get_account_event_trace"));
    assert!(!ui.contains("use_loader(get_account_event_trace"));

    let history_start = ui.find("pub fn AccountHistoryView").unwrap();
    let history_end = ui[history_start..]
        .find("enum AccountEventTraceLoad")
        .map(|offset| history_start + offset)
        .unwrap();
    let history = &ui[history_start..history_end];
    assert!(history.contains("/history/events/{item.public_id}"));
    for forbidden in ["respondent_name", "availabilities", "respondent_comment"] {
        assert!(
            !history.contains(forbidden),
            "history list must stay compact"
        );
    }
}
