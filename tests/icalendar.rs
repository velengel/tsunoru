#![cfg(feature = "server")]

use tsunoru::calendar::{IcalendarError, IcalendarEvent, render_icalendar};

const EVENT_ID: &str = "7af78527-813b-4cdd-a632-058f3ce885aa";

fn calendar_event() -> IcalendarEvent {
    IcalendarEvent {
        public_id: EVENT_ID.to_owned(),
        name: "秋の餃子会".to_owned(),
        organizer_note: Some("焼きたてを囲みたいです".to_owned()),
        time_zone: "Asia/Tokyo".to_owned(),
        local_date: "2026-09-18".to_owned(),
        local_time: "19:00".to_owned(),
        generated_at_utc: "20260902T123456Z".to_owned(),
    }
}

fn unfold(calendar: &str) -> String {
    calendar.replace("\r\n ", "")
}

#[test]
fn renders_one_self_contained_utc_event_without_inventing_an_end() {
    let calendar = render_icalendar(&calendar_event()).expect("render iCalendar");
    let unfolded = unfold(&calendar);

    for expected in [
        "BEGIN:VCALENDAR\r\n",
        "PRODID:-//TSUNORU//Schedule Coordination//JA\r\n",
        "VERSION:2.0\r\n",
        "BEGIN:VEVENT\r\n",
        "UID:urn:uuid:7af78527-813b-4cdd-a632-058f3ce885aa\r\n",
        "DTSTAMP:20260902T123456Z\r\n",
        "DTSTART:20260918T100000Z\r\n",
        "SUMMARY:秋の餃子会\r\n",
        "DESCRIPTION:焼きたてを囲みたいです\r\n",
        "END:VEVENT\r\n",
        "END:VCALENDAR\r\n",
    ] {
        assert!(
            unfolded.contains(expected),
            "calendar should contain {expected:?}: {unfolded}"
        );
    }
    for forbidden in [
        "DTEND",
        "DURATION",
        "METHOD",
        "ORGANIZER",
        "ATTENDEE",
        "VALARM",
        "VTIMEZONE",
        "TZID",
        "TZOFFSETFROM",
        "TZOFFSETTO",
    ] {
        assert!(
            !unfolded.contains(forbidden),
            "calendar must not invent or expose {forbidden}: {unfolded}"
        );
    }
    assert_eq!(unfolded.matches("BEGIN:VEVENT").count(), 1);
    assert!(calendar.ends_with("\r\n"));
    assert!(!calendar.replace("\r\n", "").contains('\n'));
}

#[test]
fn keeps_uid_stable_while_using_the_injected_generation_time() {
    let first = render_icalendar(&calendar_event()).expect("render first download");
    let mut later = calendar_event();
    later.generated_at_utc = "20260902T130000Z".to_owned();
    let second = render_icalendar(&later).expect("render later download");

    assert!(
        first.contains("UID:urn:uuid:7af78527-813b-4cdd-a632-058f3ce885aa")
            && second.contains("UID:urn:uuid:7af78527-813b-4cdd-a632-058f3ce885aa")
    );
    assert!(first.contains("DTSTAMP:20260902T123456Z"));
    assert!(second.contains("DTSTAMP:20260902T130000Z"));
    assert!(!first.contains("decided_at") && !second.contains("decided_at"));
}

#[test]
fn escapes_text_before_folding_and_blocks_property_injection() {
    let mut event = calendar_event();
    event.name = "相談,持参;確認\\\r\nBEGIN:VEVENT:偽物".to_owned();
    event.organizer_note = Some("一行目\r二行目\nEND:VEVENT:偽物".to_owned());
    let calendar = render_icalendar(&event).expect("render escaped iCalendar");
    let unfolded = unfold(&calendar);

    assert!(unfolded.contains("SUMMARY:相談\\,持参\\;確認\\\\\\nBEGIN:VEVENT:偽物\r\n"));
    assert!(unfolded.contains("DESCRIPTION:一行目\\n二行目\\nEND:VEVENT:偽物\r\n"));
    assert_eq!(
        unfolded.matches("BEGIN:VEVENT").count(),
        2,
        "one real component plus one escaped TEXT token should remain on one logical line: {unfolded}"
    );
    assert_eq!(
        unfolded.matches("\r\nBEGIN:VEVENT:").count(),
        0,
        "untrusted text must not create another property: {unfolded}"
    );
    assert_eq!(unfolded.matches("\r\nEND:VEVENT:").count(), 0);
}

#[test]
fn folds_every_physical_line_at_utf8_boundaries_within_75_octets() {
    let mut event = calendar_event();
    event.name = "秋".repeat(100);
    let calendar = render_icalendar(&event).expect("render long UTF-8 summary");

    for line in calendar.split("\r\n").filter(|line| !line.is_empty()) {
        assert!(
            line.len() <= 75,
            "physical content line is {} octets: {line:?}",
            line.len()
        );
        assert!(line.is_char_boundary(line.len()));
    }
    assert!(calendar.contains("\r\n "));
    assert!(unfold(&calendar).contains(&format!("SUMMARY:{}\r\n", event.name)));
}

#[test]
fn resolves_dst_overlap_and_gap_to_unambiguous_utc_instants() {
    let mut overlap = calendar_event();
    overlap.time_zone = "America/New_York".to_owned();
    overlap.local_date = "2026-11-01".to_owned();
    overlap.local_time = "01:30".to_owned();
    let overlap = unfold(&render_icalendar(&overlap).expect("render DST overlap"));
    assert!(overlap.contains("DTSTART:20261101T053000Z\r\n"));
    assert!(!overlap.contains("VTIMEZONE") && !overlap.contains("TZID"));

    let mut gap = calendar_event();
    gap.time_zone = "America/New_York".to_owned();
    gap.local_date = "2026-03-08".to_owned();
    gap.local_time = "02:30".to_owned();
    let gap = unfold(&render_icalendar(&gap).expect("render DST gap"));
    assert!(gap.contains("DTSTART:20260308T073000Z\r\n"));
    assert!(!gap.contains("VTIMEZONE") && !gap.contains("TZID"));
}

#[test]
fn rejects_values_that_cannot_form_one_safe_calendar() {
    let mut invalid_id = calendar_event();
    invalid_id.public_id = "not-a-uuid".to_owned();
    assert!(matches!(
        render_icalendar(&invalid_id),
        Err(IcalendarError::InvalidData)
    ));

    let mut invalid_zone = calendar_event();
    invalid_zone.time_zone = "Mars/Olympus".to_owned();
    assert!(matches!(
        render_icalendar(&invalid_zone),
        Err(IcalendarError::InvalidData)
    ));

    let mut invalid_stamp = calendar_event();
    invalid_stamp.generated_at_utc = "2026-09-02 12:34:56".to_owned();
    assert!(matches!(
        render_icalendar(&invalid_stamp),
        Err(IcalendarError::InvalidData)
    ));

    let mut control = calendar_event();
    control.name = "危険\u{0001}な名前".to_owned();
    assert!(matches!(
        render_icalendar(&control),
        Err(IcalendarError::InvalidData)
    ));
}
