use super::*;
use chrono::{Datelike, NaiveDate, NaiveTime, TimeZone};
use chrono_tz::Tz;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use worker::wasm_bindgen::JsValue;

const MAX_CANDIDATES: usize = 20;
const MAX_ORGANIZER_NOTE_CHARS: usize = 500;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewEvent {
    id: String,
    name: String,
    organizer_note: Option<String>,
    time_zone: String,
    organizer_capability: String,
    candidates: Vec<Candidate>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    id: String,
    local_date: String,
    local_time: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NewResponse {
    respondent_name: String,
    availabilities: Vec<Selection>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    candidate_id: String,
    availability: Availability,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Availability {
    Available,
    Maybe,
    Unavailable,
}

#[derive(Deserialize, Serialize)]
struct PublicEvent {
    id: String,
    name: String,
}

#[derive(Deserialize, Serialize)]
struct EventMetadata {
    id: String,
    name: String,
    organizer_note: Option<String>,
    time_zone: String,
}

fn datetime_valid(candidate: &Candidate, time_zone: Tz) -> bool {
    let date = &candidate.local_date;
    let time = &candidate.local_time;
    if date.len() != 10
        || time.len() != 5
        || date.bytes().enumerate().any(|(index, byte)| {
            if matches!(index, 4 | 7) {
                byte != b'-'
            } else {
                !byte.is_ascii_digit()
            }
        })
        || time.bytes().enumerate().any(|(index, byte)| {
            if index == 2 {
                byte != b':'
            } else {
                !byte.is_ascii_digit()
            }
        })
    {
        return false;
    }
    let (Ok(date), Ok(time)) = (
        NaiveDate::parse_from_str(date, "%Y-%m-%d"),
        NaiveTime::parse_from_str(time, "%H:%M"),
    ) else {
        return false;
    };
    // Tz uses its bundled IANA rules; chrono::Local would use JS Date on wasm
    // and silently choose an instant for missing or ambiguous wall-clock times.
    date.year() >= 1
        && time_zone
            .from_local_datetime(&date.and_time(time))
            .single()
            .is_some()
}

pub(super) async fn create_event(request: &mut Request, env: &Env) -> ApiResult<Response> {
    let mut input: NewEvent = body(request).await?;
    input.name = input.name.trim().to_owned();
    input.organizer_note = input
        .organizer_note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty())
        .map(ToOwned::to_owned);
    input.time_zone = input.time_zone.trim().to_owned();
    if !identifier_valid(&input.id)
        || !name_valid(&input.name)
        || input
            .organizer_note
            .as_deref()
            .is_some_and(|note| note.chars().count() > MAX_ORGANIZER_NOTE_CHARS)
        || input.time_zone.len() > 64
        || !capability_valid(&input.organizer_capability)
        || input.candidates.is_empty()
        || input.candidates.len() > MAX_CANDIDATES
    {
        return Err(ApiError::invalid());
    }
    let time_zone = input
        .time_zone
        .parse::<Tz>()
        .map_err(|_| ApiError::invalid())?;
    input.candidates.sort_by(|a, b| a.id.cmp(&b.id));
    let mut local_datetimes = BTreeSet::new();
    for candidate in &mut input.candidates {
        candidate.local_date = candidate.local_date.trim().to_owned();
        candidate.local_time = candidate.local_time.trim().to_owned();
        if !identifier_valid(&candidate.id)
            || !datetime_valid(candidate, time_zone)
            || !local_datetimes.insert((candidate.local_date.clone(), candidate.local_time.clone()))
        {
            return Err(ApiError::invalid());
        }
    }
    if input.candidates.windows(2).any(|w| w[0].id == w[1].id) {
        return Err(ApiError::invalid());
    }
    let capability_hash = hash(&input.organizer_capability);
    let payload_hash = hash(
        &serde_json::to_string(&(
            &input.id,
            &input.name,
            &input.organizer_note,
            &input.time_zone,
            &input.candidates,
        ))
        .map_err(|_| ApiError::invalid())?,
    );
    let candidates = serde_json::to_string(&input.candidates).map_err(|_| ApiError::invalid())?;
    let db = env.d1("DB")?;
    // changes() refers to the immediately preceding statement in this atomic batch.
    // An ID conflict must not append candidates to another organizer's event.
    let result = db.batch(vec![
        db.prepare("INSERT INTO events(id,name,organizer_note,time_zone,organizer_capability_hash,creation_payload_hash) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO NOTHING")
            .bind(&[input.id.clone().into(), input.name.clone().into(), input.organizer_note.clone().map_or(JsValue::NULL, Into::into), input.time_zone.clone().into(), capability_hash.clone().into(), payload_hash.clone().into()])?,
        db.prepare("INSERT INTO candidates(event_id,id,local_date,local_time) SELECT ?1,json_extract(value,'$.id'),json_extract(value,'$.local_date'),json_extract(value,'$.local_time') FROM json_each(?2) WHERE changes() = 1")
            .bind(&[input.id.clone().into(), candidates.into()])?,
        db.prepare("SELECT id,name FROM events WHERE id=?1 AND organizer_capability_hash=?2 AND creation_payload_hash=?3")
            .bind(&[input.id.into(), capability_hash.into(), payload_hash.into()])?,
    ]).await?;
    let saved = result[2]
        .results::<PublicEvent>()?
        .into_iter()
        .next()
        .ok_or(ApiError::new(409, "event_conflict"))?;
    let created = result[0]
        .meta()?
        .and_then(|m| m.changes)
        .unwrap_or_default()
        > 0;
    json_response(if created { 201 } else { 200 }, &saved)
}

pub(super) async fn get_event(id: &str, env: &Env) -> ApiResult<Response> {
    let db = env.d1("DB")?;
    let event: EventMetadata = db
        .prepare("SELECT id,name,organizer_note,time_zone FROM events WHERE id=?1")
        .bind(&[id.into()])?
        .first(None)
        .await?
        .ok_or(ApiError::new(404, "event_not_found"))?;
    let candidates: Vec<Candidate> = db
        .prepare("SELECT id,local_date,local_time FROM candidates WHERE event_id=?1 ORDER BY id")
        .bind(&[id.into()])?
        .all()
        .await?
        .results()?;
    json_response(
        200,
        &json!({"id":event.id,"name":event.name,"organizer_note":event.organizer_note,"time_zone":event.time_zone,"candidates":candidates}),
    )
}

#[derive(Deserialize)]
struct StoredResponse {
    id: String,
    event_id: String,
    payload_hash: String,
}

pub(super) async fn submit_response(
    id: &str,
    request: &mut Request,
    env: &Env,
) -> ApiResult<Response> {
    let capability = header_capability(request, "x-response-capability")?;
    let mut input: NewResponse = body(request).await?;
    input.respondent_name = input.respondent_name.trim().to_owned();
    if !name_valid(&input.respondent_name)
        || input.availabilities.is_empty()
        || input.availabilities.len() > MAX_CANDIDATES
        || input
            .availabilities
            .iter()
            .any(|v| !identifier_valid(&v.candidate_id))
    {
        return Err(ApiError::invalid());
    }
    input
        .availabilities
        .sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
    if input
        .availabilities
        .windows(2)
        .any(|w| w[0].candidate_id == w[1].candidate_id)
    {
        return Err(ApiError::invalid());
    }
    let capability_hash = hash(&capability);
    let payload_hash =
        hash(&serde_json::to_string(&(id, &input)).map_err(|_| ApiError::invalid())?);
    let choices = serde_json::to_string(&input.availabilities).map_err(|_| ApiError::invalid())?;
    let db = env.d1("DB")?;
    let result = db.batch(vec![
        // Check the candidate set in the same transaction as the answer insert.
        // Unique capability + no update makes identical and competing retries safe.
        db.prepare(r#"
            INSERT INTO responses(event_id,response_capability_hash,respondent_name,payload_hash)
            SELECT ?1,?2,?3,?4
            WHERE EXISTS(SELECT 1 FROM events WHERE id=?1)
              AND (SELECT COUNT(*) FROM candidates WHERE event_id=?1)=json_array_length(?5)
              AND NOT EXISTS(
                SELECT 1 FROM json_each(?5) AS choice
                WHERE NOT EXISTS(SELECT 1 FROM candidates
                    WHERE event_id=?1 AND id=json_extract(choice.value,'$.candidate_id'))
              )
            ON CONFLICT(response_capability_hash) DO NOTHING
        "#).bind(&[id.into(),capability_hash.clone().into(),input.respondent_name.into(),payload_hash.clone().into(),choices.clone().into()])?,
        db.prepare(r#"
            INSERT INTO answers(event_id,response_id,candidate_id,availability)
            SELECT r.event_id,r.id,json_extract(choice.value,'$.candidate_id'),json_extract(choice.value,'$.availability')
            FROM responses r,json_each(?4) AS choice
            WHERE r.event_id=?1 AND r.response_capability_hash=?2 AND r.payload_hash=?3
            ON CONFLICT(response_id,candidate_id) DO NOTHING
        "#).bind(&[id.into(),capability_hash.clone().into(),payload_hash.clone().into(),choices.into()])?,
        db.prepare("SELECT id,event_id,payload_hash FROM responses WHERE response_capability_hash=?1")
            .bind(&[capability_hash.into()])?,
        db.prepare("SELECT id,name FROM events WHERE id=?1").bind(&[id.into()])?,
    ]).await?;
    if result[3].results::<PublicEvent>()?.is_empty() {
        return Err(ApiError::new(404, "event_not_found"));
    }
    let saved = result[2]
        .results::<StoredResponse>()?
        .into_iter()
        .next()
        .ok_or(ApiError::new(400, "candidate_set_mismatch"))?;
    if saved.event_id != id || saved.payload_hash != payload_hash {
        return Err(ApiError::new(409, "response_conflict"));
    }
    let created = result[0]
        .meta()?
        .and_then(|m| m.changes)
        .unwrap_or_default()
        > 0;
    json_response(
        if created { 201 } else { 200 },
        &json!({"event_id":id,"response_id":saved.id}),
    )
}

#[derive(Deserialize)]
struct AnswerRow {
    response_id: String,
    respondent_name: String,
    candidate_id: String,
    availability: Availability,
}

#[derive(Serialize)]
struct ResponseView {
    response_id: String,
    respondent_name: String,
    availabilities: Vec<Selection>,
}

pub(super) async fn get_responses(id: &str, request: &Request, env: &Env) -> ApiResult<Response> {
    let capability = header_capability(request, "x-organizer-capability")?;
    let db = env.d1("DB")?;
    let capability_hash = hash(&capability);
    let authorized = db
        .prepare("SELECT id,name FROM events WHERE id=?1 AND organizer_capability_hash=?2")
        .bind(&[id.into(), capability_hash.clone().into()])?
        .first::<PublicEvent>(None)
        .await?;
    if authorized.is_none() {
        return Err(ApiError::new(403, "forbidden"));
    }
    // Re-check authorization in the data query; never project hashes or credentials.
    let rows = db
        .prepare(
            r#"
        SELECT r.id AS response_id,r.respondent_name,a.candidate_id,a.availability
        FROM responses r JOIN answers a ON a.response_id=r.id AND a.event_id=r.event_id
        JOIN events e ON e.id=r.event_id
        WHERE e.id=?1 AND e.organizer_capability_hash=?2
        ORDER BY r.id,a.candidate_id
    "#,
        )
        .bind(&[id.into(), capability_hash.into()])?
        .all()
        .await?
        .results::<AnswerRow>()?;
    let mut responses = BTreeMap::<String, ResponseView>::new();
    for row in rows {
        let response = responses
            .entry(row.response_id.clone())
            .or_insert_with(|| ResponseView {
                response_id: row.response_id,
                respondent_name: row.respondent_name,
                availabilities: Vec::new(),
            });
        response.availabilities.push(Selection {
            candidate_id: row.candidate_id,
            availability: row.availability,
        });
    }
    json_response(
        200,
        &json!({"responses":responses.into_values().collect::<Vec<_>>()}),
    )
}
