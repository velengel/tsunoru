//! Domain values and validation shared by the browser and server.

use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

pub const EVENT_NAME_MAX_CHARS: usize = 100;
pub const ORGANIZER_NOTE_MAX_CHARS: usize = 500;
pub const EVENT_CANDIDATE_MAX_COUNT: usize = 20;
pub const RESPONDENT_NAME_MAX_CHARS: usize = 100;
pub const RESPONDENT_COMMENT_MAX_CHARS: usize = 500;
pub const RESPONSE_CAPABILITY_HEX_LENGTH: usize = 64;
pub const ORGANIZER_CAPABILITY_HEX_LENGTH: usize = 64;
pub const LOGIN_ID_MIN_CHARS: usize = 3;
pub const LOGIN_ID_MAX_CHARS: usize = 32;
pub const PASSWORD_MIN_CHARS: usize = 15;
pub const PASSWORD_MAX_CHARS: usize = 128;
pub const PASSWORD_MAX_OCTETS: usize = 512;

/// Untrusted account-registration fields crossing the browser/server boundary.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRegistrationInput {
    pub login_id: String,
    pub password: String,
    pub password_confirmation: String,
}

impl AccountRegistrationInput {
    /// Normalize the public identifier while preserving password bytes exactly.
    pub fn prepare(&self) -> Result<PreparedAccountRegistration, AccountAuthErrors> {
        let login_id = normalize_login_id(&self.login_id);
        let mut errors = AccountAuthErrors::default();
        validate_login_id(&login_id, &mut errors);
        validate_password(&self.password, &mut errors);
        if self.password != self.password_confirmation {
            errors.password_confirmation = Some("password確認が一致しません。".to_owned());
        }

        if errors.is_empty() {
            Ok(PreparedAccountRegistration {
                login_id,
                password: self.password.clone(),
            })
        } else {
            Err(errors)
        }
    }
}

impl fmt::Debug for AccountRegistrationInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AccountRegistrationInput")
            .field("login_id", &self.login_id)
            .field("password", &"[REDACTED]")
            .field("password_confirmation", &"[REDACTED]")
            .finish()
    }
}

/// Validated registration data used only while hashing and storing one account.
#[derive(Clone, PartialEq, Eq)]
pub struct PreparedAccountRegistration {
    pub login_id: String,
    pub password: String,
}

impl fmt::Debug for PreparedAccountRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedAccountRegistration")
            .field("login_id", &self.login_id)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

/// Untrusted login fields crossing the browser/server boundary.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountLoginInput {
    pub login_id: String,
    pub password: String,
}

impl AccountLoginInput {
    /// Normalize and bound credentials before password verification.
    pub fn prepare(&self) -> Result<PreparedAccountLogin, AccountAuthErrors> {
        let login_id = normalize_login_id(&self.login_id);
        let mut errors = AccountAuthErrors::default();
        validate_login_id(&login_id, &mut errors);
        validate_password(&self.password, &mut errors);

        if errors.is_empty() {
            Ok(PreparedAccountLogin {
                login_id,
                password: self.password.clone(),
            })
        } else {
            Err(errors)
        }
    }
}

impl fmt::Debug for AccountLoginInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AccountLoginInput")
            .field("login_id", &self.login_id)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

/// Validated login data used only during one password verification.
#[derive(Clone, PartialEq, Eq)]
pub struct PreparedAccountLogin {
    pub login_id: String,
    pub password: String,
}

impl fmt::Debug for PreparedAccountLogin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedAccountLogin")
            .field("login_id", &self.login_id)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

/// Field and request errors shared by registration and login forms.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccountAuthErrors {
    pub login_id: Option<String>,
    pub password: Option<String>,
    pub password_confirmation: Option<String>,
    pub request: Option<String>,
}

impl AccountAuthErrors {
    pub fn is_empty(&self) -> bool {
        self.login_id.is_none()
            && self.password.is_none()
            && self.password_confirmation.is_none()
            && self.request.is_none()
    }
}

impl fmt::Display for AccountAuthErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages = [
            self.login_id.as_deref(),
            self.password.as_deref(),
            self.password_confirmation.as_deref(),
            self.request.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        write!(formatter, "{}", messages.join(" "))
    }
}

fn normalize_login_id(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn validate_login_id(login_id: &str, errors: &mut AccountAuthErrors) {
    let valid_length = (LOGIN_ID_MIN_CHARS..=LOGIN_ID_MAX_CHARS).contains(&login_id.len());
    let mut bytes = login_id.bytes();
    let valid_first = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric());
    let valid_rest =
        bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if !valid_length || !valid_first || !valid_rest {
        errors.login_id = Some(format!(
            "ログインIDは{LOGIN_ID_MIN_CHARS}〜{LOGIN_ID_MAX_CHARS}文字の半角英数字から始め、半角英数字と . _ - で入力してください。"
        ));
    }
}

fn validate_password(password: &str, errors: &mut AccountAuthErrors) {
    let char_count = password.chars().count();
    if !(PASSWORD_MIN_CHARS..=PASSWORD_MAX_CHARS).contains(&char_count)
        || password.len() > PASSWORD_MAX_OCTETS
    {
        errors.password = Some(format!(
            "passwordは{PASSWORD_MIN_CHARS}〜{PASSWORD_MAX_CHARS}文字で入力してください。"
        ));
    }
}

/// Public account identity returned to the authenticated browser.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentAccount {
    pub login_id: String,
}

/// A decided start projected into a compact history item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryDecision {
    pub local_date: String,
    pub local_time: String,
}

/// One event linked when the current account created it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizedEventHistoryItem {
    pub public_id: String,
    pub name: String,
    pub time_zone: String,
    pub decision: Option<HistoryDecision>,
    pub response_count: u64,
}

/// One explicitly linked recurring activity and its organizer-owned events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizedEventSeriesHistory {
    pub series_name: String,
    pub events: Vec<OrganizedEventHistoryItem>,
}

/// One event linked when the current account answered it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipatedEventHistoryItem {
    pub public_id: String,
    pub name: String,
    pub time_zone: String,
    pub decision: Option<HistoryDecision>,
}

/// Minimal private history projection for one authenticated account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountHistory {
    pub login_id: String,
    pub organized_standalone: Vec<OrganizedEventHistoryItem>,
    pub organized_series: Vec<OrganizedEventSeriesHistory>,
    pub participated: Vec<ParticipatedEventHistoryItem>,
}

/// Authenticated-history loading result without exposing session material.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountHistoryState {
    Guest,
    Expired,
    Authenticated(AccountHistory),
}

/// Untrusted request for an organizer-owned event continuation plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventContinuationPlanInput {
    pub origin_event_public_id: String,
}

impl EventContinuationPlanInput {
    /// Normalize the public origin without accepting account or series identity.
    pub fn normalized_and_validated(&self) -> Result<Self, EventContinuationErrors> {
        let origin_event_public_id = self.origin_event_public_id.trim().to_owned();
        if valid_public_id(&origin_event_public_id) {
            Ok(Self {
                origin_event_public_id,
            })
        } else {
            Err(EventContinuationErrors {
                request: Some("続きのイベントを確認できませんでした。".to_owned()),
                event: EventCreationErrors::default(),
            })
        }
    }
}

/// Untrusted request to create one event in an explicitly selected series.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventContinuationCreateInput {
    pub origin_event_public_id: String,
    pub expected_tail_event_public_id: String,
    pub event: NewEventInput,
}

impl EventContinuationCreateInput {
    /// Revalidate both public identifiers and the complete nested event draft.
    pub fn normalized_and_validated(&self) -> Result<Self, EventContinuationErrors> {
        let origin_event_public_id = self.origin_event_public_id.trim().to_owned();
        let expected_tail_event_public_id = self.expected_tail_event_public_id.trim().to_owned();
        let event = self.event.normalized_and_validated();
        let mut errors = EventContinuationErrors::default();
        if !valid_public_id(&origin_event_public_id)
            || !valid_public_id(&expected_tail_event_public_id)
        {
            errors.request = Some("続きのイベントを確認できませんでした。".to_owned());
        }
        match event {
            Ok(event) if errors.is_empty() => Ok(Self {
                origin_event_public_id,
                expected_tail_event_public_id,
                event,
            }),
            Ok(_) => Err(errors),
            Err(event) => {
                errors.event = event;
                Err(errors)
            }
        }
    }
}

/// Validation errors for a private continuation request and its event form.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventContinuationErrors {
    pub request: Option<String>,
    pub event: EventCreationErrors,
}

impl EventContinuationErrors {
    pub fn is_empty(&self) -> bool {
        self.request.is_none() && self.event.is_empty()
    }
}

impl fmt::Display for EventContinuationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let event_message = (!self.event.is_empty()).then(|| self.event.to_string());
        let messages = [self.request.as_deref(), event_message.as_deref()]
            .into_iter()
            .flatten()
            .filter(|message| !message.is_empty())
            .collect::<Vec<_>>();
        write!(formatter, "{}", messages.join(" "))
    }
}

/// Private organizer projection used to prefill, but never submit, a next name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventContinuationPlan {
    pub origin_event_public_id: String,
    pub origin_event_name: String,
    pub series_name: String,
    pub tail_event_public_id: String,
    pub suggested_event_name: Option<String>,
}

/// Private continuation-plan result without account or session material.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountEventContinuationState {
    Guest,
    Expired,
    Authenticated(EventContinuationPlan),
}

/// Suggest only an exact, positive, non-zero-padded trailing ASCII ` #N` successor.
pub fn suggest_next_event_name(event_name: &str) -> Option<String> {
    let (base, number) = strict_series_suffix(event_name.trim())?;
    let next = number.checked_add(1)?;
    let suggestion = format!("{base} #{next}");
    (suggestion.chars().count() <= EVENT_NAME_MAX_CHARS).then_some(suggestion)
}

/// Derive a display label without making the event name the series identity.
pub fn derive_event_series_name(event_name: &str) -> String {
    let event_name = event_name.trim();
    strict_series_suffix(event_name)
        .map(|(base, _)| base.to_owned())
        .unwrap_or_else(|| event_name.to_owned())
}

fn strict_series_suffix(event_name: &str) -> Option<(&str, u64)> {
    let (base, digits) = event_name.rsplit_once(" #")?;
    if base.is_empty()
        || base.chars().last().is_some_and(char::is_whitespace)
        || digits.is_empty()
        || digits.starts_with('0')
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let number = digits.parse::<u64>().ok()?;
    (number > 0).then_some((base, number))
}

/// Untrusted request for one account-private event trace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountEventTraceInput {
    pub event_public_id: String,
}

impl AccountEventTraceInput {
    /// Trim and validate the public identifier without accepting account authority.
    pub fn normalized_and_validated(&self) -> Result<Self, AccountEventTraceErrors> {
        let event_public_id = self.event_public_id.trim().to_owned();
        if valid_public_id(&event_public_id) {
            Ok(Self { event_public_id })
        } else {
            Err(AccountEventTraceErrors {
                request: Some("イベントを確認できませんでした。".to_owned()),
            })
        }
    }
}

/// Validation errors that do not reveal whether an event or account link exists.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccountEventTraceErrors {
    pub request: Option<String>,
}

impl AccountEventTraceErrors {
    pub fn is_empty(&self) -> bool {
        self.request.is_none()
    }
}

impl fmt::Display for AccountEventTraceErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.request.as_deref().unwrap_or_default())
    }
}

/// The current account's read relationship to one event, decided only by the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountEventTraceRelationship {
    Organized,
    Participated,
    OrganizedAndParticipated,
}

/// One candidate date in authored order without an internal database identifier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountEventTraceCandidate {
    pub local_date: String,
    pub local_time: String,
}

/// One response visible within the current account's role-scoped trace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountEventTraceResponse {
    pub respondent_name: String,
    pub comment: Option<String>,
    pub availabilities: Vec<Availability>,
    pub is_current_account: bool,
}

/// Existing event facts projected only after account authentication and authorization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountEventTrace {
    pub public_id: String,
    pub name: String,
    pub organizer_note: Option<String>,
    pub time_zone: String,
    pub relationship: AccountEventTraceRelationship,
    pub candidates: Vec<AccountEventTraceCandidate>,
    pub decision: Option<HistoryDecision>,
    pub responses: Vec<AccountEventTraceResponse>,
}

/// Account-private trace loading result without session material.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountEventTraceState {
    Guest,
    Expired,
    Authenticated(AccountEventTrace),
}

/// A candidate start represented in the event's local time zone.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateInput {
    pub local_date: String,
    pub local_time: String,
}

impl CandidateInput {
    fn is_complete(&self) -> bool {
        !self.local_date.trim().is_empty() && !self.local_time.trim().is_empty()
    }

    fn normalized(&self) -> Self {
        Self {
            local_date: self.local_date.trim().to_owned(),
            local_time: self.local_time.trim().to_owned(),
        }
    }

    fn has_valid_shape(&self) -> bool {
        valid_local_date(&self.local_date) && valid_local_time(&self.local_time)
    }
}

/// Browser-side draft, including a candidate that has not yet been added.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventCreationDraft {
    pub name: String,
    pub organizer_note: String,
    pub time_zone: String,
    pub candidates: Vec<CandidateInput>,
    pub pending_candidate: CandidateInput,
}

impl EventCreationDraft {
    /// Normalize and validate the complete draft before calling the server.
    pub fn prepare(&self) -> Result<NewEventInput, EventCreationErrors> {
        let mut candidates = self.candidates.clone();
        let pending = self.pending_candidate.normalized();
        let mut errors = EventCreationErrors::default();

        // The shared candidate picker always has a base time, even when its
        // direct-entry date is unused. Only a date starts a pending candidate.
        if !pending.local_date.is_empty() {
            if !pending.is_complete() {
                errors.candidates = Some("日付と開始時刻を両方入力してください。".to_owned());
            } else if candidates.iter().any(|candidate| {
                candidate.local_date.trim() == pending.local_date
                    && candidate.local_time.trim() == pending.local_time
            }) {
                errors.candidates = Some("同じ候補日時がすでに追加されています。".to_owned());
            } else {
                candidates.push(pending);
            }
        }

        let input = NewEventInput {
            name: self.name.clone(),
            organizer_note: if self.organizer_note.trim().is_empty() {
                None
            } else {
                Some(self.organizer_note.clone())
            },
            time_zone: self.time_zone.clone(),
            candidates,
        };

        match input.normalized_and_validated() {
            Ok(input) if errors.is_empty() => Ok(input),
            Ok(_) => Err(errors),
            Err(validation_errors) => {
                errors.merge(validation_errors);
                Err(errors)
            }
        }
    }
}

/// Input accepted by the event-creation server function.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewEventInput {
    pub name: String,
    pub organizer_note: Option<String>,
    pub time_zone: String,
    pub candidates: Vec<CandidateInput>,
}

impl NewEventInput {
    /// Revalidate untrusted server-function input and return its normalized form.
    pub fn normalized_and_validated(&self) -> Result<Self, EventCreationErrors> {
        let name = self.name.trim().to_owned();
        let organizer_note = self
            .organizer_note
            .as_deref()
            .map(str::trim)
            .filter(|note| !note.is_empty())
            .map(ToOwned::to_owned);
        let time_zone = self.time_zone.trim().to_owned();
        let candidates = self
            .candidates
            .iter()
            .map(CandidateInput::normalized)
            .collect::<Vec<_>>();

        let mut errors = EventCreationErrors::default();
        if name.is_empty() {
            errors.name = Some("イベント名を入力してください。".to_owned());
        } else if name.chars().count() > EVENT_NAME_MAX_CHARS {
            errors.name = Some(format!(
                "イベント名は{EVENT_NAME_MAX_CHARS}文字以内で入力してください。"
            ));
        }

        if organizer_note
            .as_deref()
            .is_some_and(|note| note.chars().count() > ORGANIZER_NOTE_MAX_CHARS)
        {
            errors.organizer_note = Some(format!(
                "主催者のひとことは{ORGANIZER_NOTE_MAX_CHARS}文字以内で入力してください。"
            ));
        }

        if time_zone.is_empty() || !valid_time_zone_name(&time_zone) {
            errors.time_zone = Some(
                "ブラウザーのタイムゾーンを取得できませんでした。再読み込みしてください。"
                    .to_owned(),
            );
        }

        if candidates.is_empty() {
            errors.candidates = Some("候補日時を一件以上追加してください。".to_owned());
        } else if candidates.len() > EVENT_CANDIDATE_MAX_COUNT {
            errors.candidates = Some(format!(
                "候補日時は{EVENT_CANDIDATE_MAX_COUNT}件以内で入力してください。"
            ));
        } else if candidates
            .iter()
            .any(|candidate| !candidate.is_complete() || !candidate.has_valid_shape())
        {
            errors.candidates = Some("候補日時を正しい日付と時刻で入力してください。".to_owned());
        } else {
            let unique_count = candidates
                .iter()
                .map(|candidate| (&candidate.local_date, &candidate.local_time))
                .collect::<HashSet<_>>()
                .len();
            if unique_count != candidates.len() {
                errors.candidates = Some("同じ候補日時がすでに追加されています。".to_owned());
            }
        }

        if errors.is_empty() {
            Ok(Self {
                name,
                organizer_note,
                time_zone,
                candidates,
            })
        } else {
            Err(errors)
        }
    }
}

/// Field-level errors used by both domain tests and the creation form.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventCreationErrors {
    pub name: Option<String>,
    pub organizer_note: Option<String>,
    pub candidates: Option<String>,
    pub time_zone: Option<String>,
}

impl EventCreationErrors {
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.organizer_note.is_none()
            && self.candidates.is_none()
            && self.time_zone.is_none()
    }

    fn merge(&mut self, other: Self) {
        if self.name.is_none() && other.name.is_some() {
            self.name = other.name;
        }
        if self.organizer_note.is_none() && other.organizer_note.is_some() {
            self.organizer_note = other.organizer_note;
        }
        if self.candidates.is_none() && other.candidates.is_some() {
            self.candidates = other.candidates;
        }
        if self.time_zone.is_none() && other.time_zone.is_some() {
            self.time_zone = other.time_zone;
        }
    }
}

impl fmt::Display for EventCreationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages = [
            self.name.as_deref(),
            self.organizer_note.as_deref(),
            self.candidates.as_deref(),
            self.time_zone.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        write!(formatter, "{}", messages.join(" "))
    }
}

/// A persisted candidate exposed through the public event route.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicCandidate {
    pub id: i64,
    pub local_date: String,
    pub local_time: String,
}

/// The organizer-selected candidate fields safe for every holder of the shared URL.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicEventDecision {
    pub candidate_id: i64,
    pub local_date: String,
    pub local_time: String,
}

/// Public-by-link event data. It deliberately excludes organizer authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicEvent {
    pub public_id: String,
    pub name: String,
    pub organizer_note: Option<String>,
    pub time_zone: String,
    pub candidates: Vec<PublicCandidate>,
    pub decision: Option<PublicEventDecision>,
}

/// One-time creation result returned to the organizer's browser.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatedEvent {
    pub event: PublicEvent,
    pub organizer_capability: String,
}

impl fmt::Debug for CreatedEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreatedEvent")
            .field("event", &self.event)
            .field("organizer_capability", &"[REDACTED]")
            .finish()
    }
}

/// One answer to one candidate date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    Maybe,
    Unavailable,
}

impl Availability {
    pub fn storage_value(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Maybe => "maybe",
            Self::Unavailable => "unavailable",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Available => "○",
            Self::Maybe => "△",
            Self::Unavailable => "×",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Available => "行ける",
            Self::Maybe => "条件次第",
            Self::Unavailable => "難しい",
        }
    }

    pub fn accessible_label(self) -> &'static str {
        match self {
            Self::Available => "○ 行ける",
            Self::Maybe => "△ 条件次第・たぶん行ける",
            Self::Unavailable => "× 難しい",
        }
    }
}

impl TryFrom<&str> for Availability {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "available" => Ok(Self::Available),
            "maybe" => Ok(Self::Maybe),
            "unavailable" => Ok(Self::Unavailable),
            _ => Err(format!("unknown availability value: {value}")),
        }
    }
}

/// A typed candidate choice crossing the browser/server boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateAvailabilityInput {
    pub candidate_id: i64,
    pub availability: Availability,
}

/// Browser-side response state before field validation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AvailabilityResponseDraft {
    pub respondent_name: String,
    pub candidate_ids: Vec<i64>,
    pub availabilities: Vec<CandidateAvailabilityInput>,
}

impl AvailabilityResponseDraft {
    /// Validate visible fields and arrange choices in the event's authored order.
    pub fn prepare(&self) -> Result<PreparedAvailabilityResponse, AvailabilityResponseErrors> {
        let respondent_name = self.respondent_name.trim().to_owned();
        let mut errors = AvailabilityResponseErrors::default();

        validate_respondent_name(&respondent_name, &mut errors);

        if self.candidate_ids.is_empty()
            || self.candidate_ids.len() > EVENT_CANDIDATE_MAX_COUNT
            || self
                .candidate_ids
                .iter()
                .any(|candidate_id| *candidate_id <= 0)
            || self.candidate_ids.iter().collect::<HashSet<_>>().len() != self.candidate_ids.len()
        {
            errors.request = Some("候補日時を正しく読み込めませんでした。".to_owned());
        }

        let expected = self.candidate_ids.iter().copied().collect::<HashSet<_>>();
        let mut selected = HashMap::with_capacity(self.availabilities.len());
        for choice in &self.availabilities {
            if choice.candidate_id <= 0
                || !expected.contains(&choice.candidate_id)
                || selected
                    .insert(choice.candidate_id, choice.availability)
                    .is_some()
            {
                errors.request = Some("候補日時への回答を確認してください。".to_owned());
            }
        }

        errors.candidate_ids = self
            .candidate_ids
            .iter()
            .copied()
            .filter(|candidate_id| !selected.contains_key(candidate_id))
            .collect();

        if errors.is_empty() {
            Ok(PreparedAvailabilityResponse {
                respondent_name,
                availabilities: self
                    .candidate_ids
                    .iter()
                    .map(|candidate_id| CandidateAvailabilityInput {
                        candidate_id: *candidate_id,
                        availability: selected[candidate_id],
                    })
                    .collect(),
            })
        } else {
            Err(errors)
        }
    }
}

/// A normalized name and complete candidate-choice aggregate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedAvailabilityResponse {
    pub respondent_name: String,
    pub availabilities: Vec<CandidateAvailabilityInput>,
}

/// Untrusted input accepted by the anonymous response server function.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewAvailabilityResponseInput {
    pub event_public_id: String,
    pub response_capability: String,
    pub response: PreparedAvailabilityResponse,
}

impl NewAvailabilityResponseInput {
    /// Revalidate and normalize a typed request at the server boundary.
    pub fn normalized_and_validated(&self) -> Result<Self, AvailabilityResponseErrors> {
        let event_public_id = self.event_public_id.trim().to_owned();
        let response_capability = self.response_capability.trim().to_owned();
        let respondent_name = self.response.respondent_name.trim().to_owned();
        let mut errors = AvailabilityResponseErrors::default();

        validate_respondent_name(&respondent_name, &mut errors);

        if !valid_public_id(&event_public_id) {
            errors.request = Some("イベントを確認できませんでした。".to_owned());
        }
        if response_capability.len() != RESPONSE_CAPABILITY_HEX_LENGTH
            || !response_capability
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            errors.request = Some("回答の送信準備に失敗しました。".to_owned());
        }

        if self.response.availabilities.is_empty()
            || self.response.availabilities.len() > EVENT_CANDIDATE_MAX_COUNT
            || self
                .response
                .availabilities
                .iter()
                .any(|choice| choice.candidate_id <= 0)
        {
            errors.request = Some("候補日時への回答を確認してください。".to_owned());
        }

        let unique_candidate_count = self
            .response
            .availabilities
            .iter()
            .map(|choice| choice.candidate_id)
            .collect::<HashSet<_>>()
            .len();
        if unique_candidate_count != self.response.availabilities.len() {
            errors.request = Some("候補日時への回答を確認してください。".to_owned());
        }

        if errors.is_empty() {
            let mut availabilities = self.response.availabilities.clone();
            availabilities.sort_unstable_by_key(|choice| choice.candidate_id);
            Ok(Self {
                event_public_id,
                response_capability,
                response: PreparedAvailabilityResponse {
                    respondent_name,
                    availabilities,
                },
            })
        } else {
            Err(errors)
        }
    }
}

/// Field and request errors shared by validation tests and the response form.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AvailabilityResponseErrors {
    pub respondent_name: Option<String>,
    pub candidate_ids: Vec<i64>,
    pub request: Option<String>,
}

impl AvailabilityResponseErrors {
    pub fn is_empty(&self) -> bool {
        self.respondent_name.is_none() && self.candidate_ids.is_empty() && self.request.is_none()
    }
}

impl fmt::Display for AvailabilityResponseErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut messages = Vec::new();
        if let Some(message) = self.respondent_name.as_deref() {
            messages.push(message);
        }
        if !self.candidate_ids.is_empty() {
            messages.push("すべての候補日時へ都合を選んでください。");
        }
        if let Some(message) = self.request.as_deref() {
            messages.push(message);
        }
        write!(formatter, "{}", messages.join(" "))
    }
}

/// Browser-side optional comment before validation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResponseCommentDraft {
    pub comment: String,
}

impl ResponseCommentDraft {
    /// Normalize one intentional utterance without turning an empty draft into data.
    pub fn prepare(&self) -> Result<PreparedResponseComment, ResponseCommentErrors> {
        let comment = self.comment.trim().to_owned();
        let mut errors = ResponseCommentErrors::default();
        validate_response_comment(&comment, &mut errors);

        if errors.is_empty() {
            Ok(PreparedResponseComment { comment })
        } else {
            Err(errors)
        }
    }
}

/// A normalized optional utterance ready to cross the server boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedResponseComment {
    pub comment: String,
}

/// Untrusted input accepted by the anonymous response-comment server function.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewResponseCommentInput {
    pub event_public_id: String,
    pub response_capability: String,
    pub comment: String,
}

impl NewResponseCommentInput {
    /// Revalidate authorization identifiers and comment text at the server boundary.
    pub fn normalized_and_validated(&self) -> Result<Self, ResponseCommentErrors> {
        let event_public_id = self.event_public_id.trim().to_owned();
        let response_capability = self.response_capability.trim().to_owned();
        let comment = self.comment.trim().to_owned();
        let mut errors = ResponseCommentErrors::default();

        validate_response_comment(&comment, &mut errors);
        if !valid_public_id(&event_public_id) {
            errors.request = Some("イベントを確認できませんでした。".to_owned());
        }
        if response_capability.len() != RESPONSE_CAPABILITY_HEX_LENGTH
            || !response_capability
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            errors.request = Some("回答を確認できませんでした。".to_owned());
        }

        if errors.is_empty() {
            Ok(Self {
                event_public_id,
                response_capability,
                comment,
            })
        } else {
            Err(errors)
        }
    }
}

/// Field and request errors for one optional response comment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResponseCommentErrors {
    pub comment: Option<String>,
    pub request: Option<String>,
}

impl ResponseCommentErrors {
    pub fn is_empty(&self) -> bool {
        self.comment.is_none() && self.request.is_none()
    }
}

impl fmt::Display for ResponseCommentErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages = [self.comment.as_deref(), self.request.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        write!(formatter, "{}", messages.join(" "))
    }
}

fn validate_response_comment(comment: &str, errors: &mut ResponseCommentErrors) {
    if comment.is_empty() {
        errors.comment = Some("ひとことを入力してください。".to_owned());
    } else if comment.chars().count() > RESPONDENT_COMMENT_MAX_CHARS {
        errors.comment = Some(format!(
            "ひとことは{RESPONDENT_COMMENT_MAX_CHARS}文字以内で入力してください。"
        ));
    } else if comment.contains('\0') {
        errors.comment = Some("ひとことに使用できない文字が含まれています。".to_owned());
    }
}

fn validate_respondent_name(name: &str, errors: &mut AvailabilityResponseErrors) {
    if name.is_empty() {
        errors.respondent_name = Some("名前を入力してください。".to_owned());
    } else if name.chars().count() > RESPONDENT_NAME_MAX_CHARS {
        errors.respondent_name = Some(format!(
            "名前は{RESPONDENT_NAME_MAX_CHARS}文字以内で入力してください。"
        ));
    }
}

fn valid_public_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn valid_time_zone_name(value: &str) -> bool {
    let safe_shape = !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-' | b'+'));
    if !safe_shape {
        return false;
    }

    #[cfg(feature = "server")]
    {
        value.parse::<chrono_tz::Tz>().is_ok()
    }

    #[cfg(not(feature = "server"))]
    {
        true
    }
}

fn valid_local_time(value: &str) -> bool {
    let Some((hour, minute)) = value.split_once(':') else {
        return false;
    };
    hour.len() == 2
        && minute.len() == 2
        && hour.parse::<u8>().is_ok_and(|hour| hour < 24)
        && minute.parse::<u8>().is_ok_and(|minute| minute < 60)
}

fn valid_local_date(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return false;
    }

    let (Ok(year), Ok(month), Ok(day)) = (
        parts[0].parse::<i32>(),
        parts[1].parse::<u8>(),
        parts[2].parse::<u8>(),
    ) else {
        return false;
    };
    if year < 1 || !(1..=12).contains(&month) {
        return false;
    }

    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month).contains(&day)
}

/// Untrusted organizer authority used to request one private response summary.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizerSummaryInput {
    pub event_public_id: String,
    pub organizer_capability: String,
}

impl OrganizerSummaryInput {
    /// Normalize and validate identifiers without exposing the bearer capability.
    pub fn normalized_and_validated(&self) -> Result<Self, OrganizerSummaryErrors> {
        let event_public_id = self.event_public_id.trim().to_owned();
        let organizer_capability = self.organizer_capability.trim().to_owned();
        let mut errors = OrganizerSummaryErrors::default();

        if !valid_public_id(&event_public_id) {
            errors.request = Some("イベントを確認できませんでした。".to_owned());
        }
        if organizer_capability.len() != ORGANIZER_CAPABILITY_HEX_LENGTH
            || !organizer_capability
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            errors.request = Some("主催者用の復旧キーを確認してください。".to_owned());
        }

        if errors.is_empty() {
            Ok(Self {
                event_public_id,
                organizer_capability,
            })
        } else {
            Err(errors)
        }
    }
}

impl fmt::Debug for OrganizerSummaryInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OrganizerSummaryInput")
            .field("event_public_id", &self.event_public_id)
            .field("organizer_capability", &"[REDACTED]")
            .finish()
    }
}

/// Request validation errors for an organizer-only response summary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OrganizerSummaryErrors {
    pub request: Option<String>,
}

impl OrganizerSummaryErrors {
    pub fn is_empty(&self) -> bool {
        self.request.is_none()
    }
}

impl fmt::Display for OrganizerSummaryErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.request.as_deref().unwrap_or_default())
    }
}

/// Untrusted organizer authority and candidate selection for one event decision.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizerDecisionInput {
    pub event_public_id: String,
    pub candidate_id: i64,
    pub organizer_capability: String,
}

impl OrganizerDecisionInput {
    /// Normalize and validate identifiers without exposing the bearer capability.
    pub fn normalized_and_validated(&self) -> Result<Self, OrganizerDecisionErrors> {
        let event_public_id = self.event_public_id.trim().to_owned();
        let organizer_capability = self.organizer_capability.trim().to_owned();
        let mut errors = OrganizerDecisionErrors::default();

        if !valid_public_id(&event_public_id) {
            errors.request = Some("イベントを確認できませんでした。".to_owned());
        }
        if self.candidate_id <= 0 {
            errors.request = Some("候補日時を確認してください。".to_owned());
        }
        if organizer_capability.len() != ORGANIZER_CAPABILITY_HEX_LENGTH
            || !organizer_capability
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            errors.request = Some("主催者用の復旧キーを確認してください。".to_owned());
        }

        if errors.is_empty() {
            Ok(Self {
                event_public_id,
                candidate_id: self.candidate_id,
                organizer_capability,
            })
        } else {
            Err(errors)
        }
    }
}

impl fmt::Debug for OrganizerDecisionInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OrganizerDecisionInput")
            .field("event_public_id", &self.event_public_id)
            .field("candidate_id", &self.candidate_id)
            .field("organizer_capability", &"[REDACTED]")
            .finish()
    }
}

/// Request validation errors for an organizer-only event decision.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OrganizerDecisionErrors {
    pub request: Option<String>,
}

impl OrganizerDecisionErrors {
    pub fn is_empty(&self) -> bool {
        self.request.is_none()
    }
}

impl fmt::Display for OrganizerDecisionErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.request.as_deref().unwrap_or_default())
    }
}

/// One bounded fact derived from a candidate's response counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateSummaryFact {
    EveryoneAvailable,
    EveryoneAvailableIncludingMaybe,
    OneUnavailable,
    UniqueMostAvailable,
}

/// Organizer-only aggregate for one candidate, retaining authored order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateResponseSummary {
    pub id: i64,
    pub local_date: String,
    pub local_time: String,
    pub available_count: u64,
    pub maybe_count: u64,
    pub unavailable_count: u64,
    pub fact: Option<CandidateSummaryFact>,
}

/// One plain-text comment selected for the bounded organizer preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseCommentPreview {
    pub respondent_name: String,
    pub comment: String,
}

/// The one candidate explicitly selected by the organizer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizerEventDecision {
    pub candidate_id: i64,
    pub local_date: String,
    pub local_time: String,
}

/// Organizer-only response summary loaded from one database snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizerEventSummary {
    pub public_id: String,
    pub name: String,
    pub organizer_note: Option<String>,
    pub time_zone: String,
    pub response_count: u64,
    pub candidates: Vec<CandidateResponseSummary>,
    pub comment_count: u64,
    pub comment_previews: Vec<ResponseCommentPreview>,
    pub decision: Option<OrganizerEventDecision>,
}

/// One candidate column in the organizer-only response matrix.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseMatrixCandidate {
    pub local_date: String,
    pub local_time: String,
}

/// One stored anonymous response projected across every candidate column.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseMatrixRow {
    pub respondent_name: String,
    pub availabilities: Vec<Availability>,
}

/// Complete response matrix loaded from one authorized database snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseMatrix {
    pub name: String,
    pub time_zone: String,
    pub candidates: Vec<ResponseMatrixCandidate>,
    pub responses: Vec<ResponseMatrixRow>,
}

/// Existing organizer-authorized name retained at its server boundary.
pub type OrganizerResponseMatrix = ResponseMatrix;

/// Matrix returned only after one participant answer has been accepted.
pub type ParticipantResponseMatrix = ResponseMatrix;

/// Attach at most one conservative fact to every candidate summary.
pub fn derive_candidate_summary_facts(
    response_count: u64,
    candidates: &mut [CandidateResponseSummary],
) {
    let unique_most_available_id = if response_count == 0 {
        None
    } else {
        candidates
            .iter()
            .map(|candidate| candidate.available_count)
            .max()
            .filter(|maximum| *maximum > 0)
            .and_then(|maximum| {
                let mut matching = candidates
                    .iter()
                    .filter(|candidate| candidate.available_count == maximum);
                let candidate = matching.next()?;
                matching.next().is_none().then_some(candidate.id)
            })
    };

    for candidate in candidates {
        candidate.fact = if response_count == 0 {
            None
        } else if candidate.available_count == response_count {
            Some(CandidateSummaryFact::EveryoneAvailable)
        } else if candidate.unavailable_count == 0 {
            Some(CandidateSummaryFact::EveryoneAvailableIncludingMaybe)
        } else if candidate.unavailable_count == 1 {
            Some(CandidateSummaryFact::OneUnavailable)
        } else if unique_most_available_id == Some(candidate.id) {
            Some(CandidateSummaryFact::UniqueMostAvailable)
        } else {
            None
        };
    }
}
