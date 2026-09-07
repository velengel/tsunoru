//! Calendar, candidate editor, and response table shared by both deployments.
use crate::domain::{
    Availability, CandidateInput, EVENT_CANDIDATE_MAX_COUNT, EventCreationDraft,
    EventCreationErrors, OrganizerResponseMatrix,
};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub const DEFAULT_CANDIDATE_TIME: &str = "19:00";

/// One Gregorian month used only to lay out the inline candidate calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarMonth {
    pub year: i32,
    pub month: u8,
}

impl CalendarMonth {
    pub fn new(year: i32, month: u8) -> Option<Self> {
        ((1..=9999).contains(&year) && (1..=12).contains(&month)).then_some(Self { year, month })
    }

    pub fn from_iso_date(value: &str) -> Option<Self> {
        let bytes = value.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return None;
        }
        let mut parts = value.split('-');
        let year = parts.next()?.parse().ok()?;
        let month = parts.next()?.parse().ok()?;
        let day = parts.next()?.parse::<u8>().ok()?;
        if parts.next().is_some() {
            return None;
        }
        let month = Self::new(year, month)?;
        (day >= 1 && day <= month.days_in_month()).then_some(month)
    }

    pub fn previous(self) -> Option<Self> {
        if self.month > 1 {
            Self::new(self.year, self.month - 1)
        } else {
            Self::new(self.year.checked_sub(1)?, 12)
        }
    }

    pub fn next(self) -> Option<Self> {
        if self.month < 12 {
            Self::new(self.year, self.month + 1)
        } else {
            Self::new(self.year.checked_add(1)?, 1)
        }
    }

    pub fn days_in_month(self) -> u8 {
        match self.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if is_gregorian_leap_year(self.year) => 29,
            2 => 28,
            _ => unreachable!("CalendarMonth validates its month"),
        }
    }

    /// Sunday-first offset for the first day of this month.
    pub fn leading_blank_days(self) -> u8 {
        weekday_index(self.year, self.month, 1)
    }

    pub fn iso_date(self, day: u8) -> Option<String> {
        (day >= 1 && day <= self.days_in_month())
            .then(|| format!("{:04}-{:02}-{day:02}", self.year, self.month))
    }

    fn label(self) -> String {
        format!("{}年{}月", self.year, self.month)
    }
}

fn is_gregorian_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Weekday index where Sunday is zero, valid for the proleptic Gregorian calendar.
fn weekday_index(year: i32, month: u8, day: u8) -> u8 {
    let adjusted_year = year - i32::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let shifted_month = i32::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i32::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days_since_epoch = i64::from(era * 146_097 + day_of_era - 719_468);
    (days_since_epoch + 4).rem_euclid(7) as u8
}

fn weekday_name(index: u8) -> &'static str {
    [
        "日曜日",
        "月曜日",
        "火曜日",
        "水曜日",
        "木曜日",
        "金曜日",
        "土曜日",
    ][usize::from(index)]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarCandidateChange {
    Added,
    Removed,
}

/// Toggle one exact calendar-selected date and base-time pair.
pub fn toggle_calendar_candidate(
    candidates: &mut Vec<CandidateInput>,
    local_date: &str,
    local_time: &str,
) -> Result<CalendarCandidateChange, String> {
    let local_date = local_date.trim();
    let local_time = local_time.trim();
    if CalendarMonth::from_iso_date(local_date).is_none() {
        return Err("日付を正しく入力してください。".to_owned());
    }
    if !valid_calendar_time(local_time) {
        return Err("時刻は24時間表記のHH:MMで入力してください。".to_owned());
    }
    if let Some(position) = candidates.iter().position(|candidate| {
        candidate.local_date.trim() == local_date && candidate.local_time.trim() == local_time
    }) {
        candidates.remove(position);
        return Ok(CalendarCandidateChange::Removed);
    }
    if candidates.len() >= EVENT_CANDIDATE_MAX_COUNT {
        return Err(format!(
            "候補日時は{EVENT_CANDIDATE_MAX_COUNT}件以内で入力してください。"
        ));
    }
    candidates.push(CandidateInput {
        local_date: local_date.to_owned(),
        local_time: local_time.to_owned(),
    });
    candidates.sort_by(|left, right| {
        (&left.local_date, &left.local_time).cmp(&(&right.local_date, &right.local_time))
    });
    Ok(CalendarCandidateChange::Added)
}

fn valid_calendar_time(value: &str) -> bool {
    let Some((hour, minute)) = value.split_once(':') else {
        return false;
    };
    hour.len() == 2
        && minute.len() == 2
        && hour.parse::<u8>().is_ok_and(|hour| hour < 24)
        && minute.parse::<u8>().is_ok_and(|minute| minute < 60)
}

#[derive(Clone)]
pub struct CalendarDateToggleCallback(Rc<RefCell<dyn FnMut(String)>>);

impl CalendarDateToggleCallback {
    fn call(&self, local_date: String) {
        (self.0.borrow_mut())(local_date);
    }
}

impl<F> From<F> for CalendarDateToggleCallback
where
    F: FnMut(String) + 'static,
{
    fn from(callback: F) -> Self {
        Self(Rc::new(RefCell::new(callback)))
    }
}

impl PartialEq for CalendarDateToggleCallback {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// One always-visible month of native date toggle buttons.
#[component]
pub fn CandidateCalendar(
    month: CalendarMonth,
    candidate_time: String,
    candidates: Vec<CandidateInput>,
    #[props(into)] on_toggle: CalendarDateToggleCallback,
) -> Element {
    let mut visible_month = use_signal(|| month);
    let current_month = visible_month();
    let previous_month = current_month.previous();
    let next_month = current_month.next();
    let month_label = current_month.label();
    let leading_blank_days = current_month.leading_blank_days();
    let weekday_labels = [
        ("日", "日曜日"),
        ("月", "月曜日"),
        ("火", "火曜日"),
        ("水", "水曜日"),
        ("木", "木曜日"),
        ("金", "金曜日"),
        ("土", "土曜日"),
    ];

    rsx! {
        section { class: "candidate-calendar", aria_label: "候補日カレンダー",
            div { class: "candidate-calendar-toolbar",
                button {
                    class: "candidate-calendar-nav",
                    r#type: "button",
                    aria_label: "前の月を表示",
                    disabled: previous_month.is_none(),
                    onclick: move |_| {
                        if let Some(previous) = visible_month().previous() {
                            visible_month.set(previous);
                        }
                    },
                    span { aria_hidden: "true", "‹" }
                }
                h3 { class: "candidate-calendar-month", aria_live: "polite", "{month_label}" }
                button {
                    class: "candidate-calendar-nav",
                    r#type: "button",
                    aria_label: "次の月を表示",
                    disabled: next_month.is_none(),
                    onclick: move |_| {
                        if let Some(next) = visible_month().next() {
                            visible_month.set(next);
                        }
                    },
                    span { aria_hidden: "true", "›" }
                }
            }
            div { class: "candidate-calendar-grid",
                for (short, full) in weekday_labels {
                    span { class: "candidate-calendar-weekday", aria_label: full, "{short}" }
                }
                for blank in 0..leading_blank_days {
                    span {
                        key: "blank-{current_month.year}-{current_month.month}-{blank}",
                        class: "candidate-calendar-blank",
                        aria_hidden: "true",
                    }
                }
                for day in 1..=current_month.days_in_month() {
                    {
                        let local_date = current_month
                            .iso_date(day)
                            .expect("calendar day belongs to its month");
                        let selected = candidates.iter().any(|candidate| {
                            candidate.local_date.trim() == local_date
                                && candidate.local_time.trim() == candidate_time.trim()
                        });
                        let weekday = weekday_name(
                            (leading_blank_days + day - 1) % 7,
                        );
                        let action = if selected { "候補から削除" } else { "候補に追加" };
                        let accessible_label = format!(
                            "{}年{}月{}日 {weekday} {}の{action}",
                            current_month.year,
                            current_month.month,
                            day,
                            candidate_time.trim(),
                        );
                        let on_toggle = on_toggle.clone();
                        rsx! {
                            button {
                                key: "{local_date}",
                                class: if selected {
                                    "candidate-calendar-day is-selected"
                                } else {
                                    "candidate-calendar-day"
                                },
                                r#type: "button",
                                aria_pressed: selected,
                                aria_label: "{accessible_label}",
                                onclick: move |_| on_toggle.call(local_date.clone()),
                                span { aria_hidden: "true", "{day}" }
                                if selected {
                                    span {
                                        class: "candidate-calendar-selected-mark",
                                        aria_hidden: "true",
                                        "✓"
                                    }
                                    span { class: "visually-hidden", "選択済み" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Shared candidate editor for ordinary and explicit-continuation creation.
#[component]
pub(crate) fn CandidateDateTimePicker(
    mut candidates: Signal<Vec<CandidateInput>>,
    mut candidate_date: Signal<String>,
    mut candidate_time: Signal<String>,
    mut errors: Signal<EventCreationErrors>,
    id_prefix: String,
) -> Element {
    let mut initial_month = use_signal(|| None::<CalendarMonth>);
    use_effect(move || {
        if let Some(month) =
            crate::browser::local_date().and_then(|value| CalendarMonth::from_iso_date(&value))
        {
            initial_month.set(Some(month));
        }
    });

    let date_id = format!("{id_prefix}-date");
    let time_id = format!("{id_prefix}-time");
    let help_id = format!("{id_prefix}-help");
    let error_id = format!("{id_prefix}-error");
    let current_error = errors().candidates;
    let current_candidates = candidates();

    rsx! {
        fieldset {
            class: "candidate-fieldset",
            aria_invalid: current_error.is_some(),
            aria_describedby: if current_error.is_some() { "{error_id}" } else { "{help_id}" },
            legend { "候補日時" }
            p { id: "{help_id}", class: "field-help",
                "カレンダーの日を押すと、下の時刻で候補に追加します。もう一度押すと解除できます。"
            }

            div { class: "candidate-base-time field-group compact",
                label { r#for: "{time_id}", "候補の時刻" }
                input {
                    id: "{time_id}",
                    name: "{time_id}",
                    r#type: "text",
                    inputmode: "numeric",
                    maxlength: 5,
                    value: "{candidate_time}",
                    placeholder: DEFAULT_CANDIDATE_TIME,
                    aria_invalid: current_error.is_some(),
                    aria_describedby: "{help_id}",
                    oninput: move |event| {
                        candidate_time.set(event.value());
                        errors.write().candidates = None;
                    },
                }
            }

            if let Some(month) = initial_month() {
                CandidateCalendar {
                    month,
                    candidate_time: candidate_time(),
                    candidates: current_candidates.clone(),
                    on_toggle: move |local_date: String| {
                        let result = {
                            let mut current = candidates.write();
                            toggle_calendar_candidate(
                                &mut current,
                                &local_date,
                                &candidate_time(),
                            )
                        };
                        match result {
                            Ok(_) => errors.write().candidates = None,
                            Err(message) => errors.write().candidates = Some(message),
                        }
                    },
                }
            } else {
                p { class: "candidate-calendar-loading", role: "status",
                    "カレンダーを準備しています…"
                }
            }

            details { class: "candidate-direct-entry",
                summary { "日付を直接入力する" }
                div { class: "candidate-editor",
                    div { class: "field-group compact",
                        label { r#for: "{date_id}", "日付" }
                        input {
                            id: "{date_id}",
                            name: "{date_id}",
                            r#type: "date",
                            value: "{candidate_date}",
                            aria_invalid: current_error.is_some(),
                            oninput: move |event| {
                                candidate_date.set(event.value());
                                errors.write().candidates = None;
                            },
                        }
                    }
                    button {
                        class: "secondary-button add-candidate",
                        r#type: "button",
                        onclick: move |_| {
                            let draft = EventCreationDraft {
                                name: "候補追加".to_owned(),
                                organizer_note: String::new(),
                                time_zone: "Etc/UTC".to_owned(),
                                candidates: candidates(),
                                pending_candidate: CandidateInput {
                                    local_date: candidate_date(),
                                    local_time: candidate_time(),
                                },
                            };
                            match draft.prepare() {
                                Ok(mut input) => {
                                    input.candidates.sort_by(|left, right| {
                                        (&left.local_date, &left.local_time)
                                            .cmp(&(&right.local_date, &right.local_time))
                                    });
                                    candidates.set(input.candidates);
                                    candidate_date.set(String::new());
                                    errors.write().candidates = None;
                                }
                                Err(next_errors) => {
                                    errors.write().candidates = next_errors.candidates;
                                }
                            }
                        },
                        "候補に追加"
                    }
                }
            }

            if let Some(message) = current_error.as_deref() {
                p { id: "{error_id}", class: "field-error", role: "alert", "{message}" }
            }

            if !current_candidates.is_empty() {
                ol { class: "candidate-list", aria_label: "追加済みの候補日時",
                    for (index, candidate) in current_candidates.iter().enumerate() {
                        li { key: "{candidate.local_date}-{candidate.local_time}-{index}",
                            time { datetime: "{candidate.local_date}T{candidate.local_time}",
                                "{format_candidate_input(candidate)}"
                            }
                            button {
                                class: "remove-candidate",
                                r#type: "button",
                                aria_label: format!("{}の候補を削除", format_candidate_input(candidate)),
                                onclick: move |_| {
                                    candidates.write().remove(index);
                                    errors.write().candidates = None;
                                },
                                "削除"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A static two-dimensional table shared after its caller has authorized a read.
pub fn candidate_suggestion_score(availabilities: &[Availability]) -> u32 {
    availabilities
        .iter()
        .map(|availability| match availability {
            Availability::Available => 2,
            Availability::Maybe => 1,
            Availability::Unavailable => 0,
        })
        .sum()
}

#[component]
pub fn OrganizerResponseMatrixView(
    matrix: OrganizerResponseMatrix,
    show_suggestions: bool,
) -> Element {
    if matrix.responses.is_empty() {
        return rsx! {
            section {
                class: "response-matrix-state response-matrix-empty",
                role: "status",
                aria_labelledby: "response-matrix-empty-heading",
                h2 { id: "response-matrix-empty-heading", "回答者ごとの候補日時への回答" }
                p { "まだ詳細回答はありません" }
            }
        };
    }

    let scores = matrix
        .candidates
        .iter()
        .enumerate()
        .map(|(index, _)| {
            candidate_suggestion_score(
                &matrix
                    .responses
                    .iter()
                    .map(|row| row.availabilities[index])
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let maximum_score = scores.iter().copied().max().unwrap_or(0);
    rsx! {
        div { class: "response-matrix-view",
            p { id: "response-matrix-scroll-help", class: "response-matrix-scroll-help",
                "表にフォーカスして、左右にスクロールするとすべての候補日時を確認できます。"
            }
            div {
                class: "response-matrix-scroll",
                role: "region",
                tabindex: "0",
                aria_labelledby: "response-matrix-caption",
                aria_describedby: "response-matrix-scroll-help",
                table { class: "response-matrix-table",
                    caption { id: "response-matrix-caption",
                        span { class: "response-matrix-caption-title",
                            "回答者ごとの候補日時への回答"
                        }
                        span { class: "response-matrix-caption-context",
                            "{matrix.name} — {matrix.time_zone} の時刻"
                        }
                    }
                    thead {
                        tr {
                            th {
                                class: "response-matrix-respondent-heading",
                                scope: "col",
                                "回答者"
                            }
                            for (candidate_index, candidate) in matrix.candidates.iter().enumerate() {
                                {
                                    let candidate_text = format_local_start(
                                        &candidate.local_date,
                                        &candidate.local_time,
                                    );
                                    let score = scores[candidate_index];
                                    rsx! {
                                        th { scope: "col", class: if show_suggestions && score == maximum_score && maximum_score > 0 { "response-matrix-suggested" } else { "" },
                                            time {
                                                datetime: "{candidate.local_date}T{candidate.local_time}",
                                                "{candidate_text}"
                                            }
                                            if show_suggestions && score == maximum_score && maximum_score > 0 { span { class: "response-matrix-suggestion", "おすすめ" } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    tbody {
                        for (row_index, response) in matrix.responses.iter().enumerate() {
                            tr { key: "response-matrix-row-{row_index}",
                                th { scope: "row", "{response.respondent_name}" }
                                for availability in response.availabilities.iter().copied() {
                                    td { class: "response-matrix-answer",
                                        span {
                                            class: "response-matrix-symbol",
                                            aria_hidden: "true",
                                            "{availability.symbol()}"
                                        }
                                        span { class: "response-matrix-meaning",
                                            "{availability.short_label()}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn format_local_start(local_date: &str, local_time: &str) -> String {
    let mut parts = local_date.split('-');
    let year = parts.next().unwrap_or(local_date);
    let month = parts
        .next()
        .and_then(|value| value.parse::<u8>().ok())
        .map_or_else(|| "?".to_owned(), |value| value.to_string());
    let day = parts
        .next()
        .and_then(|value| value.parse::<u8>().ok())
        .map_or_else(|| "?".to_owned(), |value| value.to_string());
    format!("{year}年{month}月{day}日 {local_time}")
}

fn format_candidate_input(candidate: &CandidateInput) -> String {
    format_local_start(&candidate.local_date, &candidate.local_time)
}

#[cfg(test)]
mod suggestion_tests {
    use super::candidate_suggestion_score;
    use crate::domain::Availability;

    #[test]
    fn available_is_worth_two_and_maybe_one() {
        assert_eq!(
            candidate_suggestion_score(&[Availability::Available, Availability::Maybe]),
            3
        );
    }
}
