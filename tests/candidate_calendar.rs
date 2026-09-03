use dioxus::prelude::*;
use tsunoru::{
    domain::CandidateInput,
    ui::{
        CalendarCandidateChange, CalendarDateToggleCallback, CalendarMonth, CandidateCalendar,
        DEFAULT_CANDIDATE_TIME, toggle_calendar_candidate,
    },
};

fn candidate(date: &str, time: &str) -> CandidateInput {
    CandidateInput {
        local_date: date.to_owned(),
        local_time: time.to_owned(),
    }
}

#[test]
fn calendar_month_handles_leap_year_weekday_and_year_boundaries() {
    let september = CalendarMonth::new(2026, 9).expect("valid month");
    assert_eq!(september.days_in_month(), 30);
    assert_eq!(
        september.leading_blank_days(),
        2,
        "2026-09-01 is Tuesday in a Sunday-first calendar"
    );
    assert_eq!(september.iso_date(7).as_deref(), Some("2026-09-07"));

    let leap_february = CalendarMonth::new(2028, 2).expect("valid leap month");
    assert_eq!(leap_february.days_in_month(), 29);
    assert_eq!(
        CalendarMonth::new(2026, 1).unwrap().previous(),
        Some(CalendarMonth::new(2025, 12).unwrap())
    );
    assert_eq!(
        CalendarMonth::new(2026, 12).unwrap().next(),
        Some(CalendarMonth::new(2027, 1).unwrap())
    );
    assert_eq!(CalendarMonth::new(1, 1).unwrap().previous(), None);
    assert_eq!(CalendarMonth::new(9999, 12).unwrap().next(), None);
    assert!(CalendarMonth::new(2026, 13).is_none());
    assert!(CalendarMonth::from_iso_date("2026-9-01").is_none());
    assert!(CalendarMonth::from_iso_date("2026-02-29").is_none());
}

#[test]
fn calendar_click_toggles_one_exact_datetime_and_sorts_added_candidates() {
    let mut candidates = vec![candidate("2026-09-20", "18:00")];

    assert_eq!(
        toggle_calendar_candidate(&mut candidates, "2026-09-18", "19:00").unwrap(),
        CalendarCandidateChange::Added
    );
    assert_eq!(
        candidates,
        vec![
            candidate("2026-09-18", "19:00"),
            candidate("2026-09-20", "18:00")
        ]
    );

    assert_eq!(
        toggle_calendar_candidate(&mut candidates, "2026-09-18", "19:00").unwrap(),
        CalendarCandidateChange::Removed
    );
    assert_eq!(candidates, vec![candidate("2026-09-20", "18:00")]);

    assert_eq!(
        toggle_calendar_candidate(&mut candidates, "2026-09-20", "19:00").unwrap(),
        CalendarCandidateChange::Added,
        "a different time on the same day remains a distinct candidate"
    );
    assert_eq!(candidates.len(), 2);
}

#[test]
fn calendar_click_rejects_bad_time_and_the_twenty_first_candidate_without_mutation() {
    let mut candidates = vec![candidate("2026-09-18", "19:00")];
    let before = candidates.clone();
    let error = toggle_calendar_candidate(&mut candidates, "2026-09-19", "7pm")
        .expect_err("calendar time must use the same domain shape");
    assert_eq!(error, "時刻は24時間表記のHH:MMで入力してください。");
    assert_eq!(candidates, before);

    let mut full = (1..=20)
        .map(|day| candidate(&format!("2026-09-{day:02}"), "19:00"))
        .collect::<Vec<_>>();
    let before = full.clone();
    let error = toggle_calendar_candidate(&mut full, "2026-09-21", "19:00")
        .expect_err("calendar must retain the domain transaction bound");
    assert_eq!(error, "候補日時は20件以内で入力してください。");
    assert_eq!(full, before);
}

#[test]
fn inline_calendar_uses_native_toggle_buttons_without_claiming_an_incomplete_grid() {
    let month = CalendarMonth::new(2026, 9).unwrap();
    let html = dioxus_ssr::render_element(rsx! {
        CandidateCalendar {
            month,
            candidate_time: DEFAULT_CANDIDATE_TIME.to_owned(),
            candidates: vec![candidate("2026-09-18", "19:00")],
            on_toggle: CalendarDateToggleCallback::from(move |_: String| {}),
        }
    });

    for expected in [
        "2026年9月",
        "aria-live=\"polite\"",
        "前の月を表示",
        "次の月を表示",
        "日曜日",
        "土曜日",
        "aria-pressed=true",
        "2026年9月18日 金曜日 19:00の候補から削除",
        "2026年9月19日 土曜日 19:00の候補に追加",
    ] {
        assert!(html.contains(expected), "missing {expected:?}: {html}");
    }
    assert_eq!(
        html.matches("class=\"candidate-calendar-day").count(),
        30,
        "every day in September should be one native button: {html}"
    );
    assert!(
        !html.contains("role=\"grid\"") && !html.contains("role=\"gridcell\""),
        "arrow-key grid semantics must not be promised before they are implemented: {html}"
    );
}
