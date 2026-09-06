//! Limited Cloudflare journey, kept separate from native account features.
pub mod api;
pub mod ui;

use crate::domain::{
    Availability, CandidateInput, NewEventInput, ResponseMatrix, ResponseMatrixCandidate,
    ResponseMatrixRow,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const CREATION_KEY: &str = "tsunoru.cloud.creation.v1";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: String,
    pub local_date: String,
    pub local_time: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    pub id: String,
    pub name: String,
    pub organizer_capability: String,
    pub time_zone: String,
    pub organizer_note: Option<String>,
    pub candidates: Vec<Candidate>,
}

impl CreateRequest {
    pub fn new(input: NewEventInput, id: String, organizer_capability: String) -> Self {
        Self {
            id,
            name: input.name,
            organizer_capability,
            time_zone: input.time_zone,
            organizer_note: input.organizer_note,
            candidates: input
                .candidates
                .into_iter()
                .enumerate()
                .map(|(index, candidate)| Candidate {
                    id: format!("{:02}", index + 1),
                    local_date: candidate.local_date,
                    local_time: candidate.local_time,
                })
                .collect(),
        }
    }

    pub fn event(&self) -> Event {
        Event {
            id: self.id.clone(),
            name: self.name.clone(),
            time_zone: self.time_zone.clone(),
            organizer_note: self.organizer_note.clone(),
            candidates: self.candidates.clone(),
        }
    }

    pub fn input(&self) -> NewEventInput {
        NewEventInput {
            name: self.name.clone(),
            organizer_note: self.organizer_note.clone(),
            time_zone: self.time_zone.clone(),
            candidates: self
                .candidates
                .iter()
                .map(|candidate| CandidateInput {
                    local_date: candidate.local_date.clone(),
                    local_time: candidate.local_time.clone(),
                })
                .collect(),
        }
    }

    fn valid(&self) -> bool {
        valid_key(&self.id) && valid_key(&self.organizer_capability) && self.event().valid()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub name: String,
    pub time_zone: String,
    pub organizer_note: Option<String>,
    pub candidates: Vec<Candidate>,
}

impl Event {
    pub fn valid(&self) -> bool {
        valid_id(&self.id)
            && self
                .candidates
                .iter()
                .all(|candidate| valid_id(&candidate.id))
            && self
                .candidates
                .iter()
                .map(|candidate| &candidate.id)
                .collect::<BTreeSet<_>>()
                .len()
                == self.candidates.len()
            && (NewEventInput {
                name: self.name.clone(),
                organizer_note: self.organizer_note.clone(),
                time_zone: self.time_zone.clone(),
                candidates: self
                    .candidates
                    .iter()
                    .map(|candidate| CandidateInput {
                        local_date: candidate.local_date.clone(),
                        local_time: candidate.local_time.clone(),
                    })
                    .collect(),
            })
            .normalized_and_validated()
            .is_ok()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub candidate_id: String,
    pub availability: Availability,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    pub respondent_name: String,
    pub availabilities: Vec<Selection>,
}

impl Answer {
    pub fn prepare(
        event: &Event,
        name: &str,
        choices: &BTreeMap<String, Availability>,
    ) -> Result<Self, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
            return Err("名前は1〜100文字で入力してください。".to_owned());
        }
        if choices.len() != event.candidates.len()
            || event
                .candidates
                .iter()
                .any(|candidate| !choices.contains_key(&candidate.id))
        {
            return Err("すべての候補日時への都合を選んでください。".to_owned());
        }
        Ok(Self {
            respondent_name: name.to_owned(),
            availabilities: event
                .candidates
                .iter()
                .map(|candidate| Selection {
                    candidate_id: candidate.id.clone(),
                    availability: choices[&candidate.id],
                })
                .collect(),
        })
    }

    fn valid(&self) -> bool {
        !self.respondent_name.trim().is_empty()
            && self.respondent_name.chars().count() <= 100
            && !self.respondent_name.chars().any(char::is_control)
            && !self.availabilities.is_empty()
            && self.availabilities.len() <= 20
            && self
                .availabilities
                .iter()
                .all(|choice| valid_id(&choice.candidate_id))
            && self
                .availabilities
                .iter()
                .map(|choice| &choice.candidate_id)
                .collect::<BTreeSet<_>>()
                .len()
                == self.availabilities.len()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationRecord {
    pub request: CreateRequest,
    pub accepted: bool,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseRecord {
    pub event_id: String,
    pub capability: String,
    pub answer: Answer,
    pub accepted: bool,
}

pub trait Store {
    fn read(&self, key: &str) -> Result<Option<String>, String>;
    fn write(&self, key: &str, value: &str) -> Result<(), String>;
    fn remove(&self, key: &str) -> Result<(), String>;
}

pub fn save_creation(store: &impl Store, record: &CreationRecord) -> Result<(), String> {
    if !record.request.valid() {
        return Err("保存するイベントを確認できません。".to_owned());
    }
    // Both writes must succeed before a network request is allowed. The separate
    // organizer entry survives starting another event from the creation screen.
    store.write(
        &organizer_key(&record.request.id),
        &record.request.organizer_capability,
    )?;
    save(store, CREATION_KEY, record)
}

pub fn load_creation(store: &impl Store) -> Result<Option<CreationRecord>, String> {
    let record: Option<CreationRecord> = load(store, CREATION_KEY)?;
    if record
        .as_ref()
        .is_some_and(|record| !record.request.valid())
    {
        return Err(saved_data_error());
    }
    Ok(record)
}

pub fn save_response(store: &impl Store, record: &ResponseRecord) -> Result<(), String> {
    if !valid_id(&record.event_id) || !valid_key(&record.capability) || !record.answer.valid() {
        return Err("保存する回答を確認できません。".to_owned());
    }
    save(store, &response_key(&record.event_id), record)
}

pub fn load_response(store: &impl Store, event_id: &str) -> Result<Option<ResponseRecord>, String> {
    let record: Option<ResponseRecord> = load(store, &response_key(event_id))?;
    if record.as_ref().is_some_and(|record| {
        record.event_id != event_id || !valid_key(&record.capability) || !record.answer.valid()
    }) {
        return Err(saved_data_error());
    }
    Ok(record)
}

pub fn load_organizer(store: &impl Store, event_id: &str) -> Result<Option<String>, String> {
    let key = store.read(&organizer_key(event_id))?;
    if key.as_ref().is_some_and(|key| !valid_key(key)) {
        return Err(saved_data_error());
    }
    Ok(key)
}

fn organizer_key(event_id: &str) -> String {
    format!("tsunoru.cloud.organizer.v1.{event_id}")
}
fn response_key(event_id: &str) -> String {
    format!("tsunoru.cloud.response.v1.{event_id}")
}
fn saved_data_error() -> String {
    "保存していた送信内容を読み込めません。別のブラウザーで新しく送る前に、主催者へ確認してください。".to_owned()
}

fn save(store: &impl Store, key: &str, value: &impl Serialize) -> Result<(), String> {
    store.write(
        key,
        &serde_json::to_string(value).map_err(|_| saved_data_error())?,
    )
}
fn load<T: serde::de::DeserializeOwned>(
    store: &impl Store,
    key: &str,
) -> Result<Option<T>, String> {
    store
        .read(key)?
        .map(|value| {
            if value.len() > 65_536 {
                return Err(saved_data_error());
            }
            serde_json::from_str(&value).map_err(|_| saved_data_error())
        })
        .transpose()
}

pub fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
pub fn valid_key(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct ResponseView {
    pub response_id: String,
    pub respondent_name: String,
    pub availabilities: Vec<Selection>,
}

pub fn response_matrix(
    event: &Event,
    responses: &[ResponseView],
) -> Result<ResponseMatrix, String> {
    let mut rows = Vec::new();
    for response in responses {
        let choices = response
            .availabilities
            .iter()
            .map(|choice| (choice.candidate_id.clone(), choice.availability))
            .collect::<BTreeMap<_, _>>();
        if choices.len() != response.availabilities.len() || choices.len() != event.candidates.len()
        {
            return Err("回答の一覧を確認できませんでした。".to_owned());
        }
        let availabilities = event
            .candidates
            .iter()
            .map(|candidate| choices.get(&candidate.id).copied())
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| "回答の一覧を確認できませんでした。".to_owned())?;
        rows.push(ResponseMatrixRow {
            respondent_name: response.respondent_name.clone(),
            availabilities,
        });
    }
    Ok(ResponseMatrix {
        name: event.name.clone(),
        time_zone: event.time_zone.clone(),
        candidates: event
            .candidates
            .iter()
            .map(|candidate| ResponseMatrixCandidate {
                local_date: candidate.local_date.clone(),
                local_time: candidate.local_time.clone(),
            })
            .collect(),
        responses: rows,
    })
}
