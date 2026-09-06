//! User-visible Dioxus components for creating and opening an event.

pub use crate::shared_ui::{
    CalendarCandidateChange, CalendarDateToggleCallback, CalendarMonth, CandidateCalendar,
    DEFAULT_CANDIDATE_TIME, OrganizerResponseMatrixView, toggle_calendar_candidate,
};
use crate::shared_ui::{CandidateDateTimePicker, format_local_start};
use crate::{
    domain::{
        AccountAuthErrors, AccountEventContinuationState, AccountEventTrace,
        AccountEventTraceCandidate, AccountEventTraceInput, AccountEventTraceRelationship,
        AccountEventTraceResponse, AccountEventTraceState, AccountHistory, AccountHistoryState,
        AccountLoginInput, AccountRegistrationInput, Availability, AvailabilityResponseDraft,
        AvailabilityResponseErrors, CandidateAvailabilityInput, CandidateInput,
        CandidateResponseSummary, CandidateSummaryFact, CreatedEvent, EVENT_NAME_MAX_CHARS,
        EventContinuationCreateInput, EventContinuationPlan, EventContinuationPlanInput,
        EventCreationDraft, EventCreationErrors, LOGIN_ID_MAX_CHARS, NewAvailabilityResponseInput,
        NewResponseCommentInput, ORGANIZER_NOTE_MAX_CHARS, OrganizerDecisionInput,
        OrganizerEventDecision, OrganizerEventSummary, OrganizerResponseMatrix,
        OrganizerSummaryInput, ParticipantResponseMatrix, PublicCandidate, PublicEvent,
        PublicEventDecision, RESPONDENT_COMMENT_MAX_CHARS, RESPONDENT_NAME_MAX_CHARS,
        ResponseCommentDraft,
    },
    server::{
        create_account_event_continuation, create_event, get_account_event_continuation_plan,
        get_account_event_trace, get_account_history, get_organizer_event_decision,
        get_organizer_event_summary, get_organizer_response_matrix, get_public_event,
        login_account, logout_account, register_account, submit_availability_response,
        submit_response_comment,
    },
};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResponseCommentOutcome {
    Pending,
    Saved,
    Skipped,
}

/// User-visible state after attempting native share or its copy fallback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShareActionState {
    #[default]
    ReadyToShare,
    ReadyToCopy,
    InProgress,
    ShareStarted,
    UrlCopied,
    ManualCopy,
}

/// Start at most one browser share or copy operation at a time.
pub fn begin_share_action(current: ShareActionState) -> Option<ShareActionState> {
    (current != ShareActionState::InProgress).then_some(ShareActionState::InProgress)
}

/// Keep UI labels aligned with the result sent by the browser script.
pub fn next_share_action_state(current: ShareActionState, result: &str) -> ShareActionState {
    match result {
        "started" => ShareActionState::ShareStarted,
        "copied" => ShareActionState::UrlCopied,
        "cancelled" | "failed" => ShareActionState::ReadyToCopy,
        "manual" => ShareActionState::ManualCopy,
        _ if current == ShareActionState::ReadyToCopy => ShareActionState::ManualCopy,
        _ => ShareActionState::ReadyToCopy,
    }
}

#[derive(Clone)]
pub struct ResponseCommentSubmitCallback(Rc<RefCell<dyn FnMut(String)>>);

impl ResponseCommentSubmitCallback {
    fn call(&self, comment: String) {
        (self.0.borrow_mut())(comment);
    }
}

impl<F> From<F> for ResponseCommentSubmitCallback
where
    F: FnMut(String) + 'static,
{
    fn from(callback: F) -> Self {
        Self(Rc::new(RefCell::new(callback)))
    }
}

impl PartialEq for ResponseCommentSubmitCallback {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
pub struct ResponseCommentSkipCallback(Rc<RefCell<dyn FnMut(())>>);

impl ResponseCommentSkipCallback {
    fn call(&self) {
        (self.0.borrow_mut())(());
    }
}

impl<F> From<F> for ResponseCommentSkipCallback
where
    F: FnMut(()) + 'static,
{
    fn from(callback: F) -> Self {
        Self(Rc::new(RefCell::new(callback)))
    }
}

impl PartialEq for ResponseCommentSkipCallback {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
pub struct OrganizerRecoverySubmitCallback(Rc<RefCell<dyn FnMut(String)>>);

impl OrganizerRecoverySubmitCallback {
    fn call(&self, capability: String) {
        (self.0.borrow_mut())(capability);
    }
}

impl<F> From<F> for OrganizerRecoverySubmitCallback
where
    F: FnMut(String) + 'static,
{
    fn from(callback: F) -> Self {
        Self(Rc::new(RefCell::new(callback)))
    }
}

impl PartialEq for OrganizerRecoverySubmitCallback {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
pub struct OrganizerResponseMatrixRetryCallback(Rc<RefCell<dyn FnMut(())>>);

impl OrganizerResponseMatrixRetryCallback {
    fn call(&self) {
        (self.0.borrow_mut())(());
    }
}

impl<F> From<F> for OrganizerResponseMatrixRetryCallback
where
    F: FnMut(()) + 'static,
{
    fn from(callback: F) -> Self {
        Self(Rc::new(RefCell::new(callback)))
    }
}

impl PartialEq for OrganizerResponseMatrixRetryCallback {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
pub struct OrganizerDecisionSubmitCallback(Rc<RefCell<dyn FnMut(i64)>>);

impl OrganizerDecisionSubmitCallback {
    fn call(&self, candidate_id: i64) {
        (self.0.borrow_mut())(candidate_id);
    }
}

impl<F> From<F> for OrganizerDecisionSubmitCallback
where
    F: FnMut(i64) + 'static,
{
    fn from(callback: F) -> Self {
        Self(Rc::new(RefCell::new(callback)))
    }
}

impl PartialEq for OrganizerDecisionSubmitCallback {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum OrganizerSummaryGate {
    Loading,
    Recovery,
    Failure(String),
}

enum StoredOrganizerSummaryLoad {
    MissingCapability,
    RejectedCapability,
    Loaded(Box<OrganizerEventSummary>),
    Failed,
}

fn begin_summary_request(summary_request_epoch: &mut Signal<u64>) -> u64 {
    let request_epoch = (*summary_request_epoch.peek()).wrapping_add(1);
    summary_request_epoch.set(request_epoch);
    request_epoch
}

fn summary_request_is_current(summary_request_epoch: Signal<u64>, request_epoch: u64) -> bool {
    *summary_request_epoch.peek() == request_epoch
}

fn supersede_summary_refresh(
    mut refreshing: Signal<bool>,
    mut refresh_request_epoch: Signal<Option<u64>>,
) {
    refresh_request_epoch.set(None);
    refreshing.set(false);
}

fn finish_summary_refresh(
    mut refreshing: Signal<bool>,
    mut refresh_request_epoch: Signal<Option<u64>>,
    request_epoch: u64,
) {
    if *refresh_request_epoch.peek() == Some(request_epoch) {
        refresh_request_epoch.set(None);
        refreshing.set(false);
    }
}

/// Route component for the anonymous creation page.
#[component]
pub fn Create() -> Element {
    rsx! {
        main { class: "app-shell",
            section { class: "creation-layout",
                header { class: "page-heading",
                    div { class: "page-heading-topline",
                        a { class: "wordmark", href: "/", "TSUNORU" }
                        a { class: "account-entry-link", href: "/history", "履歴" }
                    }
                    p { class: "eyebrow", "友人と仲間の日程調整" }
                    h1 { "日程をつのる" }
                    p { class: "lead",
                        "集まりたい気持ちと候補の日を、みんなに渡せる形にします。"
                    }
                }
                EventCreationForm { initial_errors: EventCreationErrors::default() }
            }
        }
    }
}

/// Route for creating an optional account without entering the anonymous core flow.
#[component]
pub fn Register() -> Element {
    rsx! {
        document::Title { "アカウントを作る | TSUNORU" }
        document::Meta { name: "robots", content: "noindex,nofollow" }
        main { class: "app-shell account-route",
            section { class: "account-page",
                a { class: "wordmark", href: "/", "TSUNORU" }
                p { class: "eyebrow", "任意の履歴" }
                h1 { "アカウントを作る" }
                p { class: "account-lead",
                    "ログイン中に主催・回答したイベントへ、履歴から戻れるようになります。ログインなしでも日程調整できます。"
                }
                AccountRegistrationForm { initial_errors: AccountAuthErrors::default() }
                p { class: "account-switch",
                    "すでにアカウントがある場合は "
                    a { href: "/login", "ログイン" }
                }
            }
        }
    }
}

/// Route for beginning one account-history session.
#[component]
pub fn Login() -> Element {
    rsx! {
        document::Title { "ログイン | TSUNORU" }
        document::Meta { name: "robots", content: "noindex,nofollow" }
        main { class: "app-shell account-route",
            section { class: "account-page",
                a { class: "wordmark", href: "/", "TSUNORU" }
                p { class: "eyebrow", "任意の履歴" }
                h1 { "ログイン" }
                p { class: "account-lead",
                    "ログイン中に主催・回答したイベントの履歴を開きます。"
                }
                AccountLoginForm { initial_errors: AccountAuthErrors::default() }
                p { class: "account-switch",
                    "初めて使う場合は "
                    a { href: "/register", "アカウントを作る" }
                }
            }
        }
    }
}

/// Stateful account-registration form with client and server validation.
#[component]
pub fn AccountRegistrationForm(initial_errors: AccountAuthErrors) -> Element {
    let router = try_router();
    let mut login_id = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut password_confirmation = use_signal(String::new);
    let mut errors = use_signal(|| initial_errors);
    let mut submitting = use_signal(|| false);

    let current_errors = errors();
    rsx! {
        form {
            class: "account-form",
            novalidate: true,
            onsubmit: move |submit_event| {
                async move {
                    submit_event.prevent_default();
                    if submitting() {
                        return;
                    }
                    let input = AccountRegistrationInput {
                        login_id: login_id(),
                        password: password(),
                        password_confirmation: password_confirmation(),
                    };
                    if let Err(next_errors) = input.prepare() {
                        let focus_target = first_account_error_target(&next_errors, true);
                        errors.set(next_errors);
                        focus_element(focus_target).await;
                        return;
                    }

                    errors.set(AccountAuthErrors::default());
                    submitting.set(true);
                    match register_account(input).await {
                        Ok(_) => {
                            password.set(String::new());
                            password_confirmation.set(String::new());
                            if let Some(router) = router.as_ref() {
                                let _ = router.replace(crate::Route::History {});
                            }
                        }
                        Err(error) => {
                            errors.write().request =
                                Some(account_registration_failure_message(&error));
                            submitting.set(false);
                            focus_element_after_render("account-error-summary").await;
                        }
                    }
                }
            },

            if let Some(message) = current_errors.request.as_deref() {
                p { id: "account-error-summary", class: "form-error", role: "alert", tabindex: "-1", "{message}" }
            }
            div { class: "field-group",
                label { r#for: "register-login-id", "ログインID" }
                input {
                    id: "register-login-id",
                    name: "username",
                    r#type: "text",
                    autocomplete: "username",
                    maxlength: LOGIN_ID_MAX_CHARS,
                    required: true,
                    value: "{login_id}",
                    aria_invalid: current_errors.login_id.is_some(),
                    aria_describedby: if current_errors.login_id.is_some() { "register-login-id-help register-login-id-error" } else { "register-login-id-help" },
                    oninput: move |event| {
                        login_id.set(event.value());
                        errors.write().login_id = None;
                    },
                }
                p { id: "register-login-id-help", class: "field-help",
                    "3〜32文字。半角英数字から始め、. _ - も使えます。"
                }
                if let Some(message) = current_errors.login_id.as_deref() {
                    p { id: "register-login-id-error", class: "field-error", "{message}" }
                }
            }
            div { class: "field-group",
                label { r#for: "register-password", "password" }
                input {
                    id: "register-password",
                    name: "new-password",
                    r#type: "password",
                    autocomplete: "new-password",
                    required: true,
                    value: "{password}",
                    aria_invalid: current_errors.password.is_some(),
                    aria_describedby: if current_errors.password.is_some() { "register-password-help register-password-error" } else { "register-password-help" },
                    oninput: move |event| {
                        password.set(event.value());
                        errors.write().password = None;
                    },
                }
                p { id: "register-password-help", class: "field-help",
                    "15文字以上128文字以下。空白も文字として扱い、貼り付けとpassword managerを利用できます。"
                }
                if let Some(message) = current_errors.password.as_deref() {
                    p { id: "register-password-error", class: "field-error", "{message}" }
                }
            }
            div { class: "field-group",
                label { r#for: "register-password-confirmation", "password確認" }
                input {
                    id: "register-password-confirmation",
                    name: "new-password-confirmation",
                    r#type: "password",
                    autocomplete: "new-password",
                    required: true,
                    value: "{password_confirmation}",
                    aria_invalid: current_errors.password_confirmation.is_some(),
                    aria_describedby: if current_errors.password_confirmation.is_some() { "register-password-confirmation-error" } else { "" },
                    oninput: move |event| {
                        password_confirmation.set(event.value());
                        errors.write().password_confirmation = None;
                    },
                }
                if let Some(message) = current_errors.password_confirmation.as_deref() {
                    p { id: "register-password-confirmation-error", class: "field-error", "{message}" }
                }
            }
            p { class: "account-recovery-warning",
                "現在はpasswordの再設定を用意していません。passwordを失うと、このアカウントの履歴は復旧できません。"
            }
            p { class: "account-switch",
                "ログインなしでも日程調整できます。"
            }
            button {
                class: "primary-button account-submit",
                r#type: "submit",
                disabled: submitting(),
                aria_busy: submitting(),
                if submitting() { "作成中…" } else { "アカウントを作る" }
            }
        }
    }
}

/// Stateful login form with one generic credential failure.
#[component]
pub fn AccountLoginForm(initial_errors: AccountAuthErrors) -> Element {
    let router = try_router();
    let mut login_id = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut errors = use_signal(|| initial_errors);
    let mut submitting = use_signal(|| false);

    let current_errors = errors();
    rsx! {
        form {
            class: "account-form",
            novalidate: true,
            onsubmit: move |submit_event| {
                async move {
                    submit_event.prevent_default();
                    if submitting() {
                        return;
                    }
                    let input = AccountLoginInput {
                        login_id: login_id(),
                        password: password(),
                    };
                    if let Err(next_errors) = input.prepare() {
                        let focus_target = first_account_error_target(&next_errors, false);
                        errors.set(next_errors);
                        focus_element(focus_target).await;
                        return;
                    }

                    errors.set(AccountAuthErrors::default());
                    submitting.set(true);
                    match login_account(input).await {
                        Ok(_) => {
                            password.set(String::new());
                            if let Some(router) = router.as_ref() {
                                let _ = router.replace(crate::Route::History {});
                            }
                        }
                        Err(error) => {
                            errors.write().request = Some(account_login_failure_message(&error));
                            submitting.set(false);
                            focus_element_after_render("account-error-summary").await;
                        }
                    }
                }
            },

            if let Some(message) = current_errors.request.as_deref() {
                p { id: "account-error-summary", class: "form-error", role: "alert", tabindex: "-1", "{message}" }
            }
            div { class: "field-group",
                label { r#for: "login-id", "ログインID" }
                input {
                    id: "login-id",
                    name: "username",
                    r#type: "text",
                    autocomplete: "username",
                    maxlength: LOGIN_ID_MAX_CHARS,
                    required: true,
                    value: "{login_id}",
                    aria_invalid: current_errors.login_id.is_some(),
                    aria_describedby: if current_errors.login_id.is_some() { "login-id-error" } else { "" },
                    oninput: move |event| {
                        login_id.set(event.value());
                        errors.write().login_id = None;
                    },
                }
                if let Some(message) = current_errors.login_id.as_deref() {
                    p { id: "login-id-error", class: "field-error", "{message}" }
                }
            }
            div { class: "field-group",
                label { r#for: "login-password", "password" }
                input {
                    id: "login-password",
                    name: "password",
                    r#type: "password",
                    autocomplete: "current-password",
                    required: true,
                    value: "{password}",
                    aria_invalid: current_errors.password.is_some(),
                    aria_describedby: if current_errors.password.is_some() { "login-password-error" } else { "" },
                    oninput: move |event| {
                        password.set(event.value());
                        errors.write().password = None;
                    },
                }
                if let Some(message) = current_errors.password.as_deref() {
                    p { id: "login-password-error", class: "field-error", "{message}" }
                }
            }
            button {
                class: "primary-button account-submit",
                r#type: "submit",
                disabled: submitting(),
                aria_busy: submitting(),
                if submitting() { "ログイン中…" } else { "ログイン" }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AccountHistoryLoad {
    Loading,
    Loaded(AccountHistoryState),
    Failed,
}

/// Private-history route; only its loading shell is rendered before hydration.
#[component]
pub fn History() -> Element {
    rsx! {
        document::Title { "イベント履歴 | TSUNORU" }
        document::Meta { name: "robots", content: "noindex,nofollow" }
        AccountHistoryClient {}
    }
}

#[component]
fn AccountHistoryClient() -> Element {
    let mut load = use_signal(|| AccountHistoryLoad::Loading);
    use_effect(move || {
        spawn(async move {
            let next = match get_account_history().await {
                Ok(state) => AccountHistoryLoad::Loaded(state),
                Err(_) => AccountHistoryLoad::Failed,
            };
            load.set(next);
            focus_element_after_render("account-history-heading").await;
        });
    });

    match load() {
        AccountHistoryLoad::Loading => rsx! { AccountHistoryLoading {} },
        AccountHistoryLoad::Loaded(AccountHistoryState::Guest) => {
            rsx! { AccountHistoryGuest { session_expired: false } }
        }
        AccountHistoryLoad::Loaded(AccountHistoryState::Expired) => {
            rsx! { AccountHistoryGuest { session_expired: true } }
        }
        AccountHistoryLoad::Loaded(AccountHistoryState::Authenticated(history)) => {
            rsx! {
                AccountHistoryView {
                    history,
                    on_logged_out: move |_| {
                        load.set(AccountHistoryLoad::Loaded(AccountHistoryState::Guest));
                        spawn(async move {
                            focus_element_after_render("account-history-heading").await;
                        });
                    },
                }
            }
        }
        AccountHistoryLoad::Failed => rsx! { AccountHistoryFailure {} },
    }
}

/// Loading state that does not imply an empty or logged-out history.
#[component]
pub fn AccountHistoryLoading() -> Element {
    rsx! {
        main { class: "app-shell history-loading", aria_busy: "true",
            p { class: "loading", role: "status", aria_live: "polite",
                "履歴を読み込んでいます…"
            }
        }
    }
}

/// Logged-out state, optionally explaining that a previously supplied session expired.
#[component]
pub fn AccountHistoryGuest(session_expired: bool) -> Element {
    rsx! {
        main { class: "app-shell account-route",
            section { class: "account-page account-guest",
                a { class: "wordmark", href: "/", "TSUNORU" }
                p { class: "eyebrow", "任意の履歴" }
                h1 { id: "account-history-heading", tabindex: "-1", "あなたの履歴" }
                if session_expired {
                    p { class: "form-error", role: "status",
                        "セッションの有効期限が切れました。もう一度ログインしてください。"
                    }
                }
                p { class: "account-lead",
                    "ログイン中に作成・回答したイベントが、主催と参加に分かれて表示されます。過去の匿名利用は自動で取り込みません。"
                }
                div { class: "account-guest-actions",
                    a { class: "primary-button", href: "/login", "ログイン" }
                    a { class: "secondary-button", href: "/register", "アカウントを作る" }
                }
                a { class: "text-link", href: "/", "ログインせず日程をつのる" }
            }
        }
    }
}

/// Private history read failure with an explicit native retry.
#[component]
pub fn AccountHistoryFailure() -> Element {
    rsx! {
        main { class: "app-shell account-route",
            section { class: "account-page account-failure",
                a { class: "wordmark", href: "/", "TSUNORU" }
                h1 { id: "account-history-heading", tabindex: "-1", "履歴を読み込めませんでした" }
                p { role: "alert", "入力内容はありません。少し待ってから、もう一度お試しください。" }
                button {
                    class: "secondary-button",
                    r#type: "button",
                    onclick: move |_| {
                        document::eval("window.location.reload();");
                    },
                    "もう一度試す"
                }
            }
        }
    }
}

/// Two short role-specific lists for one account, deliberately linking only to public events.
#[component]
pub fn AccountHistoryView(
    history: AccountHistory,
    on_logged_out: Option<EventHandler<()>>,
) -> Element {
    let mut logging_out = use_signal(|| false);
    let mut logout_error = use_signal(|| None::<String>);
    rsx! {
        main { class: "app-shell",
            section { class: "history-page",
                header { class: "history-header",
                    div { class: "history-title",
                        a { class: "wordmark", href: "/", "TSUNORU" }
                        p { class: "eyebrow", "任意の履歴" }
                        h1 { id: "account-history-heading", tabindex: "-1", "あなたの履歴" }
                    }
                    div { class: "account-session-summary",
                        p { "ログイン中: " span { "{history.login_id}" } }
                        button {
                            id: "account-logout",
                            class: "secondary-button",
                            r#type: "button",
                            disabled: logging_out(),
                            aria_busy: logging_out(),
                            onclick: move |_| async move {
                                if logging_out() {
                                    return;
                                }
                                logout_error.set(None);
                                logging_out.set(true);
                                match logout_account().await {
                                    Ok(()) => {
                                        if let Some(on_logged_out) = on_logged_out {
                                            on_logged_out.call(());
                                        }
                                    }
                                    Err(_) => {
                                        logout_error.set(Some(
                                            "ログアウトできませんでした。もう一度お試しください。"
                                                .to_owned(),
                                        ));
                                        logging_out.set(false);
                                        focus_element_after_render("account-logout").await;
                                    }
                                }
                            },
                            if logging_out() { "ログアウト中…" } else { "ログアウト" }
                        }
                    }
                }
                if let Some(message) = logout_error().as_deref() {
                    p { class: "form-error", role: "status", "{message}" }
                }
                p { class: "history-explanation",
                    "ログイン中に作成・回答したイベントだけを表示します。回答の痕跡は一覧を広げず、当時の記録で確認できます。"
                }
                div { class: "history-grid",
                    section { class: "history-section", aria_labelledby: "organized-history-heading",
                        h2 { id: "organized-history-heading", "主催したイベント" }
                        if history.organized_series.is_empty() && history.organized_standalone.is_empty() {
                            p { class: "history-empty", "主催したイベントはまだありません。" }
                        } else {
                            if !history.organized_series.is_empty() {
                                section { class: "history-series-section", aria_labelledby: "organized-series-heading",
                                    h3 { id: "organized-series-heading", "継続している活動" }
                                    ul { class: "history-series-list",
                                        for (series_index, series) in history.organized_series.iter().enumerate() {
                                            li { key: "series-{series_index}-{series.series_name}",
                                                details { class: "history-series",
                                                    summary {
                                                        span { class: "history-series-name", "{series.series_name}" }
                                                        span { class: "history-series-count", "{series.events.len()}件" }
                                                    }
                                                    ul { class: "history-list history-series-events",
                                                        for item in &series.events {
                                                            OrganizedHistoryItem { item: item.clone() }
                                                        }
                                            }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if !history.organized_standalone.is_empty() {
                                section { class: "history-standalone-section", aria_labelledby: "organized-standalone-heading",
                                    h3 { id: "organized-standalone-heading", "その他の主催イベント" }
                                    ul { class: "history-list",
                                        for item in &history.organized_standalone {
                                            OrganizedHistoryItem { item: item.clone() }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    section { class: "history-section", aria_labelledby: "participated-history-heading",
                        h2 { id: "participated-history-heading", "参加したイベント" }
                        if history.participated.is_empty() {
                            p { class: "history-empty", "参加したイベントはまだありません。" }
                        } else {
                            ul { class: "history-list",
                                for item in &history.participated {
                                    li { key: "participated-{item.public_id}",
                                        a { href: "/history/events/{item.public_id}",
                                            h3 { "{item.name}" }
                                            p { class: "history-metadata", "回答済み" }
                                            if let Some(decision) = item.decision.as_ref() {
                                                p { class: "history-decision",
                                                    time { datetime: "{decision.local_date}T{decision.local_time}",
                                                        "{format_local_start(&decision.local_date, &decision.local_time)}"
                                                    }
                                                }
                                                p { class: "time-zone", "{item.time_zone} の時刻" }
                                            } else {
                                                p { class: "history-pending", "調整中" }
                                            }
                                            span { class: "history-detail-label", "当時の記録を見る" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                a { class: "primary-button history-create-link", href: "/", "新しい日程をつのる" }
            }
        }
    }
}

#[component]
fn OrganizedHistoryItem(item: crate::domain::OrganizedEventHistoryItem) -> Element {
    rsx! {
        li { key: "organized-{item.public_id}",
            a { href: "/history/events/{item.public_id}",
                h4 { "{item.name}" }
                if let Some(decision) = item.decision.as_ref() {
                    p { class: "history-decision",
                        time { datetime: "{decision.local_date}T{decision.local_time}",
                            "{format_local_start(&decision.local_date, &decision.local_time)}"
                        }
                    }
                    p { class: "time-zone", "{item.time_zone} の時刻" }
                } else {
                    p { class: "history-pending", "調整中" }
                }
                p { class: "history-metadata", "回答 {item.response_count}件" }
                span { class: "history-detail-label", "当時の記録を見る" }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AccountEventTraceLoad {
    Loading,
    Loaded(AccountEventTraceState),
    Missing,
    Failed,
}

/// Private event-trace route; SSR emits only the generic loading shell.
#[component]
pub fn HistoryEvent(public_id: String) -> Element {
    rsx! { AccountEventTraceRouteContent { key: "{public_id}", public_id } }
}

#[component]
fn AccountEventTraceRouteContent(public_id: String) -> Element {
    rsx! {
        document::Title { "日程調整の記録 | TSUNORU" }
        document::Meta { name: "robots", content: "noindex,nofollow" }
        AccountEventTraceClient { public_id }
    }
}

#[component]
fn AccountEventTraceClient(public_id: String) -> Element {
    let mut load = use_signal(|| AccountEventTraceLoad::Loading);
    let mut retry_epoch = use_signal(|| 0_u64);
    let mut request_epoch = use_signal(|| 0_u64);

    use_effect(use_reactive((&public_id,), move |(event_public_id,)| {
        let _retry_epoch = retry_epoch();
        let current_request = (*request_epoch.peek()).wrapping_add(1);
        request_epoch.set(current_request);
        load.set(AccountEventTraceLoad::Loading);
        spawn(async move {
            let input = AccountEventTraceInput { event_public_id };
            let next = match get_account_event_trace(input).await {
                Ok(state) => AccountEventTraceLoad::Loaded(state),
                Err(error) if error.account_status_code() == 404 => AccountEventTraceLoad::Missing,
                Err(_) => AccountEventTraceLoad::Failed,
            };
            if *request_epoch.peek() != current_request {
                return;
            }
            load.set(next);
            focus_element_after_render("account-event-trace-heading").await;
        });
    }));

    match load() {
        AccountEventTraceLoad::Loading => rsx! { AccountEventTraceLoading {} },
        AccountEventTraceLoad::Loaded(AccountEventTraceState::Guest) => {
            rsx! { AccountEventTraceGuest { session_expired: false } }
        }
        AccountEventTraceLoad::Loaded(AccountEventTraceState::Expired) => {
            rsx! { AccountEventTraceGuest { session_expired: true } }
        }
        AccountEventTraceLoad::Loaded(AccountEventTraceState::Authenticated(trace)) => {
            rsx! { AccountEventTraceView { trace } }
        }
        AccountEventTraceLoad::Missing => rsx! { AccountEventTraceMissing {} },
        AccountEventTraceLoad::Failed => rsx! {
            AccountEventTraceFailure {
                on_retry: move |_| {
                    retry_epoch.set(retry_epoch().wrapping_add(1));
                },
            }
        },
    }
}

/// Generic SSR-safe progress state for one private trace.
#[component]
pub fn AccountEventTraceLoading() -> Element {
    rsx! {
        main { class: "app-shell history-loading", aria_busy: "true",
            p { class: "loading", role: "status", aria_live: "polite",
                "イベントの記録を読み込んでいます…"
            }
        }
    }
}

/// Login guidance that does not retain previously loaded private trace data.
#[component]
pub fn AccountEventTraceGuest(session_expired: bool) -> Element {
    rsx! {
        main { class: "app-shell account-route",
            section { class: "account-page account-guest",
                a { class: "wordmark", href: "/", "TSUNORU" }
                p { class: "eyebrow", "日程調整の記録" }
                h1 { id: "account-event-trace-heading", tabindex: "-1", "ログインして記録を見る" }
                if session_expired {
                    p { class: "form-error", role: "status",
                        "セッションの有効期限が切れました。もう一度ログインしてください。"
                    }
                }
                p { class: "account-lead",
                    "この記録は、ログイン中に主催または回答したaccountだけが確認できます。"
                }
                div { class: "account-guest-actions",
                    a { class: "primary-button", href: "/login", "ログイン" }
                    a { class: "secondary-button", href: "/register", "アカウントを作る" }
                }
                a { class: "text-link", href: "/history", "履歴へ戻る" }
            }
        }
    }
}

/// One generic state for both absent events and accounts without a read relationship.
#[component]
pub fn AccountEventTraceMissing() -> Element {
    rsx! {
        main { class: "app-shell account-route",
            section { class: "account-page account-failure",
                a { class: "wordmark", href: "/", "TSUNORU" }
                p { class: "eyebrow", "日程調整の記録" }
                h1 { id: "account-event-trace-heading", tabindex: "-1", "記録が見つかりません" }
                p { "イベントが存在しないか、このaccountの履歴には結び付いていません。" }
                a { class: "secondary-button", href: "/history", "履歴へ戻る" }
            }
        }
    }
}

/// Recoverable private trace failure with an explicit retry action.
#[component]
pub fn AccountEventTraceFailure(on_retry: Option<EventHandler<()>>) -> Element {
    rsx! {
        main { class: "app-shell account-route",
            section { class: "account-page account-failure",
                a { class: "wordmark", href: "/", "TSUNORU" }
                p { class: "eyebrow", "日程調整の記録" }
                h1 { id: "account-event-trace-heading", tabindex: "-1", "記録を読み込めませんでした" }
                p { role: "alert", "少し待ってから、もう一度お試しください。" }
                button {
                    class: "secondary-button",
                    r#type: "button",
                    onclick: move |_| {
                        if let Some(on_retry) = on_retry {
                            on_retry.call(());
                        } else {
                            document::eval("window.location.reload();");
                        }
                    },
                    "もう一度読み込む"
                }
                a { class: "text-link", href: "/history", "履歴へ戻る" }
            }
        }
    }
}

/// Read-only private detail assembled from facts already created during scheduling.
#[component]
pub fn AccountEventTraceView(trace: AccountEventTrace) -> Element {
    let own_responses = trace
        .responses
        .iter()
        .filter(|response| response.is_current_account)
        .cloned()
        .collect::<Vec<_>>();
    let other_responses = trace
        .responses
        .iter()
        .filter(|response| !response.is_current_account)
        .cloned()
        .collect::<Vec<_>>();
    let own_response_count = own_responses.len();
    let other_response_count = other_responses.len();
    let has_own_responses = !own_responses.is_empty();
    let can_see_event_responses = matches!(
        trace.relationship,
        AccountEventTraceRelationship::Organized
            | AccountEventTraceRelationship::OrganizedAndParticipated
    );
    let relationship_label = match trace.relationship {
        AccountEventTraceRelationship::Organized => "主催したイベント",
        AccountEventTraceRelationship::Participated => "回答したイベント",
        AccountEventTraceRelationship::OrganizedAndParticipated => "主催し、自分も回答したイベント",
    };

    rsx! {
        main { class: "app-shell",
            article { class: "history-trace-page", aria_labelledby: "account-event-trace-heading",
                header { class: "trace-header",
                    a { class: "wordmark", href: "/", "TSUNORU" }
                    a { class: "trace-back-link", href: "/history", "← 履歴へ" }
                    p { class: "eyebrow", "日程調整の記録" }
                    h1 { id: "account-event-trace-heading", tabindex: "-1", "{trace.name}" }
                    p { class: "trace-relationship", "{relationship_label}" }
                    if let Some(note) = trace.organizer_note.as_deref() {
                        blockquote { class: "trace-organizer-note", "{note}" }
                    }
                }

                section { class: "trace-decision", aria_labelledby: "trace-decision-heading",
                    h2 { id: "trace-decision-heading", "決まった日時" }
                    if let Some(decision) = trace.decision.as_ref() {
                        time { datetime: "{decision.local_date}T{decision.local_time}",
                            "{format_local_start(&decision.local_date, &decision.local_time)}"
                        }
                        p { class: "time-zone", "{trace.time_zone} の時刻" }
                    } else {
                        p { class: "trace-pending", "まだ調整中です" }
                    }
                }

                if has_own_responses {
                    section { class: "trace-response-section", aria_labelledby: "own-trace-heading",
                        h2 { id: "own-trace-heading", "あなたが送った回答" }
                        div { class: "trace-response-list",
                            for (index, response) in own_responses.into_iter().enumerate() {
                                AccountEventTraceResponseView {
                                    candidates: trace.candidates.clone(),
                                    response,
                                    expanded: true,
                                    ordinal: (own_response_count > 1)
                                        .then_some((index + 1, own_response_count)),
                                }
                            }
                        }
                    }
                }

                if can_see_event_responses {
                    section { class: "trace-response-section", aria_labelledby: "other-trace-heading",
                        h2 { id: "other-trace-heading",
                            if has_own_responses { "ほかに届いた回答" } else { "届いた回答" }
                        }
                        if other_responses.is_empty() {
                            p { class: "trace-empty", role: "status",
                                if trace.responses.is_empty() {
                                    "このイベントには回答がありません"
                                } else {
                                    "ほかの回答はありません"
                                }
                            }
                        } else {
                            div { class: "trace-response-list",
                                for (index, response) in other_responses.into_iter().enumerate() {
                                    AccountEventTraceResponseView {
                                        candidates: trace.candidates.clone(),
                                        response,
                                        expanded: false,
                                        ordinal: (other_response_count > 1)
                                            .then_some((index + 1, other_response_count)),
                                    }
                                }
                            }
                        }
                    }
                }

                footer { class: "trace-actions",
                    a { class: "primary-button", href: "/events/{trace.public_id}",
                        "共有ページを開く"
                    }
                    if can_see_event_responses {
                        a {
                            class: "secondary-button",
                            href: "/history/events/{trace.public_id}/continue",
                            "同じ活動の次回をつのる"
                        }
                    }
                    a { class: "secondary-button", href: "/history", "履歴へ戻る" }
                }
            }
        }
    }
}

#[component]
fn AccountEventTraceResponseView(
    candidates: Vec<AccountEventTraceCandidate>,
    response: AccountEventTraceResponse,
    expanded: bool,
    ordinal: Option<(usize, usize)>,
) -> Element {
    let comment_summary = if response.comment.is_some() {
        "ひとことあり"
    } else {
        "ひとことなし"
    };
    rsx! {
        details { class: "trace-response", open: expanded,
            summary {
                span { class: "trace-respondent-name", "{response.respondent_name}" }
                if response.is_current_account {
                    span { class: "trace-own-label", "あなた" }
                }
                if let Some((current, total)) = ordinal {
                    span { class: "trace-response-ordinal", "回答 {current} / {total}" }
                }
                span { class: "trace-comment-summary", "{comment_summary}" }
            }
            dl { class: "trace-answers",
                for (index, (candidate, availability)) in candidates
                    .iter()
                    .zip(response.availabilities.iter())
                    .enumerate()
                {
                    div { class: "trace-answer", key: "trace-answer-{index}",
                        dt { "{format_local_start(&candidate.local_date, &candidate.local_time)}" }
                        dd { "{availability_trace_label(*availability)}" }
                    }
                }
            }
            if let Some(comment) = response.comment.as_deref() {
                blockquote { class: "trace-comment", "{comment}" }
            } else {
                p { class: "trace-comment trace-comment-empty", "ひとことなし" }
            }
        }
    }
}

fn availability_trace_label(availability: Availability) -> &'static str {
    match availability {
        Availability::Available => "○ 行ける",
        Availability::Maybe => "△ 条件次第",
        Availability::Unavailable => "× 難しい",
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AccountEventContinuationLoad {
    Loading,
    Loaded(AccountEventContinuationState),
    Missing,
    Failed,
}

/// Account-private continuation route; SSR emits only its generic loading shell.
#[component]
pub fn ContinueHistoryEvent(public_id: String) -> Element {
    rsx! { AccountEventContinuationRouteContent { key: "{public_id}", public_id } }
}

#[component]
fn AccountEventContinuationRouteContent(public_id: String) -> Element {
    rsx! {
        document::Title { "同じ活動の次回をつのる | TSUNORU" }
        document::Meta { name: "robots", content: "noindex,nofollow" }
        AccountEventContinuationClient { public_id }
    }
}

#[component]
fn AccountEventContinuationClient(public_id: String) -> Element {
    let mut load = use_signal(|| AccountEventContinuationLoad::Loading);
    let mut retry_epoch = use_signal(|| 0_u64);
    let mut request_epoch = use_signal(|| 0_u64);

    use_effect(use_reactive((&public_id,), move |(event_public_id,)| {
        let _retry_epoch = retry_epoch();
        let current_request = (*request_epoch.peek()).wrapping_add(1);
        request_epoch.set(current_request);
        load.set(AccountEventContinuationLoad::Loading);
        spawn(async move {
            let input = EventContinuationPlanInput {
                origin_event_public_id: event_public_id,
            };
            let next = match get_account_event_continuation_plan(input).await {
                Ok(state) => AccountEventContinuationLoad::Loaded(state),
                Err(error) if error.account_status_code() == 404 => {
                    AccountEventContinuationLoad::Missing
                }
                Err(_) => AccountEventContinuationLoad::Failed,
            };
            if *request_epoch.peek() != current_request {
                return;
            }
            load.set(next);
            focus_element_after_render("account-event-continuation-heading").await;
        });
    }));

    match load() {
        AccountEventContinuationLoad::Loading => {
            rsx! { AccountEventContinuationLoading {} }
        }
        AccountEventContinuationLoad::Loaded(AccountEventContinuationState::Guest) => {
            rsx! { AccountEventContinuationGuest { session_expired: false } }
        }
        AccountEventContinuationLoad::Loaded(AccountEventContinuationState::Expired) => {
            rsx! { AccountEventContinuationGuest { session_expired: true } }
        }
        AccountEventContinuationLoad::Loaded(AccountEventContinuationState::Authenticated(
            plan,
        )) => {
            rsx! { EventContinuationView { key: "{plan.origin_event_public_id}", plan } }
        }
        AccountEventContinuationLoad::Missing => rsx! { AccountEventContinuationMissing {} },
        AccountEventContinuationLoad::Failed => rsx! {
            AccountEventContinuationFailure {
                on_retry: move |_| retry_epoch.set(retry_epoch().wrapping_add(1)),
            }
        },
    }
}

/// Generic progress state that contains no private continuation facts during SSR.
#[component]
pub fn AccountEventContinuationLoading() -> Element {
    rsx! {
        main { class: "app-shell continuation-loading", aria_busy: "true",
            p { class: "loading", role: "status", aria_live: "polite",
                "続きのイベントを読み込んでいます…"
            }
        }
    }
}

/// Login guidance that distinguishes a missing session from an expired one.
#[component]
pub fn AccountEventContinuationGuest(session_expired: bool) -> Element {
    rsx! {
        main { class: "app-shell account-route",
            section { class: "account-page account-guest",
                a { class: "wordmark", href: "/", "TSUNORU" }
                p { class: "eyebrow", "継続している活動" }
                h1 { id: "account-event-continuation-heading", tabindex: "-1",
                    "ログインして次回をつのる"
                }
                if session_expired {
                    p { class: "form-error", role: "status",
                        "セッションの有効期限が切れました。もう一度ログインしてください。"
                    }
                }
                p { class: "account-lead",
                    "同じ活動の次回は、主催したaccountだけが作成できます。"
                }
                a { class: "primary-button", href: "/login", "ログイン" }
                a { class: "text-link", href: "/history", "履歴へ戻る" }
            }
        }
    }
}

/// One generic state for missing events and accounts without organizer ownership.
#[component]
pub fn AccountEventContinuationMissing() -> Element {
    rsx! {
        main { class: "app-shell account-route",
            section { class: "account-page account-failure",
                a { class: "wordmark", href: "/", "TSUNORU" }
                h1 { id: "account-event-continuation-heading", tabindex: "-1",
                    "続きのイベントが見つかりません"
                }
                p { "イベントが存在しないか、このaccountが主催した履歴ではありません。" }
                a { class: "secondary-button", href: "/history", "履歴へ戻る" }
            }
        }
    }
}

/// Recoverable continuation-plan failure without private transport details.
#[component]
pub fn AccountEventContinuationFailure(on_retry: Option<EventHandler<()>>) -> Element {
    rsx! {
        main { class: "app-shell account-route",
            section { class: "account-page account-failure",
                a { class: "wordmark", href: "/", "TSUNORU" }
                h1 { id: "account-event-continuation-heading", tabindex: "-1",
                    "続きのイベントを読み込めませんでした"
                }
                p { role: "alert", "少し待ってから、もう一度お試しください。" }
                button {
                    class: "secondary-button",
                    r#type: "button",
                    onclick: move |_| {
                        if let Some(on_retry) = on_retry {
                            on_retry.call(());
                        } else {
                            document::eval("window.location.reload();");
                        }
                    },
                    "もう一度試す"
                }
                a { class: "text-link", href: "/history", "履歴へ戻る" }
            }
        }
    }
}

/// Stable public guidance for private continuation-create failures.
pub fn event_continuation_failure_message(error: &impl AccountFailureStatus) -> String {
    match error.account_status_code() {
        401 | 403 => {
            "ログインの有効期限を確認できませんでした。入力内容は残っています。もう一度ログインしてください。"
                .to_owned()
        }
        409 => {
            "別の次回イベントが先に作成されました。入力内容は残っています。最新の続き情報を読み直してください。"
                .to_owned()
        }
        _ => {
            "次回イベントを保存できませんでした。入力内容は残っています。もう一度お試しください。"
                .to_owned()
        }
    }
}

fn first_account_error_target(errors: &AccountAuthErrors, registration: bool) -> &'static str {
    if errors.login_id.is_some() {
        if registration {
            "register-login-id"
        } else {
            "login-id"
        }
    } else if errors.password.is_some() {
        if registration {
            "register-password"
        } else {
            "login-password"
        }
    } else if errors.password_confirmation.is_some() {
        "register-password-confirmation"
    } else {
        "account-error-summary"
    }
}

fn first_continuation_error_target(errors: &EventCreationErrors) -> &'static str {
    if errors.name.is_some() {
        "continuation-event-name"
    } else if errors.organizer_note.is_some() {
        "continuation-organizer-note"
    } else if errors.candidates.is_some() {
        "continuation-candidate-date"
    } else if errors.time_zone.is_some() {
        "continuation-time-zone-error"
    } else {
        "continuation-submit"
    }
}

/// Registration guidance without replaying private transport or database details.
pub fn account_registration_failure_message(error: &impl AccountFailureStatus) -> String {
    match error.account_status_code() {
        409 => "このログインIDは使えません。別のログインIDをお試しください。".to_owned(),
        429 => "アカウント作成をしばらく試せません。時間を置いてお試しください。".to_owned(),
        _ => "アカウントを作成できませんでした。入力内容は残っています。もう一度お試しください。"
            .to_owned(),
    }
}

/// Login guidance that keeps generic credentials distinct from throttling and outages.
pub fn account_login_failure_message(error: &impl AccountFailureStatus) -> String {
    match error.account_status_code() {
        401 => "ログインIDまたはpasswordを確認してください。".to_owned(),
        429 => "ログインをしばらく試せません。時間を置いてお試しください。".to_owned(),
        _ => {
            "ログインできませんでした。入力内容は残っています。もう一度お試しください。".to_owned()
        }
    }
}

/// Status adapter shared by direct server-error tests and Dioxus client errors.
pub trait AccountFailureStatus {
    fn account_status_code(&self) -> u16;
}

impl AccountFailureStatus for ServerFnError {
    fn account_status_code(&self) -> u16 {
        match self {
            ServerFnError::ServerError { code, .. } => *code,
            _ => 500,
        }
    }
}

impl AccountFailureStatus for dioxus::CapturedError {
    fn account_status_code(&self) -> u16 {
        dioxus::fullstack::status_code_from_error(self).as_u16()
    }
}

/// Editable event form reached only after the organizer explicitly continues one private trace.
#[component]
pub fn EventContinuationView(plan: EventContinuationPlan) -> Element {
    let suggested_name = plan.suggested_event_name.clone().unwrap_or_default();
    let mut current_plan = use_signal(|| plan);
    let mut name = use_signal(|| suggested_name);
    let mut organizer_note = use_signal(String::new);
    let candidate_date = use_signal(String::new);
    let candidate_time = use_signal(|| DEFAULT_CANDIDATE_TIME.to_owned());
    let candidates = use_signal(Vec::<CandidateInput>::new);
    let mut time_zone = use_signal(String::new);
    let mut errors = use_signal(EventCreationErrors::default);
    let mut submit_error = use_signal(String::new);
    let mut plan_status = use_signal(String::new);
    let mut stale_plan = use_signal(|| false);
    let mut latest_suggestion_state = use_signal(|| None::<String>);
    let mut login_required = use_signal(|| false);
    let mut submitting = use_signal(|| false);
    let mut refreshing_plan = use_signal(|| false);
    let mut creation = use_signal(|| None::<(PublicEvent, String, Option<String>)>);

    use_effect(move || {
        spawn(async move {
            if let Some(zone) = read_browser_value(
                "dioxus.send(Intl.DateTimeFormat().resolvedOptions().timeZone || '');",
            )
            .await
            .filter(|zone| !zone.is_empty())
            {
                time_zone.set(zone);
            }
        });
    });

    if let Some((event, share_url, organizer_recovery_key)) = creation() {
        return rsx! { CreationSuccess { event, share_url, organizer_recovery_key } };
    }

    let rendered_plan = current_plan();
    let origin_history_url = format!("/history/events/{}", rendered_plan.origin_event_public_id);
    let has_suggestion = rendered_plan.suggested_event_name.is_some();
    let rendered_latest_suggestion = latest_suggestion_state();
    let has_latest_suggestion = rendered_latest_suggestion.is_some();
    let current_errors = errors();

    rsx! {
        main { class: "app-shell",
            article {
                class: "event-continuation-page",
                aria_labelledby: "account-event-continuation-heading",
                header { class: "continuation-header",
                    a { class: "wordmark", href: "/", "TSUNORU" }
                    p { class: "eyebrow", "継続している活動" }
                    h1 { id: "account-event-continuation-heading", tabindex: "-1",
                        "同じ活動の次回をつのる"
                    }
                }

                section { class: "continuation-context", aria_labelledby: "continuation-context-heading",
                    h2 { id: "continuation-context-heading", "{rendered_plan.series_name}" }
                    p { "起点: {rendered_plan.origin_event_name}" }
                    if stale_plan() {
                        p {
                            "続きの情報が更新されています。入力中の名前は変更せず、最新の情報を読み直してください。"
                        }
                    } else if has_latest_suggestion {
                        p {
                            "最新の末尾名から作った候補を下に表示しています。入力中の名前は変更していません。"
                        }
                    } else if has_suggestion {
                        p {
                            "過去の末尾名から次回名の候補を入れました。名前を変更しても同じ活動としてまとまります。"
                        }
                    } else {
                        p {
                            "次回名は提案できませんでした。イベント名は自由に入力してください。"
                        }
                    }
                }

                nav { class: "continuation-exit-links", aria_label: "続きの作成をやめる",
                    a { class: "secondary-button", href: "{origin_history_url}", "起点の記録へ戻る" }
                    a { class: "secondary-button", href: "/",
                        "このイベントの続きにしないで通常作成へ"
                    }
                }

                form {
                    class: "creation-form continuation-form",
                    novalidate: true,
                    onsubmit: move |event| async move {
                        event.prevent_default();
                        if submitting() || refreshing_plan() {
                            return;
                        }

                        let draft = EventCreationDraft {
                            name: name(),
                            organizer_note: organizer_note(),
                            time_zone: time_zone(),
                            candidates: candidates(),
                            pending_candidate: CandidateInput {
                                local_date: candidate_date(),
                                local_time: candidate_time(),
                            },
                        };
                        let event_input = match draft.prepare() {
                            Ok(input) => input,
                            Err(next_errors) => {
                                let focus_target = first_continuation_error_target(&next_errors);
                                errors.set(next_errors);
                                submit_error.set(String::new());
                                plan_status.set(String::new());
                                focus_element_after_render(focus_target).await;
                                return;
                            }
                        };
                        let active_plan = current_plan();
                        let input = EventContinuationCreateInput {
                            origin_event_public_id: active_plan.origin_event_public_id.clone(),
                            expected_tail_event_public_id: active_plan.tail_event_public_id.clone(),
                            event: event_input,
                        };

                        errors.set(EventCreationErrors::default());
                        submit_error.set(String::new());
                        plan_status.set(String::new());
                        stale_plan.set(false);
                        login_required.set(false);
                        submitting.set(true);
                        match create_account_event_continuation(input).await {
                            Ok(CreatedEvent { event, organizer_capability }) => {
                                let share_path = format!("/events/{}", event.public_id);
                                let share_url = match browser_origin().await {
                                    Some(origin) => format!("{origin}{share_path}"),
                                    None => share_path,
                                };
                                let capability_stored = store_organizer_capability(
                                    &event.public_id,
                                    &organizer_capability,
                                )
                                .await;
                                let organizer_recovery_key =
                                    (!capability_stored).then_some(organizer_capability);
                                creation.set(Some((event, share_url, organizer_recovery_key)));
                            }
                            Err(error) => {
                                let status_code = error.account_status_code();
                                stale_plan.set(status_code == 409);
                                if status_code == 409 {
                                    latest_suggestion_state.set(None);
                                }
                                login_required.set(status_code == 401);
                                submit_error.set(event_continuation_failure_message(&error));
                                focus_element_after_render("continuation-submit-error").await;
                            }
                        }
                        submitting.set(false);
                    },

                    div { class: "field-group",
                        label { r#for: "continuation-event-name", "イベント名" }
                        input {
                            id: "continuation-event-name",
                            name: "event-name",
                            r#type: "text",
                            value: "{name}",
                            maxlength: EVENT_NAME_MAX_CHARS,
                            required: true,
                            aria_invalid: current_errors.name.is_some(),
                            aria_describedby: if current_errors.name.is_some() {
                                "continuation-event-name-error"
                            } else {
                                "continuation-name-help"
                            },
                            placeholder: "例：ベストユニゾン 夏回",
                            oninput: move |event| name.set(event.value()),
                        }
                        p { id: "continuation-name-help", class: "field-help",
                            "候補は編集・削除できます。同じ活動としての関係は名前とは別に保存されます。"
                        }
                        if let Some(message) = current_errors.name.as_deref() {
                            p {
                                id: "continuation-event-name-error",
                                class: "field-error",
                                role: "alert",
                                "{message}"
                            }
                        }
                    }

                    div { class: "field-group",
                        div { class: "label-line",
                            label { r#for: "continuation-organizer-note", "主催者のひとこと" }
                            span { class: "optional", "任意" }
                        }
                        textarea {
                            id: "continuation-organizer-note",
                            name: "organizer-note",
                            value: "{organizer_note}",
                            maxlength: ORGANIZER_NOTE_MAX_CHARS,
                            rows: 3,
                            aria_invalid: current_errors.organizer_note.is_some(),
                            aria_describedby: if current_errors.organizer_note.is_some() {
                                "continuation-organizer-note-error"
                            } else {
                                ""
                            },
                            placeholder: "例：今回も、みんなで集まりたいです",
                            oninput: move |event| organizer_note.set(event.value()),
                        }
                        if let Some(message) = current_errors.organizer_note.as_deref() {
                            p {
                                id: "continuation-organizer-note-error",
                                class: "field-error",
                                role: "alert",
                                "{message}"
                            }
                        }
                    }

                    CandidateDateTimePicker {
                        candidates,
                        candidate_date,
                        candidate_time,
                        errors,
                        id_prefix: "continuation-candidate".to_owned(),
                    }

                    if let Some(message) = current_errors.time_zone.as_deref() {
                        p {
                            id: "continuation-time-zone-error",
                            class: "form-error",
                            role: "alert",
                            tabindex: "-1",
                            "{message}"
                        }
                    }
                    if !submit_error().is_empty() {
                        div {
                            id: "continuation-submit-error",
                            class: "form-error continuation-submit-error",
                            role: "alert",
                            tabindex: "-1",
                            p { "{submit_error}" }
                            if login_required() {
                                a {
                                    class: "secondary-button continuation-login-link",
                                    href: "/login",
                                    target: "_blank",
                                    rel: "noopener",
                                    "別タブでログインする"
                                }
                                p { class: "field-help",
                                    "ログインできたらこのタブへ戻ってください。入力内容はこのタブに残っています。"
                                }
                            }
                            if stale_plan() {
                                button {
                                    class: "secondary-button",
                                    r#type: "button",
                                    disabled: refreshing_plan(),
                                    aria_busy: refreshing_plan(),
                                    onclick: move |_| async move {
                                        if refreshing_plan() {
                                            return;
                                        }
                                        refreshing_plan.set(true);
                                        plan_status.set(String::new());
                                        let origin_event_public_id =
                                            current_plan().origin_event_public_id.clone();
                                        let input = EventContinuationPlanInput {
                                            origin_event_public_id,
                                        };
                                        match get_account_event_continuation_plan(input).await {
                                            Ok(AccountEventContinuationState::Authenticated(latest)) => {
                                                let next_suggestion = latest.suggested_event_name.clone();
                                                current_plan.set(latest);
                                                stale_plan.set(false);
                                                login_required.set(false);
                                                latest_suggestion_state.set(next_suggestion.clone());
                                                submit_error.set(String::new());
                                                if next_suggestion.is_some() {
                                                    plan_status.set(String::new());
                                                    focus_element_after_render(
                                                        "continuation-latest-suggestion",
                                                    )
                                                    .await;
                                                } else {
                                                    plan_status.set(
                                                        "最新の続き情報を読み込みました。入力中の名前は変更していません。最新の末尾名からは新しい候補を提案できませんでした。"
                                                            .to_owned(),
                                                    );
                                                    focus_element_after_render(
                                                        "continuation-plan-status",
                                                    )
                                                    .await;
                                                }
                                            }
                                            Ok(AccountEventContinuationState::Guest)
                                            | Ok(AccountEventContinuationState::Expired) => {
                                                login_required.set(true);
                                                submit_error.set(
                                                    "ログインの有効期限が切れました。入力内容は残っています。"
                                                        .to_owned(),
                                                );
                                            }
                                            Err(error) => {
                                                login_required
                                                    .set(error.account_status_code() == 401);
                                                submit_error.set(
                                                    event_continuation_failure_message(&error),
                                                );
                                            }
                                        }
                                        refreshing_plan.set(false);
                                    },
                                    if refreshing_plan() {
                                        "読み込み中…"
                                    } else {
                                        "最新の続き情報を読み直す"
                                    }
                                }
                            }
                        }
                    }
                    if let Some(latest_suggestion) = rendered_latest_suggestion {
                        section {
                            id: "continuation-latest-suggestion",
                            class: "continuation-latest-suggestion",
                            aria_labelledby: "continuation-latest-suggestion-heading",
                            tabindex: "-1",
                            h3 {
                                id: "continuation-latest-suggestion-heading",
                                "最新の候補"
                            }
                            p { class: "continuation-latest-suggestion-value",
                                "{latest_suggestion}"
                            }
                            p {
                                "入力中の名前は変更していません。候補を使う場合だけ、次の操作を選んでください。"
                            }
                            button {
                                class: "secondary-button",
                                r#type: "button",
                                disabled: submitting() || refreshing_plan(),
                                onclick: move |_| {
                                    let latest_suggestion = latest_suggestion.clone();
                                    async move {
                                        name.set(latest_suggestion);
                                        latest_suggestion_state.set(None);
                                        errors.write().name = None;
                                        plan_status.set(
                                            "最新の候補をイベント名へ反映しました。".to_owned(),
                                        );
                                        focus_element_after_render("continuation-event-name").await;
                                    }
                                },
                                "最新の候補を使う"
                            }
                        }
                    }
                    if !plan_status().is_empty() {
                        p {
                            id: "continuation-plan-status",
                            class: "field-help",
                            role: "status",
                            tabindex: "-1",
                            "{plan_status}"
                        }
                    }

                    button {
                        id: "continuation-submit",
                        class: "primary-button continuation-submit",
                        r#type: "submit",
                        disabled: submitting() || refreshing_plan(),
                        if submitting() { "作成中…" } else { "同じ活動の次回を作る" }
                    }
                }
            }
        }
    }
}

/// Stateful anonymous event-creation form.
#[component]
pub fn EventCreationForm(initial_errors: EventCreationErrors) -> Element {
    let mut name = use_signal(String::new);
    let mut organizer_note = use_signal(String::new);
    let candidate_date = use_signal(String::new);
    let candidate_time = use_signal(|| DEFAULT_CANDIDATE_TIME.to_owned());
    let candidates = use_signal(Vec::<CandidateInput>::new);
    let mut time_zone = use_signal(String::new);
    let mut errors = use_signal(|| initial_errors);
    let mut submit_error = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    let mut creation = use_signal(|| None::<(PublicEvent, String, Option<String>)>);

    use_effect(move || {
        spawn(async move {
            if let Some(zone) = read_browser_value(
                "dioxus.send(Intl.DateTimeFormat().resolvedOptions().timeZone || '');",
            )
            .await
            .filter(|zone| !zone.is_empty())
            {
                time_zone.set(zone);
            }
        });
    });

    if let Some((event, share_url, organizer_recovery_key)) = creation() {
        return rsx! { CreationSuccess { event, share_url, organizer_recovery_key } };
    }

    let current_errors = errors();

    rsx! {
        form {
            class: "creation-form",
            novalidate: true,
            onsubmit: move |event| async move {
                event.prevent_default();
                if submitting() {
                    return;
                }

                let draft = EventCreationDraft {
                    name: name(),
                    organizer_note: organizer_note(),
                    time_zone: time_zone(),
                    candidates: candidates(),
                    pending_candidate: CandidateInput {
                        local_date: candidate_date(),
                        local_time: candidate_time(),
                    },
                };

                let input = match draft.prepare() {
                    Ok(input) => input,
                    Err(next_errors) => {
                        errors.set(next_errors);
                        submit_error.set(String::new());
                        return;
                    }
                };

                errors.set(EventCreationErrors::default());
                submit_error.set(String::new());
                submitting.set(true);

                match create_event(input).await {
                    Ok(CreatedEvent { event, organizer_capability }) => {
                        let share_path = format!("/events/{}", event.public_id);
                        let share_url = match browser_origin().await {
                            Some(origin) => format!("{origin}{share_path}"),
                            None => share_path,
                        };
                        let capability_stored =
                            store_organizer_capability(&event.public_id, &organizer_capability)
                                .await;
                        let organizer_recovery_key =
                            (!capability_stored).then_some(organizer_capability);
                        creation.set(Some((event, share_url, organizer_recovery_key)));
                    }
                    Err(error) => {
                        eprintln!("event creation request failed: {error}");
                        submit_error.set(
                            "イベントを保存できませんでした。入力を残しているので、もう一度お試しください。"
                                .to_owned(),
                        );
                    }
                }
                submitting.set(false);
            },

            div { class: "field-group",
                label { r#for: "event-name", "イベント名" }
                input {
                    id: "event-name",
                    name: "event-name",
                    r#type: "text",
                    value: "{name}",
                    maxlength: EVENT_NAME_MAX_CHARS,
                    required: true,
                    aria_invalid: current_errors.name.is_some(),
                    aria_describedby: if current_errors.name.is_some() { "event-name-error" } else { "" },
                    placeholder: "例：秋の餃子会",
                    oninput: move |event| name.set(event.value()),
                }
                if let Some(message) = current_errors.name.as_deref() {
                    p { id: "event-name-error", class: "field-error", role: "alert", "{message}" }
                }
            }

            div { class: "field-group",
                div { class: "label-line",
                    label { r#for: "organizer-note", "主催者のひとこと" }
                    span { class: "optional", "任意" }
                }
                textarea {
                    id: "organizer-note",
                    name: "organizer-note",
                    value: "{organizer_note}",
                    maxlength: ORGANIZER_NOTE_MAX_CHARS,
                    rows: 3,
                    aria_invalid: current_errors.organizer_note.is_some(),
                    aria_describedby: if current_errors.organizer_note.is_some() { "organizer-note-error" } else { "" },
                    placeholder: "例：久しぶりに、みんなで焼きたてを囲みたいです",
                    oninput: move |event| organizer_note.set(event.value()),
                }
                if let Some(message) = current_errors.organizer_note.as_deref() {
                    p { id: "organizer-note-error", class: "field-error", role: "alert", "{message}" }
                }
            }

            CandidateDateTimePicker {
                candidates,
                candidate_date,
                candidate_time,
                errors,
                id_prefix: "candidate".to_owned(),
            }

            if let Some(message) = current_errors.time_zone.as_deref() {
                p { class: "form-error", role: "alert", "{message}" }
            }
            if !submit_error().is_empty() {
                p { class: "form-error", role: "alert", "{submit_error}" }
            }

            button {
                class: "primary-button",
                r#type: "submit",
                disabled: submitting(),
                if submitting() { "作成中…" } else { "イベントを作る" }
            }
        }
    }
}

/// Confirmation shown after the server has persisted the event.
#[component]
pub fn CreationSuccess(
    event: PublicEvent,
    share_url: String,
    organizer_recovery_key: Option<String>,
) -> Element {
    let mut copy_status = use_signal(String::new);
    let mut recovery_copy_status = use_signal(String::new);
    let copy_value = share_url.clone();
    let recovery_copy_value = organizer_recovery_key.clone().unwrap_or_default();
    let organizer_summary_url = format!("{}/summary", share_url.trim_end_matches('/'));

    use_effect(move || {
        spawn(async move {
            let _ = read_browser_value(
                "document.getElementById('creation-success-heading')?.focus(); dioxus.send('focused');",
            )
            .await;
        });
    });

    rsx! {
        section {
            class: "success-card",
            aria_labelledby: "creation-success-heading",
            aria_live: "polite",
            p { class: "success-mark", aria_hidden: "true", "✓" }
            h2 { id: "creation-success-heading", tabindex: "-1", "イベントを作りました" }
            p { class: "success-event-name", "{event.name}" }
            p { class: "success-detail", "このURLを、都合を聞きたい人へ渡してください。" }

            div { class: "share-field",
                label { r#for: "share-url", "回答用の共有URL" }
                div { class: "share-controls",
                    input {
                        id: "share-url",
                        r#type: "url",
                        value: "{share_url}",
                        readonly: true,
                    }
                    button {
                        class: "secondary-button",
                        r#type: "button",
                        onclick: move |_| {
                            let value = copy_value.clone();
                            async move {
                                if copy_to_clipboard(&value).await {
                                    copy_status.set("URLをコピーしました。".to_owned());
                                } else {
                                    copy_status.set(
                                        "コピーできませんでした。URLを選択してコピーしてください。"
                                            .to_owned(),
                                    );
                                }
                            }
                        },
                        "URLをコピー"
                    }
                }
            }
            p { class: "copy-status", aria_live: "polite", "{copy_status}" }

            if let Some(recovery_key) = organizer_recovery_key.as_deref() {
                section { class: "recovery-panel", aria_labelledby: "recovery-heading",
                    h3 { id: "recovery-heading", "主催者用の復旧キー" }
                    p { class: "recovery-warning",
                        "このブラウザーに保存できませんでした。この画面を閉じる前に、復旧キーを安全な場所へ保存してください。"
                    }
                    label { r#for: "organizer-recovery-key", "主催者用の復旧キー" }
                    div { class: "share-controls",
                        input {
                            id: "organizer-recovery-key",
                            r#type: "text",
                            value: "{recovery_key}",
                            readonly: true,
                            autocomplete: "off",
                        }
                        button {
                            class: "secondary-button",
                            r#type: "button",
                            onclick: move |_| {
                                let value = recovery_copy_value.clone();
                                async move {
                                    if copy_to_clipboard(&value).await {
                                        recovery_copy_status
                                            .set("復旧キーをコピーしました。".to_owned());
                                    } else {
                                        recovery_copy_status.set(
                                            "コピーできませんでした。復旧キーを選択してコピーしてください。"
                                                .to_owned(),
                                        );
                                    }
                                }
                            },
                            "復旧キーをコピー"
                        }
                    }
                    p {
                        class: "copy-status",
                        aria_live: "polite",
                        "{recovery_copy_status}"
                    }
                }
            }
            nav { class: "success-actions", aria_label: "作成したイベントを開く",
                a { class: "primary-button link-button", href: "{share_url}", "共有URLを開く" }
                a {
                    class: "secondary-button link-button",
                    href: "{organizer_summary_url}",
                    "回答サマリーを見る（主催者用）"
                }
            }
        }
    }
}

/// Organizer-only route. SSR resolves only the public event boundary; private data waits for hydration.
#[component]
pub fn OrganizerSummary(public_id: String) -> Element {
    let query_id = public_id.clone();
    let event = use_server_future(move || {
        let public_id = query_id.clone();
        async move { get_public_event(public_id).await }
    })?;

    match event() {
        Some(Ok(Some(_))) => rsx! {
            document::Meta { name: "robots", content: "noindex, nofollow" }
            OrganizerSummaryClient { public_id }
        },
        Some(Ok(None)) => {
            mark_route_not_found();
            rsx! {
                document::Meta { name: "robots", content: "noindex, nofollow" }
                main { class: "app-shell",
                    section { class: "message-card",
                        a { class: "wordmark", href: "/", "TSUNORU" }
                        h1 { "イベントが見つかりません" }
                        p { "URLが途中で切れていないか確認してください。" }
                        a { class: "text-link", href: "/", "新しい日程をつのる" }
                    }
                }
            }
        }
        Some(Err(_)) => rsx! {
            document::Meta { name: "robots", content: "noindex, nofollow" }
            main { class: "app-shell",
                OrganizerSummaryFailure {
                    message: "イベントを読み込めませんでした。少し待ってから、もう一度開いてください。".to_owned(),
                }
            }
        },
        None => rsx! {
            document::Meta { name: "robots", content: "noindex, nofollow" }
            main { class: "app-shell",
                OrganizerSummaryLoading {}
            }
        },
    }
}

#[component]
fn OrganizerSummaryClient(public_id: String) -> Element {
    let mut summary = use_signal(|| None::<OrganizerEventSummary>);
    let mut gate = use_signal(|| OrganizerSummaryGate::Loading);
    let mut recovery_error = use_signal(|| None::<String>);
    let mut recovery_submitting = use_signal(|| false);
    let mut recovery_visible = use_signal(|| false);
    let mut refreshing = use_signal(|| false);
    let mut refresh_error = use_signal(|| None::<String>);
    let mut refresh_status = use_signal(String::new);
    let mut storage_notice = use_signal(|| None::<String>);
    let mut matrix_open = use_signal(|| false);
    let mut matrix = use_signal(|| None::<OrganizerResponseMatrix>);
    let mut matrix_error = use_signal(|| None::<String>);
    let mut summary_request_epoch = use_signal(|| 0_u64);
    let mut refresh_request_epoch = use_signal(|| None::<u64>);
    let mut decision_submitting = use_signal(|| false);
    let mut decision_error = use_signal(|| None::<String>);

    let matrix_public_id = public_id.clone();
    let mut matrix_action = use_action(move |_: ()| {
        let event_public_id = matrix_public_id.clone();
        async move {
            matrix_error.set(None);
            let Some(capability) = read_organizer_capability(&event_public_id).await else {
                matrix_open.set(false);
                matrix.set(None);
                recovery_error.set(None);
                recovery_visible.set(true);
                return Ok::<(), ServerFnError>(());
            };

            let request = OrganizerSummaryInput {
                event_public_id,
                organizer_capability: capability,
            };
            match get_organizer_response_matrix(request).await {
                Ok(loaded) => matrix.set(Some(loaded)),
                Err(error) if organizer_authority_was_rejected(&error) => {
                    matrix_open.set(false);
                    matrix.set(None);
                    recovery_error.set(Some(
                        "保存されている主催者用の復旧キーを確認できませんでした。".to_owned(),
                    ));
                    recovery_visible.set(true);
                }
                Err(_) => matrix_error.set(Some(
                    "集計表を読み込めませんでした。サマリーはそのまま残しています。".to_owned(),
                )),
            }

            Ok::<(), ServerFnError>(())
        }
    });

    let initial_public_id = public_id.clone();
    use_effect(move || {
        let request_epoch = begin_summary_request(&mut summary_request_epoch);
        let event_public_id = initial_public_id.clone();
        spawn(async move {
            let loaded = load_saved_organizer_summary(&event_public_id).await;
            if !summary_request_is_current(summary_request_epoch, request_epoch) {
                return;
            }
            match loaded {
                StoredOrganizerSummaryLoad::Loaded(loaded) => summary.set(Some(*loaded)),
                StoredOrganizerSummaryLoad::MissingCapability => {
                    recovery_error.set(None);
                    gate.set(OrganizerSummaryGate::Recovery);
                }
                StoredOrganizerSummaryLoad::RejectedCapability => {
                    recovery_error.set(Some(
                        "保存されている主催者用の復旧キーを確認できませんでした。"
                            .to_owned(),
                    ));
                    gate.set(OrganizerSummaryGate::Recovery);
                }
                StoredOrganizerSummaryLoad::Failed => gate.set(OrganizerSummaryGate::Failure(
                    "回答サマリーを読み込めませんでした。通信状態を確認して、もう一度お試しください。"
                        .to_owned(),
                )),
            }
        });
    });

    let recovery_public_id = public_id.clone();
    let recovery_callback = OrganizerRecoverySubmitCallback::from(move |capability: String| {
        if recovery_submitting() || decision_submitting() {
            return;
        }

        recovery_error.set(None);
        recovery_submitting.set(true);
        supersede_summary_refresh(refreshing, refresh_request_epoch);
        let request_epoch = begin_summary_request(&mut summary_request_epoch);
        let event_public_id = recovery_public_id.clone();
        spawn(async move {
            let request = OrganizerSummaryInput {
                event_public_id: event_public_id.clone(),
                organizer_capability: capability.clone(),
            };
            let loaded = get_organizer_event_summary(request).await;
            if !summary_request_is_current(summary_request_epoch, request_epoch) {
                recovery_submitting.set(false);
                return;
            }
            match loaded {
                Ok(loaded) => {
                    let stored = store_organizer_capability(&event_public_id, &capability).await;
                    if !summary_request_is_current(summary_request_epoch, request_epoch) {
                        recovery_submitting.set(false);
                        return;
                    }
                    storage_notice.set((!stored).then(|| {
                        "このブラウザーに復旧キーを保存できませんでした。次に開くときも復旧キーが必要です。"
                            .to_owned()
                    }));
                    refresh_error.set(None);
                    refresh_status.set(String::new());
                    decision_error.set(None);
                    recovery_visible.set(false);
                    recovery_submitting.set(false);
                    matrix_action.reset();
                    matrix.set(None);
                    matrix_error.set(None);
                    matrix_open.set(false);
                    summary.set(Some(loaded));
                    focus_element_after_render("organizer-summary-heading").await;
                }
                Err(error) => {
                    let message = if organizer_authority_was_rejected(&error) {
                        "主催者用の復旧キーを確認してください。"
                    } else {
                        "回答サマリーを読み込めませんでした。入力は残っています。もう一度お試しください。"
                    };
                    recovery_error.set(Some(message.to_owned()));
                    recovery_submitting.set(false);
                    if summary().is_some() {
                        recovery_visible.set(true);
                    } else {
                        gate.set(OrganizerSummaryGate::Recovery);
                    }
                    focus_element("organizer-recovery-key").await;
                }
            }
        });
    });

    let decision_public_id = public_id.clone();
    let decision_callback = OrganizerDecisionSubmitCallback::from(move |candidate_id: i64| {
        if decision_submitting() || recovery_submitting() {
            return;
        }

        decision_error.set(None);
        decision_submitting.set(true);
        supersede_summary_refresh(refreshing, refresh_request_epoch);
        let request_epoch = begin_summary_request(&mut summary_request_epoch);
        let event_public_id = decision_public_id.clone();
        spawn(async move {
            let Some(capability) = read_organizer_capability(&event_public_id).await else {
                if summary_request_is_current(summary_request_epoch, request_epoch) {
                    recovery_error.set(None);
                    recovery_visible.set(true);
                }
                decision_submitting.set(false);
                return;
            };

            let request = OrganizerDecisionInput {
                event_public_id: event_public_id.clone(),
                candidate_id,
                organizer_capability: capability,
            };
            let result = get_organizer_event_decision(request).await;
            if !summary_request_is_current(summary_request_epoch, request_epoch) {
                decision_submitting.set(false);
                return;
            }

            match result {
                Ok(decision) => {
                    if let Some(mut updated_summary) = summary() {
                        updated_summary.decision = Some(decision);
                        summary.set(Some(updated_summary));
                    }
                    decision_error.set(None);
                    decision_submitting.set(false);
                    focus_element_after_render("organizer-decision-heading").await;
                }
                Err(ServerFnError::ServerError { code: 409, .. }) => {
                    let loaded = load_saved_organizer_summary(&event_public_id).await;
                    if !summary_request_is_current(summary_request_epoch, request_epoch) {
                        decision_submitting.set(false);
                        return;
                    }

                    match loaded {
                        StoredOrganizerSummaryLoad::Loaded(loaded) => {
                            let has_decision = loaded.decision.is_some();
                            summary.set(Some(*loaded));
                            decision_error.set((!has_decision).then(|| {
                                "別の画面で確定された日程を読み込めませんでした。選択は残っています。"
                                    .to_owned()
                            }));
                            decision_submitting.set(false);
                            if has_decision {
                                focus_element_after_render("organizer-decision-heading").await;
                            }
                        }
                        StoredOrganizerSummaryLoad::MissingCapability => {
                            recovery_error.set(None);
                            recovery_visible.set(true);
                            decision_submitting.set(false);
                        }
                        StoredOrganizerSummaryLoad::RejectedCapability => {
                            recovery_error.set(Some(
                                "保存されている主催者用の復旧キーを確認できませんでした。"
                                    .to_owned(),
                            ));
                            recovery_visible.set(true);
                            decision_submitting.set(false);
                        }
                        StoredOrganizerSummaryLoad::Failed => {
                            decision_error.set(Some(
                                "別の画面で日程が確定されています。最新の結果を読み込めませんでしたが、選択は残っています。"
                                    .to_owned(),
                            ));
                            decision_submitting.set(false);
                        }
                    }
                }
                Err(ServerFnError::ServerError { code: 404, .. }) => {
                    recovery_error.set(Some(
                        "保存されている主催者用の復旧キーを確認できませんでした。".to_owned(),
                    ));
                    recovery_visible.set(true);
                    decision_submitting.set(false);
                }
                Err(_) => {
                    decision_error.set(Some(
                        "日程を確定できませんでした。選択は残っています。".to_owned(),
                    ));
                    decision_submitting.set(false);
                }
            }
        });
    });

    if let Some(current_summary) = summary() {
        let refresh_public_id = public_id.clone();
        let current_decision = current_summary.decision.clone();
        let decision_candidates = current_summary.candidates.clone();
        let decision_time_zone = current_summary.time_zone.clone();
        let decision_public_id = current_summary.public_id.clone();
        let decision_event_name = current_summary.name.clone();
        let matrix_retry = OrganizerResponseMatrixRetryCallback::from(move |_: ()| {
            matrix.set(None);
            matrix_error.set(None);
            matrix_action.call(());
        });
        return rsx! {
            main { class: "app-shell",
                div { class: "organizer-summary-route",
                    OrganizerSummaryView { summary: current_summary }
                    section { class: "response-matrix-section", aria_label: "回答ごとの集計表",
                        button {
                            class: "secondary-button response-matrix-toggle",
                            r#type: "button",
                            aria_expanded: matrix_open(),
                            aria_controls: "organizer-response-matrix",
                            onclick: move |_| {
                                let will_open = !matrix_open();
                                matrix_open.set(will_open);
                                if will_open
                                    && matrix().is_none()
                                    && matrix_error().is_none()
                                    && !matrix_action.pending()
                                {
                                    matrix_action.call(());
                                }
                            },
                            if matrix_open() {
                                "回答ごとの集計表を閉じる"
                            } else {
                                "回答ごとの集計表を見る"
                            }
                        }
                        if matrix_open() {
                            div { id: "organizer-response-matrix", class: "response-matrix-content",
                                if matrix_action.pending()
                                    || (matrix().is_none() && matrix_error().is_none())
                                {
                                    OrganizerResponseMatrixLoading {}
                                } else if let Some(loaded) = matrix() {
                                    OrganizerResponseMatrixView { matrix: loaded }
                                } else if let Some(message) = matrix_error() {
                                    OrganizerResponseMatrixFailure {
                                        message,
                                        on_retry: matrix_retry,
                                    }
                                }
                            }
                        }
                    }
                    if let Some(decision) = current_decision {
                        OrganizerDecidedEventHandoff {
                            public_id: decision_public_id,
                            event_name: decision_event_name,
                            decision,
                            time_zone: decision_time_zone.clone(),
                        }
                    } else {
                        OrganizerDecisionForm {
                            candidates: decision_candidates,
                            time_zone: decision_time_zone,
                            initial_selected_candidate_id: None,
                            initial_error: decision_error(),
                            submitting: decision_submitting(),
                            on_submit: decision_callback,
                        }
                    }
                    section { class: "summary-refresh-panel", aria_label: "回答サマリーの更新",
                        button {
                            class: "secondary-button",
                            r#type: "button",
                            disabled: refreshing()
                                || recovery_submitting()
                                || decision_submitting(),
                            onclick: move |_| {
                                if refreshing() || recovery_submitting() || decision_submitting() {
                                    return;
                                }

                                refreshing.set(true);
                                refresh_error.set(None);
                                refresh_status.set(String::new());
                                let request_epoch = begin_summary_request(&mut summary_request_epoch);
                                refresh_request_epoch.set(Some(request_epoch));
                                let event_public_id = refresh_public_id.clone();
                                spawn(async move {
                                    let loaded = load_saved_organizer_summary(&event_public_id).await;
                                    if !summary_request_is_current(
                                        summary_request_epoch,
                                        request_epoch,
                                    ) {
                                        finish_summary_refresh(
                                            refreshing,
                                            refresh_request_epoch,
                                            request_epoch,
                                        );
                                        return;
                                    }
                                    match loaded {
                                        StoredOrganizerSummaryLoad::Loaded(loaded) => {
                                            matrix_action.reset();
                                            matrix.set(None);
                                            matrix_error.set(None);
                                            matrix_open.set(false);
                                            decision_error.set(None);
                                            summary.set(Some(*loaded));
                                            refresh_status
                                                .set("最新の回答を読み込みました。".to_owned());
                                        }
                                        StoredOrganizerSummaryLoad::MissingCapability => {
                                            recovery_error.set(None);
                                            recovery_visible.set(true);
                                        }
                                        StoredOrganizerSummaryLoad::RejectedCapability => {
                                            recovery_error.set(Some(
                                                "保存されている主催者用の復旧キーを確認できませんでした。"
                                                    .to_owned(),
                                            ));
                                            recovery_visible.set(true);
                                        }
                                        StoredOrganizerSummaryLoad::Failed => {
                                            refresh_error.set(Some(
                                                "最新の回答を読み込めませんでした。表示中のサマリーはそのまま残しています。"
                                                    .to_owned(),
                                            ));
                                        }
                                    }
                                    finish_summary_refresh(
                                        refreshing,
                                        refresh_request_epoch,
                                        request_epoch,
                                    );
                                });
                            },
                            if refreshing() {
                                "読み込み中…"
                            } else {
                                "最新の回答を読み込む"
                            }
                        }
                        if let Some(message) = refresh_error().as_deref() {
                            p { class: "form-error", role: "alert", "{message}" }
                        }
                        if !refresh_status().is_empty() {
                            p { class: "summary-refresh-status", role: "status", "{refresh_status}" }
                        }
                        if let Some(message) = storage_notice().as_deref() {
                            p { class: "form-error", role: "status", "{message}" }
                        }
                    }
                    if recovery_visible() {
                        OrganizerRecoveryForm {
                            initial_error: recovery_error(),
                            submitting: recovery_submitting() || decision_submitting(),
                            on_submit: recovery_callback,
                        }
                    }
                }
            }
        };
    }

    match gate() {
        OrganizerSummaryGate::Loading => rsx! {
            main { class: "app-shell",
                OrganizerSummaryLoading {}
            }
        },
        OrganizerSummaryGate::Recovery => rsx! {
            main { class: "app-shell",
                OrganizerRecoveryForm {
                    initial_error: recovery_error(),
                    submitting: recovery_submitting(),
                    on_submit: recovery_callback,
                }
            }
        },
        OrganizerSummaryGate::Failure(message) => {
            let retry_public_id = public_id.clone();
            rsx! {
                main { class: "app-shell",
                    div { class: "organizer-summary-state-shell",
                        OrganizerSummaryFailure { message }
                        button {
                            class: "secondary-button",
                            r#type: "button",
                            onclick: move |_| {
                                gate.set(OrganizerSummaryGate::Loading);
                                let request_epoch = begin_summary_request(&mut summary_request_epoch);
                                let event_public_id = retry_public_id.clone();
                                spawn(async move {
                                    let loaded = load_saved_organizer_summary(&event_public_id).await;
                                    if !summary_request_is_current(
                                        summary_request_epoch,
                                        request_epoch,
                                    ) {
                                        return;
                                    }
                                    match loaded {
                                        StoredOrganizerSummaryLoad::Loaded(loaded) => {
                                            summary.set(Some(*loaded));
                                        }
                                        StoredOrganizerSummaryLoad::MissingCapability => {
                                            recovery_error.set(None);
                                            gate.set(OrganizerSummaryGate::Recovery);
                                        }
                                        StoredOrganizerSummaryLoad::RejectedCapability => {
                                            recovery_error.set(Some(
                                                "保存されている主催者用の復旧キーを確認できませんでした。"
                                                    .to_owned(),
                                            ));
                                            gate.set(OrganizerSummaryGate::Recovery);
                                        }
                                        StoredOrganizerSummaryLoad::Failed => {
                                            gate.set(OrganizerSummaryGate::Failure(
                                                "回答サマリーを読み込めませんでした。通信状態を確認して、もう一度お試しください。"
                                                    .to_owned(),
                                            ));
                                        }
                                    }
                                });
                            },
                            "もう一度読み込む"
                        }
                    }
                }
            }
        }
    }
}

/// The organizer-facing projection. It deliberately accepts no authority prop.
#[component]
pub fn OrganizerSummaryView(summary: OrganizerEventSummary) -> Element {
    use_effect(move || {
        spawn(async move {
            focus_element("organizer-summary-heading").await;
        });
    });

    let shown_comment_count = summary.comment_previews.len();
    rsx! {
        article {
            class: "organizer-summary-page",
            aria_labelledby: "organizer-summary-heading",
            header { class: "organizer-summary-header",
                a { class: "wordmark", href: "/", "TSUNORU" }
                p { class: "eyebrow", "主催者用" }
                h1 { id: "organizer-summary-heading", tabindex: "-1", "{summary.name}" }
                if let Some(note) = summary.organizer_note.as_deref() {
                    blockquote { class: "organizer-note", "{note}" }
                }
            }

            section { class: "summary-overview", aria_labelledby: "summary-overview-heading",
                div { class: "summary-overview-heading",
                    h2 { id: "summary-overview-heading", "回答サマリー" }
                    p { class: "summary-response-count", "{summary.response_count}件の回答" }
                }
                p { class: "time-zone", "{summary.time_zone} の時刻" }
                if summary.response_count == 0 {
                    p { class: "summary-empty", role: "status", "まだ回答は届いていません" }
                }

                div {
                    class: "summary-card-grid",
                    aria_label: "候補日時ごとの回答サマリー",
                    for candidate in summary.candidates.iter() {
                        {
                            let candidate_text = format_candidate_summary(candidate);
                            let available = format!("○ 行ける {}件", candidate.available_count);
                            let maybe = format!("△ 条件次第 {}件", candidate.maybe_count);
                            let unavailable = format!("× 難しい {}件", candidate.unavailable_count);
                            rsx! {
                                article {
                                    class: "summary-candidate-card",
                                    id: format!("summary-candidate-{}", candidate.id),
                                    h3 {
                                        time {
                                            datetime: "{candidate.local_date}T{candidate.local_time}",
                                            "{candidate_text}"
                                        }
                                    }
                                    if let Some(fact) = candidate.fact.as_ref() {
                                        p { class: "summary-fact", "{candidate_fact_label(fact)}" }
                                    }
                                    div { class: "summary-count-grid", aria_label: "{candidate_text}の回答件数",
                                        p { class: "summary-count summary-count-available", "{available}" }
                                        p { class: "summary-count summary-count-maybe", "{maybe}" }
                                        p { class: "summary-count summary-count-unavailable", "{unavailable}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if summary.comment_count > 0 {
                details { id: "summary-comments", class: "summary-comment-disclosure",
                    summary { "みんなから {summary.comment_count}件" }
                    p { class: "summary-comment-count",
                        "{summary.comment_count}件中{shown_comment_count}件を表示しています"
                    }
                    div { class: "summary-comment-list",
                        for (index, preview) in summary.comment_previews.iter().enumerate() {
                            article { class: "comment-preview", key: "comment-{index}",
                                p { class: "comment-preview-name", "{preview.respondent_name}" }
                                p { class: "comment-preview-body", "{preview.comment}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Explicit organizer choice. Authority remains in the parent submit callback.
#[component]
pub fn OrganizerDecisionForm(
    candidates: Vec<CandidateResponseSummary>,
    time_zone: String,
    initial_selected_candidate_id: Option<i64>,
    initial_error: Option<String>,
    submitting: bool,
    #[props(into)] on_submit: OrganizerDecisionSubmitCallback,
) -> Element {
    let mut selected_candidate_id = use_signal(|| initial_selected_candidate_id);
    let selected = selected_candidate_id();
    let selected_candidate = selected.and_then(|candidate_id| {
        candidates
            .iter()
            .find(|candidate| candidate.id == candidate_id)
    });
    let selected_text = selected_candidate
        .map(format_candidate_summary)
        .unwrap_or_default();
    let described_by = if initial_error.is_some() {
        "organizer-decision-help organizer-decision-error"
    } else {
        "organizer-decision-help"
    };

    rsx! {
        section { class: "organizer-decision-section", aria_labelledby: "organizer-decision-form-heading",
            h2 { id: "organizer-decision-form-heading", "日程を確定する" }
            p { id: "organizer-decision-help", class: "field-help organizer-decision-help",
                "候補を一つ選び、選択中の日時を確認してから確定してください。一度確定すると、この画面では変更できません。"
            }
            form {
                class: "organizer-decision-form",
                aria_busy: submitting,
                onsubmit: move |submit_event| {
                    submit_event.prevent_default();
                    if submitting {
                        return;
                    }
                    if let Some(candidate_id) = selected_candidate_id() {
                        on_submit.call(candidate_id);
                    }
                },
                fieldset {
                    class: "organizer-decision-fieldset",
                    disabled: submitting,
                    aria_describedby: "{described_by}",
                    legend { "候補日時から一つ選ぶ" }
                    p { class: "time-zone", "{time_zone} の時刻" }
                    div { class: "organizer-decision-options",
                        for candidate in candidates.iter().cloned() {
                            {
                                let candidate_id = candidate.id;
                                let input_id = format!(
                                    "organizer-decision-candidate-{}",
                                    candidate_id,
                                );
                                let candidate_text = format_candidate_summary(&candidate);
                                rsx! {
                                    div { class: "organizer-decision-option",
                                        input {
                                            class: "organizer-decision-radio",
                                            id: "{input_id}",
                                            name: "organizer-decision-candidate",
                                            r#type: "radio",
                                            value: "{candidate_id}",
                                            checked: selected == Some(candidate_id),
                                            required: true,
                                            onchange: move |_| {
                                                selected_candidate_id.set(Some(candidate_id));
                                            },
                                        }
                                        label {
                                            class: "organizer-decision-option-label",
                                            r#for: "{input_id}",
                                            time {
                                                datetime: "{candidate.local_date}T{candidate.local_time}",
                                                "{candidate_text}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(message) = initial_error.as_deref() {
                    p { id: "organizer-decision-error", class: "form-error", role: "alert",
                        "{message}"
                    }
                }
                if selected_candidate.is_some() {
                    p {
                        id: "organizer-decision-selection",
                        class: "organizer-decision-selection",
                        role: "status",
                        "選択中: {selected_text}"
                    }
                }
                button {
                    id: "organizer-decision-submit",
                    class: "primary-button organizer-decision-submit",
                    r#type: "submit",
                    disabled: submitting || selected.is_none(),
                    if submitting {
                        "確定中…"
                    } else if initial_error.is_some() {
                        "もう一度確定する"
                    } else {
                        "この日時に確定する"
                    }
                }
            }
        }
    }
}

/// Organizer-only immutable result after the first decision has been committed.
#[component]
pub fn OrganizerDecisionView(decision: OrganizerEventDecision, time_zone: String) -> Element {
    use_effect(move || {
        spawn(async move {
            focus_element("organizer-decision-heading").await;
        });
    });

    let decided_start = format_local_start(&decision.local_date, &decision.local_time);
    rsx! {
        section {
            class: "organizer-decision-section organizer-decision-result",
            role: "status",
            aria_labelledby: "organizer-decision-heading",
            h2 { id: "organizer-decision-heading", tabindex: "-1", "日程を確定しました" }
            p { class: "organizer-decision-result-date",
                time {
                    datetime: "{decision.local_date}T{decision.local_time}",
                    "{decided_start}"
                }
            }
            p { class: "time-zone", "{time_zone} の時刻" }
            p { class: "organizer-decision-result-note",
                "このイベントの日程は、上記の候補で確定しています。"
            }
        }
    }
}

/// Compose the immutable organizer result with the same public handoff actions participants see.
#[component]
pub fn OrganizerDecidedEventHandoff(
    public_id: String,
    event_name: String,
    decision: OrganizerEventDecision,
    time_zone: String,
) -> Element {
    rsx! {
        OrganizerDecisionView {
            decision,
            time_zone,
        }
        DecidedEventActions {
            public_id,
            event_name,
        }
    }
}

/// Native calendar download plus a truthful Web Share and clipboard fallback.
#[component]
pub fn DecidedEventActions(public_id: String, event_name: String) -> Element {
    let share_path = format!("/events/{public_id}");
    let calendar_path = format!("/api/events/{public_id}/calendar.ics");
    let download_name = format!("tsunoru-{public_id}.ics");
    let initial_share_path = share_path.clone();
    let mut share_url = use_signal(|| share_path);
    let mut share_state = use_signal(ShareActionState::default);

    use_effect(move || {
        let share_path = initial_share_path.clone();
        spawn(async move {
            if let Some(origin) = browser_origin().await {
                share_url.set(format!("{origin}{share_path}"));
            }
        });
    });

    let state = share_state();
    let share_label = match state {
        ShareActionState::ReadyToShare => "この予定を共有",
        ShareActionState::ReadyToCopy => "共有URLをコピー",
        ShareActionState::InProgress => "処理しています…",
        ShareActionState::ShareStarted => "もう一度共有",
        ShareActionState::UrlCopied | ShareActionState::ManualCopy => "もう一度コピー",
    };
    let feedback = match state {
        ShareActionState::ReadyToShare => "",
        ShareActionState::ReadyToCopy => {
            "共有を完了しませんでした。必要なら共有URLをコピーしてください。"
        }
        ShareActionState::InProgress => "端末の操作を待っています。",
        ShareActionState::ShareStarted => "共有操作を開始しました。",
        ShareActionState::UrlCopied => "共有URLをコピーしました。",
        ShareActionState::ManualCopy => "URLを選択してコピーしてください。",
    };
    let manual_copy = state == ShareActionState::ManualCopy;
    let action_in_progress = state == ShareActionState::InProgress;
    let action_event_name = event_name.clone();

    rsx! {
        section {
            class: "decided-event-actions",
            aria_labelledby: "decided-event-actions-heading",
            h3 { id: "decided-event-actions-heading", "次の行動" }
            div { class: "decided-event-actions-grid",
                a {
                    class: "primary-button link-button calendar-download-link",
                    href: "{calendar_path}",
                    download: "{download_name}",
                    r#type: "text/calendar",
                    "カレンダーに追加"
                }
                button {
                    class: "secondary-button decided-event-share-button",
                    r#type: "button",
                    disabled: action_in_progress,
                    aria_busy: action_in_progress,
                    onclick: move |_| {
                        let current = share_state();
                        let Some(in_progress) = begin_share_action(current) else {
                            return;
                        };
                        share_state.set(in_progress);
                        let current_url = share_url();
                        let script = if matches!(
                            current,
                            ShareActionState::ReadyToCopy
                                | ShareActionState::UrlCopied
                                | ShareActionState::ManualCopy
                        ) {
                            decided_event_copy_script(&current_url)
                        } else {
                            decided_event_share_script(&action_event_name, &current_url)
                        };

                        #[cfg(feature = "web")]
                        {
                            let mut evaluation = document::eval(&script);
                            spawn(async move {
                                let result = evaluation
                                    .recv::<String>()
                                    .await
                                    .unwrap_or_else(|_| "failed".to_owned());
                                let next = next_share_action_state(current, &result);
                                share_state.set(next);
                                if next == ShareActionState::ManualCopy {
                                    focus_and_select_element_after_render(
                                        "decided-event-share-url",
                                    )
                                    .await;
                                }
                            });
                        }

                        #[cfg(not(feature = "web"))]
                        let _ = script;
                    },
                    "{share_label}"
                }
            }
            p { class: "decided-event-actions-help",
                "端末により、calendarで開くかfileとして保存します。共有する内容はこのイベントの共有URLです。"
            }
            p {
                class: "decided-event-share-status",
                role: "status",
                aria_live: "polite",
                "{feedback}"
            }
            div { class: "decided-event-manual-copy", hidden: !manual_copy,
                label { r#for: "decided-event-share-url", "共有URL" }
                input {
                    id: "decided-event-share-url",
                    r#type: "url",
                    readonly: true,
                    value: "{share_url}",
                }
            }
        }
    }
}

/// Participant-facing context around the shared accessible response matrix.
#[component]
pub fn ParticipantResponseMatrixView(matrix: ParticipantResponseMatrix) -> Element {
    rsx! {
        section {
            class: "participant-response-matrix",
            aria_labelledby: "participant-response-matrix-heading",
            h2 { id: "participant-response-matrix-heading", "みんなの回答" }
            p { class: "participant-response-matrix-note",
                "送った時点の一覧です。あとから届いた回答は自動では増えません。"
            }
            OrganizerResponseMatrixView { matrix }
        }
    }
}

/// Inline loading state that leaves the already rendered summary in place.
#[component]
pub fn OrganizerResponseMatrixLoading() -> Element {
    rsx! {
        section {
            class: "response-matrix-state response-matrix-loading",
            role: "status",
            aria_busy: "true",
            aria_live: "polite",
            p { "集計表を読み込んでいます…" }
        }
    }
}

/// Inline matrix failure with an explicit retry that does not submit a surrounding form.
#[component]
pub fn OrganizerResponseMatrixFailure(
    message: String,
    #[props(into)] on_retry: OrganizerResponseMatrixRetryCallback,
) -> Element {
    rsx! {
        section { class: "response-matrix-state response-matrix-failure", role: "alert",
            p { "{message}" }
            button {
                class: "secondary-button",
                r#type: "button",
                onclick: move |_| on_retry.call(),
                "もう一度読み込む"
            }
        }
    }
}

/// Hydration-safe placeholder for the private organizer request.
#[component]
pub fn OrganizerSummaryLoading() -> Element {
    rsx! {
        section {
            class: "organizer-summary-state",
            role: "status",
            aria_busy: "true",
            aria_live: "polite",
            p { class: "loading", "回答サマリーを読み込んでいます…" }
        }
    }
}

/// Non-secret failure state used when no prior summary can be retained.
#[component]
pub fn OrganizerSummaryFailure(message: String) -> Element {
    use_effect(move || {
        spawn(async move {
            focus_element("organizer-summary-error-heading").await;
        });
    });

    rsx! {
        section {
            class: "organizer-summary-state organizer-summary-failure",
            role: "alert",
            aria_labelledby: "organizer-summary-error-heading",
            h1 { id: "organizer-summary-error-heading", tabindex: "-1", "{message}" }
        }
    }
}

/// Explicit recovery form. The raw capability exists only in this input and its submit callback.
#[component]
pub fn OrganizerRecoveryForm(
    initial_error: Option<String>,
    submitting: bool,
    #[props(into)] on_submit: OrganizerRecoverySubmitCallback,
) -> Element {
    let mut capability = use_signal(String::new);
    let mut validation_error = use_signal(|| None::<String>);
    let current_error = validation_error().or(initial_error);

    use_effect(move || {
        spawn(async move {
            focus_element("organizer-recovery-heading").await;
        });
    });

    rsx! {
        section { class: "organizer-recovery-card", aria_labelledby: "organizer-recovery-heading",
            h1 { id: "organizer-recovery-heading", tabindex: "-1", "主催者用の復旧キー" }
            p { class: "field-help", id: "organizer-recovery-help",
                "イベント作成時に保存した64文字の復旧キーを入力してください。"
            }
            form {
                class: "organizer-recovery-form",
                novalidate: true,
                onsubmit: move |submit_event| {
                    submit_event.prevent_default();
                    if submitting {
                        return;
                    }

                    let normalized = capability().trim().to_owned();
                    if !valid_organizer_capability(&normalized) {
                        validation_error.set(Some(
                            "主催者用の復旧キーは0〜9とa〜fからなる64文字で入力してください。"
                                .to_owned(),
                        ));
                        spawn(async move {
                            focus_element("organizer-recovery-key").await;
                        });
                        return;
                    }

                    validation_error.set(None);
                    on_submit.call(normalized);
                },
                div { class: "field-group",
                    label { r#for: "organizer-recovery-key", "主催者用の復旧キー" }
                    input {
                        id: "organizer-recovery-key",
                        name: "organizer-recovery-key",
                        r#type: "password",
                        autocomplete: "off",
                        value: "{capability}",
                        maxlength: 64,
                        required: true,
                        disabled: submitting,
                        aria_invalid: current_error.is_some(),
                        aria_describedby: if current_error.is_some() {
                            "organizer-recovery-error"
                        } else {
                            "organizer-recovery-help"
                        },
                        oninput: move |input_event| {
                            capability.set(input_event.value());
                            validation_error.set(None);
                        },
                    }
                    if let Some(message) = current_error.as_deref() {
                        p {
                            id: "organizer-recovery-error",
                            class: "field-error",
                            role: "alert",
                            "{message}"
                        }
                    }
                }
                button {
                    class: "primary-button",
                    r#type: "submit",
                    disabled: submitting,
                    if submitting { "確認中…" } else { "回答サマリーを開く" }
                }
            }
        }
    }
}

/// Route component that loads one event from its public identifier.
#[component]
pub fn SharedEvent(public_id: String) -> Element {
    let query_id = public_id.clone();
    let event = use_server_future(move || {
        let public_id = query_id.clone();
        async move { get_public_event(public_id).await }
    })?;

    match event() {
        Some(Ok(Some(event))) => rsx! {
            main { class: "app-shell",
                PublicEventView { event }
            }
        },
        Some(Ok(None)) => {
            mark_route_not_found();
            rsx! {
                document::Meta { name: "robots", content: "noindex, nofollow" }
                main { class: "app-shell",
                    section { class: "message-card",
                        a { class: "wordmark", href: "/", "TSUNORU" }
                        h1 { "イベントが見つかりません" }
                        p { "URLが途中で切れていないか確認してください。" }
                        a { class: "text-link", href: "/", "新しい日程をつのる" }
                    }
                }
            }
        }
        Some(Err(error)) => rsx! {
            document::Meta { name: "robots", content: "noindex, nofollow" }
            main { class: "app-shell",
                section { class: "message-card",
                    a { class: "wordmark", href: "/", "TSUNORU" }
                    h1 { "イベントを読み込めませんでした" }
                    p { "少し待ってから、もう一度開いてください。" }
                    p { class: "visually-hidden", "{error}" }
                }
            }
        },
        None => rsx! {
            document::Meta { name: "robots", content: "noindex, nofollow" }
            main { class: "app-shell",
                p { class: "loading", aria_live: "polite", "イベントを読み込んでいます…" }
            }
        },
    }
}

fn mark_route_not_found() {
    #[cfg(feature = "server")]
    dioxus::fullstack::FullstackContext::commit_http_status(
        dioxus::fullstack::StatusCode::NOT_FOUND,
        None,
    );
}

/// Public event details followed by the shortest anonymous answering path.
#[component]
pub fn PublicEventView(event: PublicEvent) -> Element {
    let decision = event.decision.clone();
    let eyebrow = if decision.is_some() {
        "決定した予定"
    } else {
        "届いた日程候補"
    };
    let response_event = event.clone();
    rsx! {
        document::Meta { name: "robots", content: "noindex, nofollow" }
        article { class: "public-event",
            a { class: "wordmark", href: "/", "TSUNORU" }
            p { class: "eyebrow", "{eyebrow}" }
            h1 { "{event.name}" }
            if let Some(note) = event.organizer_note.as_deref() {
                blockquote { class: "organizer-note", "{note}" }
            }
            if let Some(decision) = decision {
                PublicDecidedEvent {
                    public_id: event.public_id,
                    event_name: event.name,
                    time_zone: event.time_zone,
                    decision,
                }
            } else {
                AvailabilityResponseForm {
                    event: response_event,
                    initial_errors: AvailabilityResponseErrors::default(),
                }
            }
        }
    }
}

/// Final public result shown instead of accepting an answer that cannot affect the decision.
#[component]
pub fn PublicDecidedEvent(
    public_id: String,
    event_name: String,
    time_zone: String,
    decision: PublicEventDecision,
) -> Element {
    let decided_start = format_local_start(&decision.local_date, &decision.local_time);
    rsx! {
        section {
            class: "public-decision-result",
            aria_labelledby: "public-decision-heading",
            h2 { id: "public-decision-heading", "日程が決まりました" }
            p { class: "public-decision-date",
                time {
                    datetime: "{decision.local_date}T{decision.local_time}",
                    "{decided_start}"
                }
            }
            p { class: "time-zone", "{time_zone} の時刻" }
            p { class: "public-decision-note",
                "このイベントは上記の日時で確定しています。"
            }
            DecidedEventActions {
                public_id,
                event_name,
            }
        }
    }
}

/// Stateful response form that keeps the same capability and payload across a failed retry.
#[component]
pub fn AvailabilityResponseForm(
    event: PublicEvent,
    initial_errors: AvailabilityResponseErrors,
) -> Element {
    let mut respondent_name = use_signal(String::new);
    let mut availabilities = use_signal(Vec::<CandidateAvailabilityInput>::new);
    let mut errors = use_signal(|| initial_errors);
    let mut submit_error = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    let mut response_capability = use_signal(|| None::<String>);
    let mut submitted_matrix = use_signal(|| None::<ParticipantResponseMatrix>);
    let mut comment_submitting = use_signal(|| false);
    let mut comment_error = use_signal(|| None::<String>);
    let mut comment_outcome = use_signal(|| ResponseCommentOutcome::Pending);

    if let Some(matrix) = submitted_matrix() {
        let current_comment_error = comment_error();
        let current_comment_outcome = comment_outcome();
        let comment_event_public_id = event.public_id.clone();

        return rsx! {
            AvailabilityResponseSuccess {}
            ParticipantResponseMatrixView { matrix }
            if current_comment_outcome == ResponseCommentOutcome::Saved {
                ResponseCommentSuccess {}
            } else if current_comment_outcome == ResponseCommentOutcome::Skipped {
                ResponseCommentSkipped {}
            } else {
                ResponseCommentOffer {
                    initial_comment: String::new(),
                    initial_error: current_comment_error,
                    submitting: comment_submitting(),
                    on_submit: move |comment: String| {
                        let event_public_id = comment_event_public_id.clone();
                        spawn(async move {
                            if comment_submitting() {
                                return;
                            }

                            comment_error.set(None);
                            comment_submitting.set(true);
                            let Some(capability) = response_capability() else {
                                comment_error.set(Some(
                                    "この回答へひとことを追加できません。このまま画面を閉じてください。"
                                        .to_owned(),
                                ));
                                comment_submitting.set(false);
                                return;
                            };

                            let input = NewResponseCommentInput {
                                event_public_id,
                                response_capability: capability,
                                comment,
                            };
                            match submit_response_comment(input).await {
                                Ok(()) => {
                                    response_capability.set(None);
                                    comment_submitting.set(false);
                                    comment_outcome.set(ResponseCommentOutcome::Saved);
                                }
                                Err(error) => {
                                    comment_error
                                        .set(Some(response_comment_failure_message(&error)));
                                    comment_submitting.set(false);
                                }
                            }
                        });
                    },
                    on_skip: move |_: ()| {
                        if comment_submitting() {
                            return;
                        }
                        response_capability.set(None);
                        comment_error.set(None);
                        comment_outcome.set(ResponseCommentOutcome::Skipped);
                    },
                }
            }
        };
    }

    let current_errors = errors();
    let current_availabilities = availabilities();
    let event_public_id = event.public_id.clone();
    let candidate_ids = event
        .candidates
        .iter()
        .map(|candidate| candidate.id)
        .collect::<Vec<_>>();

    rsx! {
        section { class: "response-section", aria_labelledby: "response-heading",
            div { class: "response-heading",
                p { class: "response-step", "1分で回答" }
                h2 { id: "response-heading", tabindex: "-1", "都合を教えてください" }
                p { class: "availability-meaning", id: "availability-help",
                    "すべての候補を一つずつ選んでください。○は行ける、△は条件次第・たぶん行ける、×は難しいです。"
                }
            }

            form {
                class: "availability-form",
                novalidate: true,
                onsubmit: move |submit_event| {
                    let event_public_id = event_public_id.clone();
                    let candidate_ids = candidate_ids.clone();
                    async move {
                        submit_event.prevent_default();
                        if submitting() {
                            return;
                        }

                        let response = match (AvailabilityResponseDraft {
                            respondent_name: respondent_name(),
                            candidate_ids,
                            availabilities: availabilities(),
                        })
                        .prepare()
                        {
                            Ok(response) => response,
                            Err(next_errors) => {
                                let focus_target = first_response_error_target(&next_errors);
                                errors.set(next_errors);
                                submit_error.set(String::new());
                                focus_element(&focus_target).await;
                                return;
                            }
                        };

                        errors.set(AvailabilityResponseErrors::default());
                        submit_error.set(String::new());
                        submitting.set(true);

                        let capability = match response_capability() {
                            Some(capability) => capability,
                            None => match generate_response_capability().await {
                                Some(capability) => {
                                    response_capability.set(Some(capability.clone()));
                                    capability
                                }
                                None => {
                                    submit_error.set(
                                        "回答の送信準備に失敗しました。再読み込みしてお試しください。"
                                            .to_owned(),
                                    );
                                    submitting.set(false);
                                    return;
                                }
                            },
                        };

                        let input = NewAvailabilityResponseInput {
                            event_public_id,
                            response_capability: capability,
                            response,
                        };
                        match submit_availability_response(input).await {
                            Ok(matrix) => submitted_matrix.set(Some(matrix)),
                            Err(_) => {
                                submit_error.set(
                                    "回答を保存できませんでした。入力は残っています。もう一度お試しください。"
                                        .to_owned(),
                                );
                                submitting.set(false);
                            }
                        }
                    }
                },

                if !current_errors.is_empty() {
                    p { id: "response-error-summary", class: "form-error", role: "alert",
                        "入力内容を確認してください。"
                    }
                }

                div { class: "field-group",
                    label { r#for: "respondent-name", "あなたの名前" }
                    input {
                        id: "respondent-name",
                        name: "respondent-name",
                        r#type: "text",
                        autocomplete: "name",
                        value: "{respondent_name}",
                        maxlength: RESPONDENT_NAME_MAX_CHARS,
                        required: true,
                        aria_invalid: current_errors.respondent_name.is_some(),
                        aria_describedby: if current_errors.respondent_name.is_some() {
                            "respondent-name-error"
                        } else {
                            ""
                        },
                        placeholder: "例：ミナ",
                        oninput: move |input_event| {
                            respondent_name.set(input_event.value());
                            response_capability.set(None);
                            submit_error.set(String::new());
                            let mut current = errors.write();
                            current.respondent_name = None;
                            current.request = None;
                        },
                    }
                    if let Some(message) = current_errors.respondent_name.as_deref() {
                        p { id: "respondent-name-error", class: "field-error", "{message}" }
                    }
                }

                div { class: "availability-candidate-list",
                    for candidate in event.candidates.iter().cloned() {
                        AvailabilityCandidateFieldset {
                            key: "{candidate.id}",
                            selected: current_availabilities
                                .iter()
                                .find(|choice| choice.candidate_id == candidate.id)
                                .map(|choice| choice.availability),
                            has_error: current_errors.candidate_ids.contains(&candidate.id),
                            candidate,
                            on_select: move |choice: CandidateAvailabilityInput| {
                                let candidate_id = choice.candidate_id;
                                let mut selections = availabilities.write();
                                if let Some(existing) = selections
                                    .iter_mut()
                                    .find(|existing| existing.candidate_id == candidate_id)
                                {
                                    existing.availability = choice.availability;
                                } else {
                                    selections.push(choice);
                                }
                                drop(selections);
                                response_capability.set(None);
                                submit_error.set(String::new());
                                let mut current = errors.write();
                                current.candidate_ids.retain(|id| *id != candidate_id);
                                current.request = None;
                            },
                        }
                    }
                }

                p { class: "time-zone", "{event.time_zone} の時刻" }
                if let Some(message) = current_errors.request.as_deref() {
                    p { id: "response-request-error", class: "field-error", "{message}" }
                }
                if !submit_error().is_empty() {
                    p { class: "form-error", role: "status", "{submit_error}" }
                }

                button {
                    class: "primary-button response-submit",
                    r#type: "submit",
                    disabled: submitting(),
                    if submitting() { "送信中…" } else { "回答を送る" }
                }
            }
        }
    }
}

#[component]
fn AvailabilityCandidateFieldset(
    candidate: PublicCandidate,
    selected: Option<Availability>,
    has_error: bool,
    on_select: EventHandler<CandidateAvailabilityInput>,
) -> Element {
    let candidate_text = format_public_candidate(&candidate);
    let error_id = format!("candidate-{}-error", candidate.id);
    let described_by = if has_error {
        format!("availability-help {error_id}")
    } else {
        "availability-help".to_owned()
    };

    rsx! {
        fieldset {
            class: "availability-candidate",
            aria_invalid: has_error,
            aria_describedby: "{described_by}",
            legend {
                time {
                    datetime: "{candidate.local_date}T{candidate.local_time}",
                    "{candidate_text}"
                }
            }
            div { class: "availability-options",
                for availability in [
                    Availability::Available,
                    Availability::Maybe,
                    Availability::Unavailable,
                ] {
                    {
                        let input_id = format!(
                            "availability-{}-{}",
                            candidate.id,
                            availability.storage_value(),
                        );
                        let input_name = format!("availability-{}", candidate.id);
                        rsx! {
                            input {
                                class: "availability-radio",
                                id: "{input_id}",
                                name: "{input_name}",
                                r#type: "radio",
                                value: availability.storage_value(),
                                checked: selected == Some(availability),
                                required: true,
                                aria_label: availability.accessible_label(),
                                aria_describedby: if has_error { "{error_id}" } else { "" },
                                onchange: move |_| on_select.call(CandidateAvailabilityInput {
                                    candidate_id: candidate.id,
                                    availability,
                                }),
                            }
                            label { class: "availability-option-label", r#for: "{input_id}",
                                span { class: "availability-symbol", aria_hidden: "true",
                                    "{availability.symbol()}"
                                }
                                span { "{availability.short_label()}" }
                            }
                        }
                    }
                }
            }
            if has_error {
                p { id: "{error_id}", class: "field-error candidate-response-error",
                    "{candidate_text} の都合を選んでください。"
                }
            }
        }
    }
}

/// Clear completion state shown only after an accepted response.
#[component]
pub fn AvailabilityResponseSuccess() -> Element {
    use_effect(move || {
        spawn(async move {
            focus_element("response-success-heading").await;
        });
    });

    rsx! {
        section { class: "response-success", aria_live: "polite",
            p { class: "success-mark", aria_hidden: "true", "✓" }
            h2 { id: "response-success-heading", tabindex: "-1", "回答を送りました" }
            p { class: "response-success-detail", "この画面は閉じて大丈夫です。" }
        }
    }
}

/// Optional, post-answer utterance UI. Authorization stays inside its parent callbacks.
#[component]
pub fn ResponseCommentOffer(
    initial_comment: String,
    initial_error: Option<String>,
    submitting: bool,
    #[props(into)] on_submit: ResponseCommentSubmitCallback,
    #[props(into)] on_skip: ResponseCommentSkipCallback,
) -> Element {
    let mut comment = use_signal(|| initial_comment);
    let mut validation_error = use_signal(|| None::<String>);
    let current_validation_error = validation_error();
    let current_error = current_validation_error.clone().or(initial_error);

    rsx! {
        section { class: "comment-offer", aria_labelledby: "response-comment-heading",
            div { class: "comment-offer-heading",
                div { class: "label-line",
                    h2 { id: "response-comment-heading", "ひとこと添える？" }
                    span { class: "optional", "任意" }
                }
                p { class: "comment-completion-note",
                    "ここまでで回答は完了しています。このまま閉じても大丈夫です。"
                }
            }

            p { id: "response-comment-help", class: "field-help",
                "短いひとことも歓迎です。例文は選んだあとに自由に直せます。"
            }
            div { class: "comment-suggestions", aria_label: "ひとことの例文",
                button {
                    class: "comment-suggestion",
                    r#type: "button",
                    disabled: submitting,
                    onclick: move |_| {
                        comment.set("調整ありがとう！".to_owned());
                        validation_error.set(None);
                        spawn(async move {
                            focus_element("response-comment").await;
                        });
                    },
                    "調整ありがとう！"
                }
                button {
                    class: "comment-suggestion",
                    r#type: "button",
                    disabled: submitting,
                    onclick: move |_| {
                        comment.set("楽しみ！".to_owned());
                        validation_error.set(None);
                        spawn(async move {
                            focus_element("response-comment").await;
                        });
                    },
                    "楽しみ！"
                }
            }

            form {
                class: "comment-form",
                novalidate: true,
                onsubmit: move |submit_event| {
                    submit_event.prevent_default();
                    if submitting {
                        return;
                    }

                    match (ResponseCommentDraft { comment: comment() }).prepare() {
                        Ok(prepared) => {
                            validation_error.set(None);
                            on_submit.call(prepared.comment);
                        }
                        Err(errors) => {
                            validation_error.set(errors.comment);
                            spawn(async move {
                                focus_element("response-comment").await;
                            });
                        }
                    }
                },
                div { class: "field-group",
                    label { r#for: "response-comment", "自由なひとこと" }
                    textarea {
                        id: "response-comment",
                        name: "response-comment",
                        value: "{comment}",
                        maxlength: RESPONDENT_COMMENT_MAX_CHARS,
                        rows: 3,
                        disabled: submitting,
                        aria_invalid: current_validation_error.is_some(),
                        aria_describedby: if current_error.is_some() {
                            "response-comment-error"
                        } else {
                            "response-comment-help"
                        },
                        placeholder: "例：肉！",
                        oninput: move |input_event| {
                            comment.set(input_event.value());
                            validation_error.set(None);
                        },
                    }
                    if let Some(message) = current_error.as_deref() {
                        p {
                            id: "response-comment-error",
                            class: "field-error",
                            role: "status",
                            "{message}"
                        }
                    }
                }
                div { class: "comment-actions",
                    button {
                        class: "primary-button",
                        r#type: "submit",
                        disabled: submitting,
                        if submitting { "送信中…" } else { "ひとことを送る" }
                    }
                    button {
                        class: "secondary-button",
                        r#type: "button",
                        disabled: submitting,
                        onclick: move |_| on_skip.call(),
                        "今回は送らない"
                    }
                }
            }
        }
    }
}

/// Confirmation shown after the optional comment has been accepted.
#[component]
pub fn ResponseCommentSuccess() -> Element {
    use_effect(move || {
        spawn(async move {
            focus_element("response-comment-success-heading").await;
        });
    });

    rsx! {
        section { class: "comment-outcome", aria_live: "polite",
            h2 { id: "response-comment-success-heading", tabindex: "-1",
                "ひとことも送りました"
            }
            p { "ありがとうございます。このまま画面を閉じて大丈夫です。" }
        }
    }
}

/// Translate transport failures without exposing server details or claiming a changed retry won.
pub fn response_comment_failure_message(error: &ServerFnError) -> String {
    match error {
        ServerFnError::ServerError { code: 409, .. } => {
            "先のひとことは送信済みです。変更した内容は保存されていません。回答は完了しています。"
                .to_owned()
        }
        _ => "ひとことを送れませんでした。内容は残っています。もう一度お試しください。".to_owned(),
    }
}

#[component]
fn ResponseCommentSkipped() -> Element {
    use_effect(move || {
        spawn(async move {
            focus_element("response-comment-skipped-heading").await;
        });
    });

    rsx! {
        section { class: "comment-outcome", aria_live: "polite",
            h2 { id: "response-comment-skipped-heading", tabindex: "-1",
                "ひとことは送らずに完了しました"
            }
            p { "回答は送信済みです。このまま画面を閉じて大丈夫です。" }
        }
    }
}

fn first_response_error_target(errors: &AvailabilityResponseErrors) -> String {
    if errors.respondent_name.is_some() {
        "respondent-name".to_owned()
    } else if let Some(candidate_id) = errors.candidate_ids.first() {
        format!("availability-{candidate_id}-available")
    } else {
        "response-heading".to_owned()
    }
}

async fn focus_element(element_id: &str) {
    let element_id = serde_json::to_string(element_id).expect("element id should serialize");
    let script = format!("document.getElementById({element_id})?.focus(); dioxus.send('focused');");
    let _ = read_browser_value(&script).await;
}

async fn focus_element_after_render(element_id: &str) {
    let element_id = serde_json::to_string(element_id).expect("element id should serialize");
    let script = format!(
        "requestAnimationFrame(() => {{ document.getElementById({element_id})?.focus(); dioxus.send('focused'); }});"
    );
    let _ = read_browser_value(&script).await;
}

#[cfg(feature = "web")]
async fn focus_and_select_element_after_render(element_id: &str) {
    let element_id = serde_json::to_string(element_id).expect("element id should serialize");
    let script = format!(
        "requestAnimationFrame(() => {{ const element = document.getElementById({element_id}); element?.focus(); element?.select(); dioxus.send('selected'); }});"
    );
    let _ = read_browser_value(&script).await;
}

async fn generate_response_capability() -> Option<String> {
    let capability = read_browser_value(
        "try { const bytes = new Uint8Array(32); crypto.getRandomValues(bytes); dioxus.send(Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('')); } catch (_) { dioxus.send(''); }",
    )
    .await?;
    (capability.len() == 64
        && capability
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    .then_some(capability)
}

fn format_public_candidate(candidate: &PublicCandidate) -> String {
    format_local_start(&candidate.local_date, &candidate.local_time)
}

fn format_candidate_summary(candidate: &CandidateResponseSummary) -> String {
    format_local_start(&candidate.local_date, &candidate.local_time)
}

fn candidate_fact_label(fact: &CandidateSummaryFact) -> &'static str {
    match fact {
        CandidateSummaryFact::EveryoneAvailable => "回答した全員が○です",
        CandidateSummaryFact::EveryoneAvailableIncludingMaybe => {
            "△を含めると、回答した全員が参加できそうです"
        }
        CandidateSummaryFact::OneUnavailable => "×が1件あります",
        CandidateSummaryFact::UniqueMostAvailable => "○が最も多い候補です",
    }
}

async fn browser_origin() -> Option<String> {
    read_browser_value("dioxus.send(window.location.origin);").await
}

/// JavaScript started directly by one user action: native share, then unsupported copy only.
pub fn decided_event_share_script(event_name: &str, share_url: &str) -> String {
    let title = serde_json::to_string(event_name).expect("event name should serialize");
    let text = serde_json::to_string(&format!("{event_name}の日程が決まりました。"))
        .expect("share text should serialize");
    let share_url = serde_json::to_string(share_url).expect("share URL should serialize");
    format!(
        "(() => {{ const data = {{ title: {title}, text: {text}, url: new URL({share_url}, window.location.origin).href }}; const send = value => dioxus.send(value); try {{ if (typeof navigator.share !== 'function' || (typeof navigator.canShare === 'function' && !navigator.canShare(data))) {{ if (typeof navigator.clipboard === 'object' && typeof navigator.clipboard.writeText === 'function') {{ navigator.clipboard.writeText(data.url).then(() => send('copied')).catch(() => send('manual')); }} else {{ send('manual'); }} return; }} navigator.share(data).then(() => send('started')).catch(error => send(error && error.name === 'AbortError' ? 'cancelled' : 'failed')); }} catch (_) {{ send('failed'); }} }})()"
    )
}

/// JavaScript for the explicit copy retry after share cancellation or failure.
pub fn decided_event_copy_script(share_url: &str) -> String {
    let share_url = serde_json::to_string(share_url).expect("share URL should serialize");
    format!(
        "(() => {{ const url = new URL({share_url}, window.location.origin).href; try {{ if (typeof navigator.clipboard !== 'object' || typeof navigator.clipboard.writeText !== 'function') {{ dioxus.send('manual'); return; }} navigator.clipboard.writeText(url).then(() => dioxus.send('copied')).catch(() => dioxus.send('manual')); }} catch (_) {{ dioxus.send('manual'); }} }})()"
    )
}

async fn load_saved_organizer_summary(public_id: &str) -> StoredOrganizerSummaryLoad {
    let Some(capability) = read_organizer_capability(public_id).await else {
        return StoredOrganizerSummaryLoad::MissingCapability;
    };
    let request = OrganizerSummaryInput {
        event_public_id: public_id.to_owned(),
        organizer_capability: capability,
    };
    match get_organizer_event_summary(request).await {
        Ok(summary) => StoredOrganizerSummaryLoad::Loaded(Box::new(summary)),
        Err(error) if organizer_authority_was_rejected(&error) => {
            StoredOrganizerSummaryLoad::RejectedCapability
        }
        Err(_) => StoredOrganizerSummaryLoad::Failed,
    }
}

fn organizer_authority_was_rejected(error: &ServerFnError) -> bool {
    matches!(
        error,
        ServerFnError::ServerError { code, .. } if *code == 404 || *code == 422
    )
}

fn valid_organizer_capability(capability: &str) -> bool {
    capability.len() == 64
        && capability
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

async fn read_organizer_capability(public_id: &str) -> Option<String> {
    #[cfg(feature = "web")]
    {
        let key = serde_json::to_string(&format!("tsunoru.organizer.{public_id}"))
            .expect("localStorage key should serialize");
        let script = format!(
            "try {{ dioxus.send(localStorage.getItem({key}) || ''); }} catch (_) {{ dioxus.send(''); }}"
        );
        return read_browser_value(&script)
            .await
            .filter(|capability| valid_organizer_capability(capability));
    }

    #[cfg(not(feature = "web"))]
    {
        let _ = public_id;
        None
    }
}

async fn store_organizer_capability(public_id: &str, capability: &str) -> bool {
    #[cfg(feature = "web")]
    {
        let key = serde_json::to_string(&format!("tsunoru.organizer.{public_id}"))
            .expect("localStorage key should serialize");
        let value = serde_json::to_string(capability).expect("capability should serialize");
        let script = format!(
            "try {{ localStorage.setItem({key}, {value}); dioxus.send('stored'); }} catch (_) {{ dioxus.send(''); }}"
        );
        return read_browser_value(&script).await.as_deref() == Some("stored");
    }

    #[cfg(not(feature = "web"))]
    {
        let _ = (public_id, capability);
        false
    }
}

async fn copy_to_clipboard(value: &str) -> bool {
    #[cfg(feature = "web")]
    {
        let value = serde_json::to_string(value).expect("share URL should serialize");
        let script = format!(
            "try {{ if (typeof navigator.clipboard !== 'object' || typeof navigator.clipboard.writeText !== 'function') {{ dioxus.send(''); }} else {{ navigator.clipboard.writeText({value}).then(() => dioxus.send('copied')).catch(() => dioxus.send('')); }} }} catch (_) {{ dioxus.send(''); }}"
        );
        return read_browser_value(&script).await.as_deref() == Some("copied");
    }

    #[cfg(not(feature = "web"))]
    {
        let _ = value;
        false
    }
}

async fn read_browser_value(script: &str) -> Option<String> {
    #[cfg(feature = "web")]
    {
        let mut evaluation = document::eval(script);
        return evaluation.recv::<String>().await.ok();
    }

    #[cfg(not(feature = "web"))]
    {
        let _ = script;
        None
    }
}
