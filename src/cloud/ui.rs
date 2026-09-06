//! The limited browser journey: enter, create, answer, and view organizer results.
use super::{
    Answer, CREATION_KEY, Candidate, CreateRequest, CreationRecord, Event, ResponseRecord, Store,
    api, load_creation, load_organizer, load_response, response_matrix, save_creation,
    save_response,
};
use crate::{
    browser::{self, LocalStore},
    domain::{
        Availability, CandidateInput, EventCreationDraft, EventCreationErrors, NewEventInput,
    },
    shared_ui::{
        CandidateDateTimePicker, DEFAULT_CANDIDATE_TIME, OrganizerResponseMatrixView,
        format_local_start,
    },
};
use dioxus::prelude::*;
use std::collections::BTreeMap;

const GOOGLE_CLIENT_ID: &str =
    "934625445815-ng4fgukkfmnube6v1gr6rc727qeo12dh.apps.googleusercontent.com";

const CLOUD_CSS: Asset = asset!("/assets/cloud.css");

#[derive(Clone, PartialEq)]
enum Access {
    Checking,
    Required,
    Ready,
    Failed,
}

fn is_public_event_path(path: &str) -> bool {
    let mut parts = path.trim_start_matches('/').split('/');
    matches!((parts.next(), parts.next(), parts.next()), (Some("events"), Some(id), None) if !id.is_empty())
}

#[cfg(test)]
mod access_tests {
    use super::is_public_event_path;

    #[test]
    fn shared_event_path_is_public_but_summary_and_create_are_not() {
        assert!(is_public_event_path("/events/abc"));
        assert!(!is_public_event_path("/events/abc/summary"));
        assert!(!is_public_event_path("/"));
    }
}

#[derive(Clone, PartialEq, Routable)]
enum CloudRoute {
    #[route("/")]
    Create {},
    #[route("/events/:id/summary")]
    Organizer { id: String },
    #[route("/events/:id")]
    Shared { id: String },
    #[route("/:..segments")]
    Missing { segments: Vec<String> },
}

#[component]
pub fn CloudApp() -> Element {
    let mut access = use_context_provider(|| Signal::new(Access::Checking));
    use_effect(move || {
        spawn(async move {
            let public_path = browser::path().is_some_and(|path| is_public_event_path(&path));
            access.set(if public_path {
                Access::Ready
            } else {
                match api::organizer_session_status().await {
                    Ok(()) => Access::Ready,
                    Err(error) if error.needs_access() => Access::Required,
                    Err(error) if error.status == 503 => match api::session().await {
                        Ok(()) => Access::Ready,
                        Err(fallback) if fallback.needs_access() => Access::Required,
                        Err(_) => Access::Failed,
                    },
                    Err(_) => Access::Failed,
                }
            });
        });
    });
    rsx! {
        document::Meta { name: "robots", content: "noindex, nofollow" }
        document::Link { rel: "icon", href: crate::FAVICON }
        document::Link { rel: "stylesheet", href: crate::MAIN_CSS }
        document::Link { rel: "stylesheet", href: CLOUD_CSS }
        main { class: "app-shell cloud-shell",
            match access() {
                Access::Checking => rsx! { p { role: "status", "接続を確認しています…" } },
                Access::Required => rsx! { AccessEntry {} },
                Access::Failed => rsx! {
                    section { class: "message-card", h1 { "接続できませんでした" }
                        p { "少し待ってから、もう一度開いてください。" }
                        button { class: "secondary-button", onclick: move |_| access.set(Access::Required), "試用コードを入力する" }
                    }
                },
                Access::Ready => rsx! { CloudHeader {} Router::<CloudRoute> {} },
            }
        }
    }
}

#[component]
fn AccessEntry() -> Element {
    let mut access = use_context::<Signal<Access>>();
    let mut busy = use_signal(|| false);
    let mut message = use_signal(String::new);
    rsx! {
        GoogleSignInButton { on_success: move |(token, nonce): (String, String)| {
            if busy() { return; }
            busy.set(true); message.set(String::new());
            spawn(async move {
                match api::organizer_session(token, nonce).await {
                    Ok(()) => access.set(Access::Ready),
                    Err(error) => message.set(if error.status == 401 { "Googleログインを確認できませんでした。".to_owned() } else { error.message().to_owned() }),
                }
                busy.set(false);
            });
        } }
        if !message.is_empty() { p { role: "alert", class: "form-error", "{message}" } }
        p { class: "field-help", "回答者はログインせずに共有URLから回答できます。" }
        TrialCodeForm { busy: busy(), message: message(), on_submit: move |code: String| {
            if busy() { return; }
            busy.set(true); message.set(String::new());
            spawn(async move {
                match api::login(code).await {
                    Ok(()) => access.set(Access::Ready),
                    Err(error) => message.set(if error.status == 401 { "試用コードを確認してください。".to_owned() } else { error.message().to_owned() }),
                }
                busy.set(false);
            });
        } }
    }
}

#[component]
fn GoogleSignInButton(on_success: EventHandler<(String, String)>) -> Element {
    let nonce = match browser::random_key() {
        Ok(value) => value,
        Err(_) => String::new(),
    };
    use_effect(move || {
        let nonce = nonce.clone();
        if nonce.is_empty() {
            return;
        }
        spawn(async move {
            #[cfg(feature = "web")]
            {
                let script = format!(
                    r#"
                    (() => new Promise((resolve) => {{
                        const start = () => {{
                            const target = document.getElementById('google-signin-button');
                            if (!target || !window.google?.accounts?.id) {{ resolve(''); return; }}
                            google.accounts.id.initialize({{ client_id: '{GOOGLE_CLIENT_ID}', nonce: '{nonce}', callback: (response) => resolve(response.credential) }});
                            google.accounts.id.renderButton(target, {{ theme: 'outline', size: 'large', width: 320 }});
                        }};
                        if (window.google?.accounts?.id) start(); else setTimeout(start, 500);
                    }})()).then((token) => dioxus.send(token));
                "#
                );
                let mut evaluation = document::eval(&script);
                if let Ok(token) = evaluation.recv::<String>().await {
                    if !token.is_empty() {
                        on_success.call((token, nonce));
                    }
                }
            }
        });
    });
    rsx! {
        document::Script { src: Some("https://accounts.google.com/gsi/client".to_owned()), defer: Some(true) }
        div { id: "google-signin-button", class: "google-signin-button", role: "group", aria_label: "Googleでログイン" }
    }
}

/// A focused entry form that never retains the trial code after submission.
#[component]
pub fn TrialCodeForm(busy: bool, message: String, on_submit: EventHandler<String>) -> Element {
    let mut code = use_signal(String::new);
    rsx! {
        section { class: "cloud-access creation-form", aria_labelledby: "access-heading",
            a { class: "wordmark", href: "/", "TSUNORU" }
            h1 { id: "access-heading", tabindex: "-1", onmounted: move |_| browser::focus("access-heading"), "日程をつのる" }
            p { "試用コードを入力して始めてください。イベントや回答の送信途中でも、保存した内容から再開できます。" }
            form { onsubmit: move |event| { event.prevent_default(); if !busy { let value = code().trim().to_owned(); code.set(String::new()); on_submit.call(value); } },
                div { class: "field-group", label { r#for: "trial-code", "試用コード" }
                    input { id: "trial-code", r#type: "password", value: "{code}", required: true, autocomplete: "off", maxlength: 128,
                        disabled: busy, oninput: move |event| code.set(event.value()), aria_describedby: "trial-code-help" }
                    p { id: "trial-code-help", class: "field-help", "コードを受け取った人が利用できます。" }
                }
                if !message.is_empty() { p { role: "alert", class: "form-error", "{message}" } }
                button { class: "primary-button", r#type: "submit", disabled: busy, if busy { "確認しています…" } else { "始める" } }
            }
        }
    }
}

#[component]
fn CloudHeader() -> Element {
    let mut access = use_context::<Signal<Access>>();
    let mut busy = use_signal(|| false);
    let mut message = use_signal(String::new);
    rsx! {
        header { class: "cloud-header",
            a { class: "wordmark", href: "/", "TSUNORU" }
            button { class: "text-link", r#type: "button", disabled: busy(), onclick: move |_| async move {
                if busy() { return; }
                busy.set(true);
                match api::organizer_logout().await {
                    Ok(()) => access.set(Access::Required),
                    Err(error) if error.needs_access() => access.set(Access::Required),
                    Err(error) => message.set(error.message().to_owned()),
                }
                busy.set(false);
            }, "利用を終了" }
        }
        if !message().is_empty() { p { role: "alert", class: "form-error", "{message}" } }
    }
}

fn handle_error(error: api::ApiError, mut access: Signal<Access>, mut message: Signal<String>) {
    if error.needs_access() {
        access.set(Access::Required);
    } else {
        message.set(error.message().to_owned());
    }
}

#[component]
fn Create() -> Element {
    let access = use_context::<Signal<Access>>();
    let mut record = use_signal(|| None::<CreationRecord>);
    let mut initial = use_signal(|| None::<NewEventInput>);
    let mut loaded = use_signal(|| false);
    let mut storage_failed = use_signal(|| false);
    let mut message = use_signal(String::new);
    let mut busy = use_signal(|| false);
    use_effect(move || {
        match load_creation(&LocalStore) {
            Ok(saved) => record.set(saved),
            Err(error) => {
                message.set(error);
                storage_failed.set(true);
            }
        }
        loaded.set(true);
    });
    let mut submit = move |next: CreationRecord| {
        if busy() {
            return;
        }
        // A reloaded pending operation must still be writable before re-sending.
        if let Err(error) = save_creation(&LocalStore, &next) {
            message.set(error);
            return;
        }
        record.set(Some(next.clone()));
        busy.set(true);
        message.set(String::new());
        spawn(async move {
            match api::create(&next.request).await {
                Ok(()) => {
                    let mut accepted = next;
                    accepted.accepted = true;
                    // A failed acknowledgement write leaves the durable request
                    // retryable; the server has already confirmed this creation.
                    let _ = save_creation(&LocalStore, &accepted);
                    record.set(Some(accepted));
                }
                Err(error) if error.status == 400 && error.code == "invalid_request" => {
                    // This API error guarantees validation stopped before any
                    // database write. Only this response permits editing anew.
                    match LocalStore.remove(CREATION_KEY) {
                        Ok(()) => {
                            initial.set(Some(next.request.input()));
                            record.set(None);
                            message.set("日時やタイムゾーンを確認してください。夏時間の切り替えで存在しない時刻や、二通りになる時刻は保存できません。".to_owned());
                        }
                        Err(error) => message.set(error),
                    }
                }
                Err(error) => handle_error(error, access, message),
            }
            busy.set(false);
        });
    };
    if !loaded() {
        return rsx! { p { role: "status", "保存した内容を確認しています…" } };
    }
    if let Some(saved) = record() {
        if saved.accepted {
            let event = saved.request.event();
            return rsx! {
                CreatedView { event }
                button { class: "secondary-button cloud-new-event", onclick: move |_| {
                    match LocalStore.remove(CREATION_KEY) {
                        Ok(()) => { record.set(None); initial.set(None); message.set(String::new()); },
                        Err(error) => message.set(error),
                    }
                }, "別のイベントを作る" }
                StatusMessage { message: message() }
            };
        }
        let event = saved.request.event();
        return rsx! {
            section { class: "creation-form cloud-card",
                h1 { "送信途中のイベント" }
                EventDetails { event }
                ul { class:"cloud-saved-answer",
                    for candidate in &saved.request.candidates {
                        li { "{format_local_start(&candidate.local_date,&candidate.local_time)}" }
                    }
                }
                p { "保存した内容で送信を確かめます。同じ内容を再送しても、イベントは増えません。" }
                StatusMessage { message: message() }
                button { class: "primary-button", disabled: busy(), onclick: move |_| submit(saved.clone()),
                    if busy() { "保存しています…" } else { "保存した内容で再送する" }
                }
            }
        };
    }
    rsx! {
        section { class: "creation-layout cloud-create",
            header { class: "page-heading", p { class: "eyebrow", "友人と仲間の日程調整" }
                h1 { "日程をつのる" } p { class: "lead", "候補の日を選んで、みんなに都合を聞きましょう。" }
            }
            div {
                StatusMessage { message: message() }
                if !storage_failed() {
                    CreateForm { busy: busy(), initial:initial(), on_submit: move |input: NewEventInput| {
                        match browser::random_key().and_then(|id| browser::random_key().map(|cap| CreateRequest::new(input,id,cap))) {
                            Ok(request) => submit(CreationRecord { request, accepted:false }),
                            Err(error) => message.set(error),
                        }
                    } }
                }
            }
        }
    }
}

#[component]
pub fn CreateForm(
    busy: bool,
    initial: Option<NewEventInput>,
    on_submit: EventHandler<NewEventInput>,
) -> Element {
    let mut name = use_signal(|| {
        initial
            .as_ref()
            .map(|input| input.name.clone())
            .unwrap_or_default()
    });
    let mut note = use_signal(|| {
        initial
            .as_ref()
            .and_then(|input| input.organizer_note.clone())
            .unwrap_or_default()
    });
    let date = use_signal(String::new);
    let time = use_signal(|| DEFAULT_CANDIDATE_TIME.to_owned());
    let candidates = use_signal(|| {
        initial
            .as_ref()
            .map(|input| input.candidates.clone())
            .unwrap_or_default()
    });
    let mut zone = use_signal(|| {
        initial
            .as_ref()
            .map(|input| input.time_zone.clone())
            .unwrap_or_default()
    });
    let mut errors = use_signal(EventCreationErrors::default);
    use_effect(move || {
        if initial.is_none()
            && let Some(value) = browser::time_zone()
        {
            zone.set(value);
        }
    });
    let current_errors = errors();
    rsx! {
        form { class: "creation-form", novalidate: true, onsubmit: move |event| {
            event.prevent_default(); if busy { return; }
            let input = EventCreationDraft { name:name(), organizer_note:note(),time_zone:zone(),candidates:candidates(),
                pending_candidate:CandidateInput { local_date:date(),local_time:time() } }.prepare();
            match input {
                Ok(input) => { errors.set(EventCreationErrors::default()); on_submit.call(input); },
                Err(error) => {
                    let target = if error.name.is_some() { "event-name" }
                        else if error.organizer_note.is_some() { "organizer-note" }
                        else if error.candidates.is_some() { "candidate-time" }
                        else { "event-time-zone" };
                    errors.set(error); browser::focus(target);
                }
            }
        },
            div { class: "field-group", label { r#for: "event-name", "イベント名" }
                input { id:"event-name", r#type:"text", value:"{name}", required:true, maxlength:100, disabled:busy,
                    aria_invalid: current_errors.name.is_some(), aria_describedby:"event-name-error", oninput:move |event| name.set(event.value()) }
                if let Some(error) = &current_errors.name { p { id:"event-name-error", class:"field-error", role:"alert", "{error}" } }
            }
            div { class: "field-group", label { r#for:"organizer-note", "主催者のひとこと（任意）" }
                textarea { id:"organizer-note", value:"{note}", maxlength:500, rows:3, disabled:busy, aria_describedby:"organizer-note-error", oninput:move |event| note.set(event.value()) }
                if let Some(error) = &current_errors.organizer_note { p { id:"organizer-note-error", class:"field-error", role:"alert", "{error}" } }
            }
            CandidateDateTimePicker { candidates, candidate_date:date, candidate_time:time, errors, id_prefix:"candidate" }
            div { class:"field-group", label { r#for:"event-time-zone", "候補日時のタイムゾーン" }
                input { id:"event-time-zone", value:"{zone}", r#type:"text", maxlength:64, disabled:busy,
                    oninput: move |event| zone.set(event.value()), placeholder:"Asia/Tokyo", aria_describedby:"zone-help zone-error" }
                p { id:"zone-help", class:"field-help", "候補は全員にこのタイムゾーンの時刻で表示します。日本なら Asia/Tokyo です。" }
                if let Some(error) = &current_errors.time_zone { p { id:"zone-error", class:"field-error", role:"alert", "{error}" } }
            }
            button { class:"primary-button", r#type:"submit", disabled:busy, if busy { "保存しています…" } else { "イベントを作る" } }
        }
    }
}

#[component]
pub fn CreatedView(event: Event) -> Element {
    let path = format!("/events/{}", event.id);
    let summary = format!("{path}/summary");
    rsx! {
        section { class:"success-card cloud-card", aria_labelledby:"created-heading",
            h1 { id:"created-heading", tabindex:"-1", onmounted:move |_| browser::focus("created-heading"), "イベントを作りました" }
            h2 { "{event.name}" }
            p { "回答用のURLを、都合を聞きたい人へ渡してください。" }
            ShareLink { path:path.clone() }
            nav { class:"success-actions", aria_label:"作成したイベントを開く",
                a { class:"primary-button link-button", href:"{path}", "回答用ページを開く" }
                a { class:"secondary-button link-button", href:"{summary}", "みんなの回答を確認" }
            }
            p { class:"field-help", "主催者の権限はこのブラウザーに保存しました。回答の確認はこのブラウザーから開いてください。" }
        }
    }
}

#[component]
fn ShareLink(path: String) -> Element {
    let url = browser::absolute_url(&path);
    let mut message = use_signal(String::new);
    rsx! {
        div { class:"share-field", label { r#for:"share-url", "回答用の共有URL" }
            div { class:"share-controls",
                input { id:"share-url", r#type:"url", value:"{url}", readonly:true }
                button { class:"secondary-button", r#type:"button", onclick:move |_| {
                    let value=url.clone(); async move {
                        message.set(if browser::copy(&value).await { "URLをコピーしました。" } else { "URLを選択してコピーしてください。" }.to_owned());
                    }
                }, "URLをコピー" }
            }
            p { class:"copy-status", aria_live:"polite", "{message}" }
        }
    }
}

#[component]
fn Shared(id: String) -> Element {
    rsx! { ResponsePage { key:"{id}", id } }
}

#[component]
fn ResponsePage(id: String) -> Element {
    let mut access = use_context::<Signal<Access>>();
    let query = id.clone();
    let mut loaded = use_resource(move || {
        let id = query.clone();
        async move { api::event(&id).await }
    });
    use_effect(move || {
        if loaded().is_some_and(|value| value.is_err_and(|error| error.needs_access())) {
            access.set(Access::Required);
        }
    });
    match loaded() {
        Some(Ok(event)) => rsx! { ResponseEditor { key:"{event.id}", event } },
        Some(Err(error)) if error.needs_access() => {
            rsx! { p { role:"status", "試用コードを確認しています…" } }
        }
        Some(Err(error)) => {
            rsx! { LoadFailure { message:error.message().to_owned(), on_retry:move |_| loaded.restart() } }
        }
        None => rsx! { p { role:"status", "イベントを読み込んでいます…" } },
    }
}

#[component]
fn ResponseEditor(event: Event) -> Element {
    let access = use_context::<Signal<Access>>();
    let mut saved = use_signal(|| None::<ResponseRecord>);
    let mut loaded = use_signal(|| false);
    let mut storage_failed = use_signal(|| false);
    let mut message = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let id = event.id.clone();
    use_effect(move || {
        match load_response(&LocalStore, &id) {
            Ok(record) => saved.set(record),
            Err(error) => {
                message.set(error);
                storage_failed.set(true);
            }
        }
        loaded.set(true);
    });
    let mut submit = move |record: ResponseRecord| {
        if busy() {
            return;
        }
        if let Err(error) = save_response(&LocalStore, &record) {
            message.set(error);
            return;
        }
        saved.set(Some(record.clone()));
        busy.set(true);
        message.set(String::new());
        spawn(async move {
            match api::respond(&record).await {
                Ok(()) => {
                    let mut record = record;
                    record.accepted = true;
                    let _ = save_response(&LocalStore, &record);
                    saved.set(Some(record));
                }
                Err(error) => handle_error(error, access, message),
            }
            busy.set(false);
        });
    };
    let current = saved();
    rsx! {
        article { class:"public-event cloud-card",
            EventDetails { event:event.clone() }
            if !loaded() { p { role:"status", "保存した回答を確認しています…" } }
            else if let Some(record)=current {
                if record.accepted { AnswerAccepted {} }
                else {
                    p { "送信途中の回答があります。保存した内容で再送できます。" }
                    SavedAnswer { event:event.clone(), answer:record.answer.clone() }
                    button { class:"primary-button", disabled:busy(), onclick:move |_| submit(record.clone()),
                        if busy() { "送信しています…" } else { "保存した回答を再送する" }
                    }
                }
            } else if !storage_failed() {
                AnswerForm { event:event.clone(), busy:busy(), on_submit:move |answer: Answer| {
                    match browser::random_key() {
                        Ok(capability) => submit(ResponseRecord { event_id:event.id.clone(),capability,answer,accepted:false }),
                        Err(error) => message.set(error),
                    }
                } }
            }
            StatusMessage { message:message() }
        }
    }
}

#[component]
pub fn EventDetails(event: Event) -> Element {
    rsx! {
        h1 { "{event.name}" }
        if let Some(note)=event.organizer_note { blockquote { class:"organizer-note", "{note}" } }
        p { class:"field-help", "{event.time_zone} の時刻で表示しています。" }
    }
}

#[component]
pub fn AnswerForm(event: Event, busy: bool, on_submit: EventHandler<Answer>) -> Element {
    let mut name = use_signal(String::new);
    let mut choices = use_signal(BTreeMap::<String, Availability>::new);
    let mut message = use_signal(String::new);
    let submit_event = event.clone();
    rsx! {
        form { class:"availability-form", novalidate:true, onsubmit:move |event| {
            event.prevent_default(); if busy { return; }
            match Answer::prepare(&submit_event,&name(),&choices()) {
                Ok(answer) => { message.set(String::new()); on_submit.call(answer); },
                Err(error) => {
                    let target = if name().trim().is_empty() || name().chars().count() > 100 || name().chars().any(char::is_control) {
                        "respondent-name".to_owned()
                    } else {
                        submit_event.candidates.iter().find(|candidate| !choices().contains_key(&candidate.id))
                            .map(|candidate| format!("availability-{}-available",candidate.id))
                            .unwrap_or_else(|| "respondent-name".to_owned())
                    };
                    message.set(error); browser::focus(&target);
                },
            }
        },
            div { class:"field-group", label { r#for:"respondent-name", "あなたの名前" }
                input { id:"respondent-name", value:"{name}", r#type:"text", maxlength:100, required:true, disabled:busy,
                    oninput:move |event| name.set(event.value()) }
            }
            p { id:"availability-help", class:"field-help", "すべての候補に、○・△・×で回答してください。" }
            for candidate in &event.candidates {
                { let id=candidate.id.clone(); rsx! {
                    AnswerChoice { key:"{id}", candidate:candidate.clone(), selected:choices().get(&id).copied(), disabled:busy,
                        on_select:move |availability| { choices.write().insert(id.clone(),availability); } }
                } }
            }
            StatusMessage { message:message() }
            button { class:"primary-button", r#type:"submit", disabled:busy, if busy { "送信しています…" } else { "回答を送る" } }
        }
    }
}

#[component]
pub fn AnswerChoice(
    candidate: Candidate,
    selected: Option<Availability>,
    disabled: bool,
    on_select: EventHandler<Availability>,
) -> Element {
    let label = format_local_start(&candidate.local_date, &candidate.local_time);
    rsx! {
        fieldset { class:"availability-candidate", disabled,
            legend { time { datetime:"{candidate.local_date}T{candidate.local_time}", "{label}" } }
            div { class:"availability-options",
                for value in [Availability::Available,Availability::Maybe,Availability::Unavailable] {
                    { let input_id=format!("availability-{}-{}",candidate.id,value.storage_value()); rsx! {
                        input { id:input_id.clone(), class:"availability-radio", r#type:"radio", name:"availability-{candidate.id}",
                            value:value.storage_value(), checked:selected==Some(value), required:true,
                            aria_label:value.accessible_label(), onchange:move |_| on_select.call(value) }
                        label { class:"availability-option-label", r#for:input_id,
                            span { class:"availability-symbol", aria_hidden:"true", "{value.symbol()}" }
                            span { "{value.short_label()}" }
                        }
                    } }
                }
            }
        }
    }
}

#[component]
fn SavedAnswer(event: Event, answer: Answer) -> Element {
    rsx! {
        p { "回答者：{answer.respondent_name}" }
        ul { class:"cloud-saved-answer",
            for candidate in event.candidates {
                { let selection=answer.availabilities.iter().find(|choice|choice.candidate_id==candidate.id);
                  let value=selection.map(|choice|choice.availability.accessible_label()).unwrap_or("未回答");
                  rsx! { li { "{format_local_start(&candidate.local_date,&candidate.local_time)}：{value}" } }
                }
            }
        }
    }
}

#[component]
pub fn AnswerAccepted() -> Element {
    rsx! {
        section { class:"response-success", role:"status", aria_labelledby:"answer-accepted",
            h2 { id:"answer-accepted", tabindex:"-1", onmounted:move |_| browser::focus("answer-accepted"), "回答を送りました" }
            p { "主催者が回答を確認できます。このブラウザーでは送信済みの回答を保持しています。" }
        }
    }
}

#[component]
fn Organizer(id: String) -> Element {
    rsx! { OrganizerPage { key:"{id}", id } }
}

#[component]
fn OrganizerPage(id: String) -> Element {
    let mut access = use_context::<Signal<Access>>();
    let mut authority = use_signal(|| None::<String>);
    let mut message = use_signal(String::new);
    let authority_id = id.clone();
    use_effect(move || match load_organizer(&LocalStore, &authority_id) {
        Ok(value) => authority.set(value),
        Err(error) => message.set(error),
    });
    let query = id.clone();
    let mut result = use_resource(move || {
        let id = query.clone();
        let cap = authority();
        async move {
            let Some(cap) = cap else {
                return Ok(None);
            };
            let event = api::event(&id).await?;
            let responses = api::responses(&id, &cap).await?;
            let matrix = response_matrix(&event, &responses).map_err(|_| api::ApiError {
                status: 0,
                code: "invalid_response".to_owned(),
            })?;
            Ok::<_, api::ApiError>(Some((event, matrix)))
        }
    });
    // The effect runs on every result change and keeps hooks unconditional.
    use_effect(move || {
        if result().is_some_and(|value| value.is_err_and(|error| error.needs_access())) {
            access.set(Access::Required);
        }
    });
    if authority().is_none() {
        return rsx! {
            section { class:"message-card cloud-card", h1 { "主催者の回答確認" }
                p { "イベントを作ったブラウザーから開いてください。このブラウザーには主催者の権限がありません。" }
                StatusMessage { message:message() }
                a { class:"text-link", href:format!("/events/{id}"), "回答用ページを開く" }
            }
        };
    }
    rsx! {
        section { class:"cloud-results",
            h1 { "みんなの回答" }
            match result() {
                Some(Ok(Some((event,matrix)))) => rsx! {
                    h2 { "{event.name}" }
                    p { "{matrix.responses.len()}人が回答しました。" }
                    OrganizerResponseMatrixView { matrix }
                    ShareLink { path:format!("/events/{}",event.id) }
                },
                Some(Err(error)) => rsx! { StatusMessage { message:error.message().to_owned() } },
                _ => rsx! { p { role:"status", "回答を読み込んでいます…" } },
            }
            button { class:"secondary-button", r#type:"button", disabled:result.state()==UseResourceState::Pending,
                onclick:move |_| result.restart(), "回答を更新" }
        }
    }
}

#[component]
fn Missing(segments: Vec<String>) -> Element {
    let _ = segments;
    rsx! { section { class:"message-card", h1 { "ページが見つかりません" } a { href:"/", "新しい日程をつのる" } } }
}

#[component]
fn LoadFailure(message: String, on_retry: EventHandler<()>) -> Element {
    rsx! { section { class:"message-card", h1 { "イベントを開けませんでした" } p { role:"alert", "{message}" }
    button { class:"secondary-button", r#type:"button", onclick:move |_|on_retry.call(()), "もう一度読み込む" } } }
}

#[component]
fn StatusMessage(message: String) -> Element {
    rsx! { if !message.is_empty() { p { class:"form-error", role:"alert", "{message}" } } }
}
