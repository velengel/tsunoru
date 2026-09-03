use dioxus::prelude::*;
use tsunoru::{
    domain::{
        CandidateInput, CreatedEvent, EVENT_NAME_MAX_CHARS, EventCreationDraft,
        EventCreationErrors, ORGANIZER_NOTE_MAX_CHARS, PublicCandidate, PublicEvent,
    },
    ui::{CreationSuccess, EventCreationForm, PublicEventView},
};

fn candidate(date: &str, time: &str) -> CandidateInput {
    CandidateInput {
        local_date: date.to_owned(),
        local_time: time.to_owned(),
    }
}

fn public_event() -> PublicEvent {
    PublicEvent {
        public_id: "7af78527-813b-4cdd-a632-058f3ce885aa".to_owned(),
        name: "秋の餃子会".to_owned(),
        organizer_note: Some("焼きたてを囲みたいです".to_owned()),
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![PublicCandidate {
            id: 11,
            local_date: "2026-09-18".to_owned(),
            local_time: "19:00".to_owned(),
        }],
        decision: None,
    }
}

#[test]
fn creation_result_debug_redacts_the_organizer_capability() {
    let raw_capability = "raw-organizer-capability-must-not-enter-debug";
    let created = CreatedEvent {
        event: public_event(),
        organizer_capability: raw_capability.to_owned(),
    };

    let debug = format!("{created:?}");
    assert!(
        debug.contains("[REDACTED]") && !debug.contains(raw_capability),
        "Debug output must preserve the event shape without exposing bearer authority: {debug}"
    );

    let response = serde_json::to_value(&created).expect("creation result should serialize");
    assert_eq!(
        response["organizer_capability"], raw_capability,
        "the one-time success response must still carry authority to the organizer browser"
    );
}

#[test]
fn creation_form_exposes_the_smallest_mobile_friendly_input_set() {
    let html = dioxus_ssr::render_element(rsx! {
        EventCreationForm { initial_errors: EventCreationErrors::default() }
    });

    for expected in [
        "<form",
        "for=\"event-name\"",
        "イベント名",
        "主催者のひとこと",
        "任意",
        "type=\"date\"",
        "type=\"text\"",
        "inputmode=\"numeric\"",
        "value=\"19:00\"",
        "カレンダーを準備しています",
        &format!("maxlength={EVENT_NAME_MAX_CHARS}"),
        &format!("maxlength={ORGANIZER_NOTE_MAX_CHARS}"),
        "候補に追加",
        "イベントを作る",
    ] {
        assert!(
            html.contains(expected),
            "creation form should contain {expected:?}: {html}"
        );
    }

    assert!(
        !html.contains("type=\"time\"") && !html.contains("終了時刻") && !html.contains("ログイン"),
        "the first story must not add unrequested fields or login: {html}"
    );
}

#[test]
fn validation_errors_are_bound_to_the_relevant_inputs() {
    let errors = EventCreationErrors {
        name: Some("イベント名を入力してください。".to_owned()),
        organizer_note: None,
        candidates: Some("候補日時を一件以上追加してください。".to_owned()),
        time_zone: None,
    };

    let html = dioxus_ssr::render_element(rsx! {
        EventCreationForm { initial_errors: errors }
    });

    assert!(
        html.contains("aria-invalid=true")
            && html.contains("aria-describedby=\"event-name-error\"")
            && html.contains("id=\"event-name-error\"")
            && html.contains("role=\"alert\"")
            && html.contains("イベント名を入力してください。")
            && html.contains("候補日時を一件以上追加してください。"),
        "errors should be announced and connected to their fields: {html}"
    );
}

#[test]
fn optional_note_and_a_complete_pending_candidate_prepare_a_valid_event() {
    let draft = EventCreationDraft {
        name: "  秋の餃子会  ".to_owned(),
        organizer_note: "   ".to_owned(),
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: Vec::new(),
        pending_candidate: candidate("2026-09-18", "19:00"),
    };

    let prepared = draft
        .prepare()
        .expect("a note is optional and a complete pending candidate is sufficient");

    assert_eq!(prepared.name, "秋の餃子会");
    assert_eq!(prepared.organizer_note, None);
    assert_eq!(prepared.candidates, vec![candidate("2026-09-18", "19:00")]);
}

#[test]
fn default_base_time_without_a_direct_date_does_not_become_a_partial_candidate() {
    let draft = EventCreationDraft {
        name: "秋の餃子会".to_owned(),
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![candidate("2026-09-18", "19:00")],
        pending_candidate: candidate("", "19:00"),
        ..EventCreationDraft::default()
    };

    assert_eq!(
        draft
            .prepare()
            .expect("the always-visible base time is not a pending date")
            .candidates,
        vec![candidate("2026-09-18", "19:00")]
    );
}

#[test]
fn the_iana_utc_time_zone_is_accepted() {
    let draft = EventCreationDraft {
        name: "オンライン読書会".to_owned(),
        time_zone: "UTC".to_owned(),
        pending_candidate: candidate("2026-09-18", "19:00"),
        ..EventCreationDraft::default()
    };

    assert!(
        draft.prepare().is_ok(),
        "UTC is a valid IANA time-zone identifier even though it has no slash"
    );
}

#[test]
fn event_name_and_candidate_are_required() {
    let errors = EventCreationDraft::default()
        .prepare()
        .expect_err("an empty draft must not create an event");

    assert_eq!(
        errors.name.as_deref(),
        Some("イベント名を入力してください。")
    );
    assert_eq!(
        errors.candidates.as_deref(),
        Some("候補日時を一件以上追加してください。")
    );
}

#[test]
fn anonymous_creation_rejects_oversized_text_and_candidate_sets() {
    let oversized_name = EventCreationDraft {
        name: "集".repeat(EVENT_NAME_MAX_CHARS + 1),
        time_zone: "Asia/Tokyo".to_owned(),
        pending_candidate: candidate("2026-09-18", "19:00"),
        ..EventCreationDraft::default()
    }
    .prepare()
    .expect_err("an anonymous request must not store an unbounded name");
    assert_eq!(
        oversized_name.name.as_deref(),
        Some("イベント名は100文字以内で入力してください。")
    );

    let oversized_note = EventCreationDraft {
        name: "餃子会".to_owned(),
        organizer_note: "話".repeat(ORGANIZER_NOTE_MAX_CHARS + 1),
        time_zone: "Asia/Tokyo".to_owned(),
        pending_candidate: candidate("2026-09-18", "19:00"),
        ..EventCreationDraft::default()
    }
    .prepare()
    .expect_err("an anonymous request must not store an unbounded note");
    assert_eq!(
        oversized_note.organizer_note.as_deref(),
        Some("主催者のひとことは500文字以内で入力してください。")
    );

    let too_many_candidates = EventCreationDraft {
        name: "餃子会".to_owned(),
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: (1..=20)
            .map(|day| candidate(&format!("2026-09-{day:02}"), "19:00"))
            .collect(),
        pending_candidate: candidate("2026-09-21", "19:00"),
        ..EventCreationDraft::default()
    }
    .prepare()
    .expect_err("an anonymous request must bound transaction work");
    assert_eq!(
        too_many_candidates.candidates.as_deref(),
        Some("候補日時は20件以内で入力してください。")
    );
}

#[test]
fn a_partial_or_duplicate_pending_candidate_is_rejected() {
    let partial = EventCreationDraft {
        name: "餃子会".to_owned(),
        time_zone: "Asia/Tokyo".to_owned(),
        pending_candidate: candidate("2026-09-18", ""),
        ..EventCreationDraft::default()
    };
    assert_eq!(
        partial
            .prepare()
            .expect_err("date without time is incomplete")
            .candidates
            .as_deref(),
        Some("日付と開始時刻を両方入力してください。")
    );

    let duplicate = EventCreationDraft {
        name: "餃子会".to_owned(),
        time_zone: "Asia/Tokyo".to_owned(),
        candidates: vec![candidate("2026-09-18", "19:00")],
        pending_candidate: candidate("2026-09-18", "19:00"),
        ..EventCreationDraft::default()
    };
    assert_eq!(
        duplicate
            .prepare()
            .expect_err("the same candidate must not be added twice")
            .candidates
            .as_deref(),
        Some("同じ候補日時がすでに追加されています。")
    );
}

#[test]
fn creation_success_shows_only_the_answering_url() {
    let html = dioxus_ssr::render_element(rsx! {
        CreationSuccess {
            event: public_event(),
            share_url: "http://127.0.0.1:8081/events/7af78527-813b-4cdd-a632-058f3ce885aa".to_owned(),
            organizer_recovery_key: None,
        }
    });

    assert!(
        html.contains("イベントを作りました")
            && html.contains("回答用の共有URL")
            && html.contains("readonly")
            && html.contains("URLをコピー")
            && html.contains("共有URLを開く"),
        "success should make the public URL reusable: {html}"
    );
    assert!(
        !html.contains("organizer_capability") && !html.contains("主催者secret"),
        "organizer authority must not appear in the public sharing UI: {html}"
    );
}

#[test]
fn creation_success_preserves_organizer_authority_when_browser_storage_fails() {
    let html = dioxus_ssr::render_element(rsx! {
        CreationSuccess {
            event: public_event(),
            share_url: "http://127.0.0.1:8081/events/7af78527-813b-4cdd-a632-058f3ce885aa".to_owned(),
            organizer_recovery_key: Some("manual-backup-capability".to_owned()),
        }
    });

    assert!(
        html.contains("主催者用の復旧キー")
            && html.contains("manual-backup-capability")
            && html.contains("この画面を閉じる前")
            && html.contains("readonly"),
        "a capability that could not be stored must remain manually recoverable: {html}"
    );
    assert!(
        !html.contains("events/7af78527-813b-4cdd-a632-058f3ce885aa?"),
        "the public answering URL must not gain organizer authority: {html}"
    );
}

#[test]
fn creation_success_is_focusable_after_the_async_view_change() {
    let html = dioxus_ssr::render_element(rsx! {
        CreationSuccess {
            event: public_event(),
            share_url: "/events/7af78527-813b-4cdd-a632-058f3ce885aa".to_owned(),
            organizer_recovery_key: None,
        }
    });

    assert!(
        html.contains("id=\"creation-success-heading\"")
            && html.contains("tabindex=\"-1\"")
            && html.contains("aria-live=\"polite\""),
        "the success replacement should announce itself and receive programmatic focus: {html}"
    );
}

#[test]
fn public_event_view_is_readable_without_login() {
    let html = dioxus_ssr::render_element(rsx! {
        PublicEventView { event: public_event() }
    });

    assert!(
        html.contains("秋の餃子会")
            && html.contains("焼きたてを囲みたいです")
            && html.contains("2026年9月18日")
            && html.contains("19:00"),
        "the shared view should expose all created event information: {html}"
    );
    assert!(
        !html.contains("ログイン") && !html.contains("プロフィール") && !html.contains("次のStory"),
        "opening a shared event must not expose account setup or internal delivery notes: {html}"
    );
}
