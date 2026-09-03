//! Small, deterministic iCalendar serialization for one decided event.

#![cfg(feature = "server")]

use chrono::{DateTime, Duration, FixedOffset, LocalResult, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use std::fmt;
use uuid::Uuid;

use crate::domain::{EVENT_NAME_MAX_CHARS, ORGANIZER_NOTE_MAX_CHARS};

const CONTENT_LINE_OCTETS: usize = 75;
const GAP_SEARCH_MINUTES: i64 = 48 * 60;

/// All public data needed to generate one non-recurring calendar event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IcalendarEvent {
    pub public_id: String,
    pub name: String,
    pub organizer_note: Option<String>,
    pub time_zone: String,
    pub local_date: String,
    pub local_time: String,
    pub generated_at_utc: String,
}

/// Calendar output is all-or-nothing when persisted data cannot be represented safely.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IcalendarError {
    InvalidData,
}

impl fmt::Display for IcalendarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("event data cannot form a safe iCalendar object")
    }
}

impl std::error::Error for IcalendarError {}

/// Return the UTC content value used for a newly generated `DTSTAMP`.
pub fn current_icalendar_timestamp() -> String {
    Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

/// Render one complete UTF-8 iCalendar object with CRLF content lines.
pub fn render_icalendar(event: &IcalendarEvent) -> Result<String, IcalendarError> {
    validate_event(event)?;

    let local = NaiveDateTime::parse_from_str(
        &format!("{} {}", event.local_date, event.local_time),
        "%Y-%m-%d %H:%M",
    )
    .map_err(|_| IcalendarError::InvalidData)?;
    let time_zone = event
        .time_zone
        .parse::<Tz>()
        .map_err(|_| IcalendarError::InvalidData)?;
    let start_utc = resolve_event_start(time_zone, local)?;
    let summary = escape_text(&event.name)?;
    let description = event
        .organizer_note
        .as_deref()
        .filter(|note| !note.is_empty())
        .map(escape_text)
        .transpose()?;

    let mut output = String::new();
    for line in [
        "BEGIN:VCALENDAR".to_owned(),
        "PRODID:-//TSUNORU//Schedule Coordination//JA".to_owned(),
        "VERSION:2.0".to_owned(),
        "CALSCALE:GREGORIAN".to_owned(),
        "BEGIN:VEVENT".to_owned(),
        format!("UID:urn:uuid:{}", event.public_id),
        format!("DTSTAMP:{}", event.generated_at_utc),
        format!("DTSTART:{}", start_utc.format("%Y%m%dT%H%M%SZ")),
        format!("SUMMARY:{summary}"),
    ] {
        append_folded_content_line(&mut output, &line)?;
    }
    if let Some(description) = description {
        append_folded_content_line(&mut output, &format!("DESCRIPTION:{description}"))?;
    }
    append_folded_content_line(&mut output, "END:VEVENT")?;
    append_folded_content_line(&mut output, "END:VCALENDAR")?;
    Ok(output)
}

fn validate_event(event: &IcalendarEvent) -> Result<(), IcalendarError> {
    Uuid::parse_str(&event.public_id).map_err(|_| IcalendarError::InvalidData)?;
    if event.name.trim().is_empty()
        || event.name.chars().count() > EVENT_NAME_MAX_CHARS
        || event
            .organizer_note
            .as_deref()
            .is_some_and(|note| note.chars().count() > ORGANIZER_NOTE_MAX_CHARS)
        || event.time_zone.is_empty()
        || event.time_zone.len() > 64
        || !event
            .time_zone
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-' | b'+'))
        || event.generated_at_utc.len() != 16
        || NaiveDateTime::parse_from_str(&event.generated_at_utc, "%Y%m%dT%H%M%SZ").is_err()
    {
        return Err(IcalendarError::InvalidData);
    }
    Ok(())
}

fn resolve_event_start(
    time_zone: Tz,
    local: NaiveDateTime,
) -> Result<DateTime<Utc>, IcalendarError> {
    match time_zone.from_local_datetime(&local) {
        LocalResult::Single(date_time) => Ok(date_time.with_timezone(&Utc)),
        LocalResult::Ambiguous(first, _) => Ok(first.with_timezone(&Utc)),
        LocalResult::None => {
            let seconds = offset_immediately_before_gap(time_zone, local)?;
            let offset = FixedOffset::east_opt(seconds).ok_or(IcalendarError::InvalidData)?;
            offset
                .from_local_datetime(&local)
                .single()
                .map(|date_time| date_time.with_timezone(&Utc))
                .ok_or(IcalendarError::InvalidData)
        }
    }
}

fn offset_immediately_before_gap(
    time_zone: Tz,
    local: NaiveDateTime,
) -> Result<i32, IcalendarError> {
    for minutes in 1..=GAP_SEARCH_MINUTES {
        let Some(prior) = local.checked_sub_signed(Duration::minutes(minutes)) else {
            break;
        };
        match time_zone.from_local_datetime(&prior) {
            LocalResult::Single(date_time) => return Ok(offset_seconds(&date_time)),
            LocalResult::Ambiguous(_, last) => return Ok(offset_seconds(&last)),
            LocalResult::None => {}
        }
    }
    Err(IcalendarError::InvalidData)
}

fn offset_seconds(date_time: &DateTime<Tz>) -> i32 {
    date_time.offset().fix().local_minus_utc()
}

fn escape_text(value: &str) -> Result<String, IcalendarError> {
    let mut escaped = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            ',' => escaped.push_str("\\,"),
            ';' => escaped.push_str("\\;"),
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                escaped.push_str("\\n");
            }
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push('\t'),
            character if character.is_control() => return Err(IcalendarError::InvalidData),
            character => escaped.push(character),
        }
    }
    Ok(escaped)
}

fn append_folded_content_line(
    output: &mut String,
    content_line: &str,
) -> Result<(), IcalendarError> {
    if content_line.contains(['\r', '\n']) {
        return Err(IcalendarError::InvalidData);
    }

    let mut start = 0;
    let mut first = true;
    while start < content_line.len() {
        let limit = if first {
            CONTENT_LINE_OCTETS
        } else {
            CONTENT_LINE_OCTETS - 1
        };
        let remaining = &content_line[start..];
        let mut chunk_bytes = remaining.len().min(limit);
        while chunk_bytes > 0 && !remaining.is_char_boundary(chunk_bytes) {
            chunk_bytes -= 1;
        }
        if chunk_bytes == 0 {
            return Err(IcalendarError::InvalidData);
        }
        if !first {
            output.push_str("\r\n ");
        }
        output.push_str(&remaining[..chunk_bytes]);
        start += chunk_bytes;
        first = false;
    }
    output.push_str("\r\n");
    Ok(())
}
