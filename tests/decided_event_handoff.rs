use dioxus::prelude::*;
use tsunoru::{
    domain::{OrganizerEventDecision, PublicCandidate, PublicEvent, PublicEventDecision},
    ui::{
        DecidedEventActions, OrganizerDecidedEventHandoff, PublicEventView, ShareActionState,
        begin_share_action, decided_event_copy_script, decided_event_share_script,
        next_share_action_state,
    },
};

fn public_event(decision: Option<PublicEventDecision>) -> PublicEvent {
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
        decision,
    }
}

fn decision() -> PublicEventDecision {
    PublicEventDecision {
        candidate_id: 11,
        local_date: "2026-09-18".to_owned(),
        local_time: "19:00".to_owned(),
    }
}

#[test]
fn decided_public_view_replaces_answering_with_the_two_next_actions() {
    let html = dioxus_ssr::render_element(rsx! {
        PublicEventView { event: public_event(Some(decision())) }
    });

    for expected in [
        "秋の餃子会",
        "焼きたてを囲みたいです",
        "決定した予定",
        "日程が決まりました",
        "2026年9月18日 19:00",
        "Asia/Tokyo の時刻",
        "カレンダーに追加",
        "この予定を共有",
        "/api/events/7af78527-813b-4cdd-a632-058f3ce885aa/calendar.ics",
        "download",
        "共有URL",
    ] {
        assert!(
            html.contains(expected),
            "a decided public event should expose {expected:?}: {html}"
        );
    }

    for forbidden in [
        "あなたの名前",
        "回答を送る",
        "type=\"radio\"",
        "/summary",
        "organizer_capability",
        "decided_at",
        "届いた日程候補",
    ] {
        assert!(
            !html.contains(forbidden),
            "the decided public page must not expose {forbidden:?}: {html}"
        );
    }
}

#[test]
fn undecided_public_view_keeps_the_existing_short_answer_path() {
    let html = dioxus_ssr::render_element(rsx! {
        PublicEventView { event: public_event(None) }
    });

    assert!(
        html.contains("届いた日程候補")
            && html.contains("あなたの名前")
            && html.contains("回答を送る")
            && html.contains("type=\"radio\"")
            && !html.contains("日程が決まりました")
            && !html.contains("calendar.ics")
            && !html.contains("この予定を共有"),
        "an undecided event must retain the existing anonymous path: {html}"
    );
}

#[test]
fn organizer_handoff_reuses_public_actions_without_receiving_authority() {
    let html = dioxus_ssr::render_element(rsx! {
        OrganizerDecidedEventHandoff {
            public_id: "7af78527-813b-4cdd-a632-058f3ce885aa".to_owned(),
            event_name: "秋の餃子会".to_owned(),
            time_zone: "Asia/Tokyo".to_owned(),
            decision: OrganizerEventDecision {
                candidate_id: 11,
                local_date: "2026-09-18".to_owned(),
                local_time: "19:00".to_owned(),
            },
        }
    });

    assert!(
        html.contains("日程を確定しました")
            && html.contains("カレンダーに追加")
            && html.contains("この予定を共有")
            && html.contains("/events/7af78527-813b-4cdd-a632-058f3ce885aa")
            && !html.contains("/summary")
            && !html.contains("capability"),
        "the organizer should hand off only the public event URL: {html}"
    );
}

#[test]
fn handoff_actions_keep_a_native_download_and_hidden_manual_copy_fallback() {
    let html = dioxus_ssr::render_element(rsx! {
        DecidedEventActions {
            public_id: "7af78527-813b-4cdd-a632-058f3ce885aa".to_owned(),
            event_name: "秋の餃子会".to_owned(),
        }
    });

    assert!(
        html.contains("<a")
            && html
                .contains("href=\"/api/events/7af78527-813b-4cdd-a632-058f3ce885aa/calendar.ics\"")
            && html.contains("type=\"text/calendar\"")
            && html.contains("download")
            && html.contains("id=\"decided-event-share-url\"")
            && html.contains("readonly")
            && html.contains("hidden"),
        "download must work without JS while manual copy stays available after failure: {html}"
    );

    let css = include_str!("../assets/main.css");
    assert!(
        css.contains(".decided-event-manual-copy[hidden]") && css.contains("display: none"),
        "author CSS must not override the hidden fallback into the initial layout: {css}"
    );
}

#[test]
fn browser_scripts_preserve_share_activation_and_do_not_copy_after_cancel() {
    let share = decided_event_share_script(
        "秋の餃子会",
        "https://example.test/events/7af78527-813b-4cdd-a632-058f3ce885aa",
    );
    let copy = decided_event_copy_script(
        "https://example.test/events/7af78527-813b-4cdd-a632-058f3ce885aa",
    );

    assert!(
        share.contains("typeof navigator.share")
            && share
                .contains("typeof navigator.canShare === 'function' && !navigator.canShare(data)")
            && share.contains("navigator.share(data)")
            && share.contains("navigator.clipboard.writeText(data.url)")
            && share.contains("AbortError")
            && share.contains("cancelled")
            && share.contains("started"),
        "the first click should synchronously choose native share or unsupported copy: {share}"
    );
    let cancellation = share
        .split("AbortError")
        .nth(1)
        .expect("share script should distinguish cancellation");
    assert!(
        !cancellation.contains("clipboard.writeText"),
        "cancelling a share sheet must not silently overwrite the clipboard: {share}"
    );
    assert!(
        copy.contains("typeof navigator.clipboard")
            && copy.contains("navigator.clipboard.writeText")
            && copy.contains("manual"),
        "an explicit retry needs a guarded clipboard path and manual fallback: {copy}"
    );
}

#[test]
fn share_outcomes_keep_truthful_labels_and_a_manual_last_resort() {
    assert_eq!(
        begin_share_action(ShareActionState::ReadyToShare),
        Some(ShareActionState::InProgress)
    );
    assert_eq!(
        begin_share_action(ShareActionState::ReadyToCopy),
        Some(ShareActionState::InProgress)
    );
    assert_eq!(begin_share_action(ShareActionState::InProgress), None);

    assert_eq!(
        next_share_action_state(ShareActionState::ReadyToShare, "started"),
        ShareActionState::ShareStarted
    );
    assert_eq!(
        next_share_action_state(ShareActionState::ReadyToShare, "copied"),
        ShareActionState::UrlCopied
    );
    assert_eq!(
        next_share_action_state(ShareActionState::ReadyToShare, "cancelled"),
        ShareActionState::ReadyToCopy
    );
    assert_eq!(
        next_share_action_state(ShareActionState::ReadyToShare, "failed"),
        ShareActionState::ReadyToCopy
    );
    assert_eq!(
        next_share_action_state(ShareActionState::ReadyToCopy, "manual"),
        ShareActionState::ManualCopy
    );
}
