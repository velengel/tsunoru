use dioxus::prelude::*;
use tsunoru::{
    domain::{
        AccountAuthErrors, AccountHistory, HistoryDecision, OrganizedEventHistoryItem,
        ParticipatedEventHistoryItem, PublicCandidate, PublicEvent,
    },
    ui::{
        AccountHistoryFailure, AccountHistoryGuest, AccountHistoryLoading, AccountHistoryView,
        AccountLoginForm, AccountRegistrationForm, EventCreationForm, PublicEventView,
        account_login_failure_message, account_registration_failure_message,
    },
};

fn opening_tag<'a>(html: &'a str, id: &str) -> &'a str {
    let marker = format!("id=\"{id}\"");
    let marker_position = html
        .find(&marker)
        .unwrap_or_else(|| panic!("missing {id}: {html}"));
    let start = html[..marker_position]
        .rfind('<')
        .expect("the id should belong to one HTML tag");
    let end = html[marker_position..]
        .find('>')
        .map(|offset| marker_position + offset + 1)
        .expect("the HTML tag should close");
    &html[start..end]
}

fn history() -> AccountHistory {
    AccountHistory {
        login_id: "reader-with-a-long-login-id".to_owned(),
        organized_standalone: vec![OrganizedEventHistoryItem {
            public_id: "organized-public-id".to_owned(),
            name: "餃子会 <script>alert(1)</script>".to_owned(),
            time_zone: "Asia/Tokyo".to_owned(),
            decision: Some(HistoryDecision {
                local_date: "2027-01-15".to_owned(),
                local_time: "19:00".to_owned(),
            }),
            response_count: 6,
        }],
        organized_series: Vec::new(),
        participated: vec![ParticipatedEventHistoryItem {
            public_id: "participated-public-id".to_owned(),
            name: "読書会".to_owned(),
            time_zone: "UTC".to_owned(),
            decision: None,
        }],
    }
}

fn undecided_event() -> PublicEvent {
    PublicEvent {
        public_id: "public-event".to_owned(),
        name: "匿名イベント".to_owned(),
        organizer_note: None,
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![PublicCandidate {
            id: 1,
            local_date: "2027-01-15".to_owned(),
            local_time: "19:00".to_owned(),
        }],
        decision: None,
    }
}

#[test]
fn account_routes_are_explicit_and_private_from_indexing() {
    let routes = include_str!("../src/lib.rs");
    for route in ["/register", "/login", "/history"] {
        assert!(routes.contains(route), "missing account route {route}");
    }

    let ui = include_str!("../src/ui.rs");
    assert!(
        ui.contains("noindex,nofollow"),
        "auth and history pages should not invite indexing"
    );
}

#[test]
fn registration_form_explains_recovery_and_supports_password_managers() {
    let html = dioxus_ssr::render_element(rsx! {
        AccountRegistrationForm { initial_errors: AccountAuthErrors::default() }
    });

    for expected in [
        "アカウントを作る",
        "ログインID",
        "password",
        "password確認",
        "autocomplete=\"username\"",
        "autocomplete=\"new-password\"",
        "15文字以上",
        "復旧できません",
        "ログインなしでも日程調整できます",
    ] {
        assert!(
            html.contains(expected),
            "registration should contain {expected:?}: {html}"
        );
    }
    assert!(
        !html.contains("onpaste"),
        "paste must not be blocked: {html}"
    );
    for id in ["register-password", "register-password-confirmation"] {
        assert!(
            !opening_tag(&html, id).contains("maxlength"),
            "HTML maxlength counts UTF-16 units, so server-side Unicode limits remain authoritative"
        );
    }
}

#[test]
fn auth_errors_are_connected_without_echoing_passwords() {
    let html = dioxus_ssr::render_element(rsx! {
        AccountLoginForm {
            initial_errors: AccountAuthErrors {
                login_id: Some("ログインIDを確認してください。".to_owned()),
                password: None,
                password_confirmation: None,
                request: Some("ログインIDまたはpasswordを確認してください。".to_owned()),
            }
        }
    });

    assert!(html.contains("autocomplete=\"current-password\""));
    assert!(
        !opening_tag(&html, "login-password").contains("maxlength"),
        "HTML maxlength counts UTF-16 units, so server-side Unicode limits remain authoritative"
    );
    assert!(html.contains("aria-invalid=true"));
    assert!(html.contains("aria-describedby=\"login-id-error\""));
    assert!(html.contains("role=\"alert\""));
    assert!(html.contains("ログインIDまたはpasswordを確認してください。"));
    assert!(!html.contains("value=\"correct horse"));
}

#[test]
fn history_keeps_roles_separate_and_links_to_private_event_traces() {
    let html = dioxus_ssr::render_element(rsx! { AccountHistoryView { history: history() } });

    for expected in [
        "あなたの履歴",
        "主催したイベント",
        "参加したイベント",
        "餃子会",
        "2027年1月15日 19:00",
        "Asia/Tokyo の時刻",
        "回答 6件",
        "読書会",
        "調整中",
        "回答済み",
        "href=\"/history/events/organized-public-id\"",
        "href=\"/history/events/participated-public-id\"",
        "当時の記録を見る",
        "ログアウト",
    ] {
        assert!(
            html.contains(expected),
            "history should contain {expected:?}: {html}"
        );
    }
    for forbidden in [
        "<script>alert(1)</script>",
        "/summary",
        "organizer_capability",
        "respondent_name",
        "availability",
        "comment",
        "account_id",
    ] {
        assert!(
            !html.contains(forbidden),
            "history must not expose {forbidden:?}: {html}"
        );
    }
}

#[test]
fn history_states_distinguish_guest_expiry_loading_empty_and_failure() {
    let guest = dioxus_ssr::render_element(rsx! {
        AccountHistoryGuest { session_expired: false }
    });
    let expired = dioxus_ssr::render_element(rsx! {
        AccountHistoryGuest { session_expired: true }
    });
    let loading = dioxus_ssr::render_element(rsx! { AccountHistoryLoading {} });
    let failure = dioxus_ssr::render_element(rsx! { AccountHistoryFailure {} });
    let empty = dioxus_ssr::render_element(rsx! {
        AccountHistoryView {
            history: AccountHistory {
                login_id: "empty".to_owned(),
                organized_standalone: Vec::new(),
                organized_series: Vec::new(),
                participated: Vec::new(),
            }
        }
    });

    assert!(guest.contains("ログイン") && guest.contains("アカウントを作る"));
    assert!(!guest.contains("有効期限が切れました"));
    assert!(expired.contains("セッションの有効期限が切れました"));
    assert!(
        loading.contains("aria-busy=\"true\"") && loading.contains("履歴を読み込んでいます"),
        "loading state should expose progress semantics: {loading}"
    );
    assert!(failure.contains("role=\"alert\"") && failure.contains("もう一度試す"));
    assert!(empty.contains("主催したイベントはまだありません"));
    assert!(empty.contains("参加したイベントはまだありません"));
}

#[test]
fn auth_failures_keep_credential_rate_limit_and_retry_guidance_distinct() {
    let unauthorized = ServerFnError::ServerError {
        message: "private server detail".to_owned(),
        code: 401,
        details: None,
    };
    let rate_limited = ServerFnError::ServerError {
        message: "private server detail".to_owned(),
        code: 429,
        details: None,
    };
    let unavailable = ServerFnError::new("transport detail");
    let taken = ServerFnError::ServerError {
        message: "private server detail".to_owned(),
        code: 409,
        details: None,
    };

    assert_eq!(
        account_login_failure_message(&unauthorized),
        "ログインIDまたはpasswordを確認してください。"
    );
    assert!(account_login_failure_message(&rate_limited).contains("時間を置いて"));
    assert!(account_login_failure_message(&unavailable).contains("入力内容は残っています"));
    assert!(account_registration_failure_message(&taken).contains("ログインID"));
    assert!(account_registration_failure_message(&unavailable).contains("入力内容は残っています"));
}

#[test]
fn account_async_transitions_clear_private_history_and_move_focus_after_render() {
    let ui = include_str!("../src/ui.rs");
    assert!(
        ui.contains("on_logged_out.call(())")
            && ui.contains("AccountHistoryState::Guest")
            && ui.contains("focus_element_after_render(\"account-history-heading\")"),
        "logout must replace private state and async page changes must move focus"
    );
    assert!(
        ui.contains("focus_element_after_render(\"account-error-summary\")"),
        "newly rendered auth errors must receive focus after the DOM update"
    );
}

#[test]
fn account_routes_have_specific_document_titles() {
    let ui = include_str!("../src/ui.rs");
    for title in [
        "アカウントを作る | TSUNORU",
        "ログイン | TSUNORU",
        "イベント履歴 | TSUNORU",
    ] {
        assert!(ui.contains(title), "missing route-specific title {title:?}");
    }
}

#[test]
fn anonymous_forms_do_not_gain_account_fields_or_claim_actions() {
    let creation = dioxus_ssr::render_element(rsx! {
        EventCreationForm { initial_errors: Default::default() }
    });
    let response = dioxus_ssr::render_element(rsx! {
        PublicEventView { event: undecided_event() }
    });

    for html in [creation, response] {
        for forbidden in [
            "ログインID",
            "password",
            "アカウントを作る",
            "履歴に保存",
            "過去のイベントを取り込む",
        ] {
            assert!(
                !html.contains(forbidden),
                "anonymous action must not gain {forbidden:?}: {html}"
            );
        }
    }
}

#[test]
fn account_surfaces_reflow_without_hiding_long_values() {
    let css = include_str!("../assets/main.css");
    for expected in [
        ".account-page",
        "width: min(100%, 32rem)",
        ".history-page",
        "width: min(100%, 72rem)",
        "repeat(2, minmax(0, 1fr))",
        "overflow-wrap: anywhere",
        "min-height: 44px",
        "@media (max-width: 760px)",
    ] {
        assert!(
            css.contains(expected),
            "missing responsive account contract {expected:?}"
        );
    }
}
