use dioxus::prelude::*;

#[test]
fn limited_entry_explains_the_trial_code_without_account_navigation() {
    let html = render(|| {
        rsx! {
            tsunoru::cloud::ui::TrialCodeForm { busy: false, message: String::new(), on_submit: |_| {} }
        }
    });
    assert!(
        html.contains("試用コード"),
        "limited access must ask for the trial code: {html}"
    );
    assert!(!html.contains("href=\"/history\""));
    assert!(html.contains("type=\"password\""));
}

#[test]
fn responder_can_choose_each_date_without_exposing_organizer_actions() {
    let html = render(
        || rsx! { tsunoru::cloud::ui::AnswerForm { event: event(), busy:false, on_submit: |_|{} } },
    );
    assert_eq!(html.matches("type=\"radio\"").count(), 6);
    assert!(
        html.contains("name=\"availability-first\"")
            && html.contains("name=\"availability-second\"")
    );
    assert!(html.contains("2026年9月15日 19:00") && html.contains("2026年9月16日 19:00"));
    assert!(html.contains("あなたの名前") && html.contains("回答を送る"));
    assert!(!html.contains("summary") && !html.contains("organizer_capability"));
}

fn render(component: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(component);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn shared_link_contains_only_the_event_id_and_manual_copy_fallback() {
    let html =
        dioxus_ssr::render_element(rsx! { tsunoru::cloud::ui::CreatedView { event:event() } });
    assert!(html.contains("value=\"/events/test-event\""));
    assert!(html.contains("readonly=true") && html.contains("URLをコピー"));
    assert!(!html.contains("capability") && !html.contains("/history"));
}

#[test]
fn accepted_answer_does_not_promise_editing_or_other_peoples_answers() {
    let html = dioxus_ssr::render_element(rsx! { tsunoru::cloud::ui::AnswerAccepted {} });
    assert!(html.contains("回答を送りました") && html.contains("主催者が回答を確認できます"));
    assert!(!html.contains("みんなの回答") && !html.contains("コメント") && !html.contains("編集"));
}

#[test]
fn rejected_creation_form_retains_fields_for_correction() {
    let html = render(|| {
        rsx! {
            tsunoru::cloud::ui::CreateForm {
                busy:false,
                initial:Some(tsunoru::domain::NewEventInput {
                    name:"元のイベント".into(), organizer_note:Some("元のメモ".into()), time_zone:"Invalid/Zone".into(),
                    candidates:vec![tsunoru::domain::CandidateInput{local_date:"2026-09-15".into(),local_time:"19:00".into()}],
                }),
                on_submit: |_| {},
            }
        }
    });
    assert!(
        html.contains("元のイベント") && html.contains("元のメモ") && html.contains("Invalid/Zone")
    );
    assert!(html.contains("2026年9月15日 19:00") && html.contains("候補を削除"));
}

fn event() -> tsunoru::cloud::Event {
    use tsunoru::cloud::{Candidate, Event};
    Event {
        id: "test-event".into(),
        name: "餃子会".into(),
        time_zone: "Asia/Tokyo".into(),
        organizer_note: None,
        candidates: vec![
            Candidate {
                id: "first".into(),
                local_date: "2026-09-15".into(),
                local_time: "19:00".into(),
            },
            Candidate {
                id: "second".into(),
                local_date: "2026-09-16".into(),
                local_time: "19:00".into(),
            },
        ],
    }
}

#[derive(Default)]
struct MemoryStore {
    values: std::cell::RefCell<std::collections::BTreeMap<String, String>>,
    fail: std::cell::Cell<bool>,
}
impl tsunoru::cloud::Store for MemoryStore {
    fn read(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.values.borrow().get(key).cloned())
    }
    fn write(&self, key: &str, value: &str) -> Result<(), String> {
        if self.fail.get() {
            Err("unavailable".into())
        } else {
            self.values.borrow_mut().insert(key.into(), value.into());
            Ok(())
        }
    }
    fn remove(&self, key: &str) -> Result<(), String> {
        self.values.borrow_mut().remove(key);
        Ok(())
    }
}

#[test]
fn lost_creation_response_reloads_the_identical_id_authority_and_payload() {
    use tsunoru::cloud::*;
    let store = MemoryStore::default();
    let record = CreationRecord {
        request: CreateRequest {
            id: "a".repeat(64),
            name: "餃子会".into(),
            organizer_capability: "b".repeat(64),
            time_zone: "Asia/Tokyo".into(),
            organizer_note: Some("夕食".into()),
            candidates: event().candidates,
        },
        accepted: false,
    };
    save_creation(&store, &record).unwrap();
    let loaded = load_creation(&store).unwrap().unwrap();
    assert!(loaded == record);
    assert!(
        load_organizer(&store, &record.request.id)
            .unwrap()
            .as_deref()
            == Some(&record.request.organizer_capability)
    );
    // A later event must not remove the earlier organizer's retained authority.
    let mut later = record.clone();
    later.request.id = "c".repeat(64);
    later.request.organizer_capability = "d".repeat(64);
    save_creation(&store, &later).unwrap();
    assert!(
        load_organizer(&store, &record.request.id)
            .unwrap()
            .as_deref()
            == Some(&record.request.organizer_capability)
    );
}

#[test]
fn storage_failure_does_not_replace_the_durable_pending_operation() {
    use tsunoru::cloud::*;
    let store = MemoryStore::default();
    let record = ResponseRecord {
        event_id: "test-event".into(),
        capability: "a".repeat(64),
        answer: Answer {
            respondent_name: "同名".into(),
            availabilities: vec![Selection {
                candidate_id: "first".into(),
                availability: tsunoru::domain::Availability::Maybe,
            }],
        },
        accepted: false,
    };
    save_response(&store, &record).unwrap();
    store.fail.set(true);
    let mut changed = record.clone();
    changed.answer.respondent_name = "別の内容".into();
    assert!(save_response(&store, &changed).is_err());
    assert!(load_response(&store, "test-event").unwrap() == Some(record));
}

#[test]
fn response_reload_preserves_the_accepted_marker_and_rejects_corrupt_storage() {
    use tsunoru::cloud::*;
    let store = MemoryStore::default();
    let record = ResponseRecord {
        event_id: "test-event".into(),
        capability: "a".repeat(64),
        answer: Answer {
            respondent_name: "私".into(),
            availabilities: vec![Selection {
                candidate_id: "first".into(),
                availability: tsunoru::domain::Availability::Available,
            }],
        },
        accepted: true,
    };
    save_response(&store, &record).unwrap();
    assert!(load_response(&store, "test-event").unwrap() == Some(record));
    store
        .values
        .borrow_mut()
        .insert("tsunoru.cloud.creation.v1".into(), "{broken".into());
    assert!(load_creation(&store).is_err());
}

#[test]
fn answers_and_result_columns_match_candidate_ids_instead_of_response_order() {
    use tsunoru::{cloud::*, domain::Availability::*};
    let event = event();
    let choices =
        std::collections::BTreeMap::from([("second".into(), Maybe), ("first".into(), Available)]);
    let answer = Answer::prepare(&event, " 同名 ", &choices).unwrap();
    assert_eq!(answer.respondent_name, "同名");
    let responses = vec![ResponseView {
        response_id: "r".into(),
        respondent_name: "同名".into(),
        availabilities: vec![
            Selection {
                candidate_id: "second".into(),
                availability: Maybe,
            },
            Selection {
                candidate_id: "first".into(),
                availability: Available,
            },
        ],
    }];
    let matrix = response_matrix(&event, &responses).unwrap();
    assert_eq!(matrix.responses[0].availabilities, vec![Available, Maybe]);
    let missing = std::collections::BTreeMap::from([("first".into(), Available)]);
    assert!(Answer::prepare(&event, "私", &missing).is_err());
    let mut extra = choices;
    extra.insert("foreign".into(), Unavailable);
    assert!(Answer::prepare(&event, "私", &extra).is_err());
}
