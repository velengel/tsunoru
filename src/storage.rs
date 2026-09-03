//! SQLite persistence used only by the fullstack server build.

#![cfg(feature = "server")]

use crate::domain::{
    AccountEventTrace, AccountEventTraceCandidate, AccountEventTraceRelationship,
    AccountEventTraceResponse, AccountHistory, Availability, CandidateResponseSummary,
    EventContinuationCreateInput, EventContinuationPlan, HistoryDecision, NewEventInput,
    OrganizedEventHistoryItem, OrganizedEventSeriesHistory, OrganizerEventDecision,
    OrganizerEventSummary, OrganizerResponseMatrix, ParticipantResponseMatrix,
    ParticipatedEventHistoryItem, PreparedAvailabilityResponse, PublicCandidate, PublicEvent,
    PublicEventDecision, ResponseCommentPreview, ResponseMatrixCandidate, ResponseMatrixRow,
    derive_candidate_summary_facts, derive_event_series_name, suggest_next_event_name,
};
use anyhow::Context;
use dioxus::fullstack::Lazy;
use sqlx::{
    FromRow, SqlitePool,
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use std::{collections::HashMap, fmt, path::Path, str::FromStr, time::Duration};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

static DATABASE: Lazy<SqlitePool> = Lazy::new(|| async {
    let pool = open_file(Path::new("var/tsunoru.sqlite3")).await?;
    Ok::<_, anyhow::Error>(pool)
});

const SESSION_IDLE_SECONDS: i64 = 7 * 24 * 60 * 60;
const SESSION_ABSOLUTE_SECONDS: i64 = 30 * 24 * 60 * 60;
const SESSION_TOUCH_INTERVAL_SECONDS: i64 = 60 * 60;

/// Server-only account identity used while linking writes and issuing sessions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountRecord {
    pub id: i64,
    pub login_id: String,
}

/// Password record loaded only for one login verification.
#[derive(Clone, PartialEq, Eq)]
pub struct StoredAccountCredentials {
    pub id: i64,
    pub login_id: String,
    pub password_hash_phc: String,
}

impl fmt::Debug for StoredAccountCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredAccountCredentials")
            .field("id", &self.id)
            .field("login_id", &self.login_id)
            .field("password_hash_phc", &"[REDACTED]")
            .finish()
    }
}

#[derive(FromRow)]
struct EventRow {
    name: String,
    organizer_note: Option<String>,
    time_zone: String,
}

#[derive(FromRow)]
struct CandidateRow {
    id: i64,
    local_date: String,
    local_time: String,
}

#[derive(FromRow)]
struct StoredResponseRow {
    id: i64,
    event_public_id: String,
    respondent_name: String,
}

#[derive(FromRow)]
struct StoredAvailabilityRow {
    candidate_id: i64,
    availability: String,
}

#[derive(FromRow)]
struct StoredResponseCommentRow {
    id: i64,
    respondent_comment: Option<String>,
}

#[derive(FromRow)]
struct CandidateResponseSummaryRow {
    id: i64,
    local_date: String,
    local_time: String,
    available_count: i64,
    maybe_count: i64,
    unavailable_count: i64,
}

#[derive(FromRow)]
struct ResponseCommentPreviewRow {
    respondent_name: String,
    comment: String,
}

#[derive(FromRow)]
struct StoredMatrixResponseRow {
    id: i64,
    respondent_name: String,
}

#[derive(FromRow)]
struct StoredMatrixAvailabilityRow {
    response_id: i64,
    candidate_id: i64,
    availability: String,
}

#[derive(FromRow)]
struct StoredEventDecisionProjectionRow {
    candidate_id: i64,
    local_date: Option<String>,
    local_time: Option<String>,
}

#[derive(FromRow)]
struct SessionAccountRow {
    id: i64,
    login_id: String,
    last_seen_at: i64,
    expires_at: i64,
}

#[derive(FromRow)]
struct OrganizedHistoryRow {
    public_id: String,
    name: String,
    time_zone: String,
    local_date: Option<String>,
    local_time: Option<String>,
    response_count: i64,
}

#[derive(FromRow)]
struct ParticipatedHistoryRow {
    public_id: String,
    name: String,
    time_zone: String,
    local_date: Option<String>,
    local_time: Option<String>,
}

#[derive(FromRow)]
struct OrganizedSeriesHistoryRow {
    series_id: i64,
    display_name: String,
    member_owner_account_id: Option<i64>,
    event_public_id: Option<String>,
    position: Option<i64>,
    name: Option<String>,
    time_zone: Option<String>,
    created_at: Option<String>,
    organizer_account_id: Option<i64>,
    local_date: Option<String>,
    local_time: Option<String>,
    response_count: Option<i64>,
}

#[derive(FromRow)]
struct StoredSeriesMembershipRow {
    series_id: i64,
    owner_account_id: i64,
    series_owner_account_id: Option<i64>,
    display_name: Option<String>,
}

#[derive(FromRow)]
struct StoredSeriesMemberRow {
    owner_account_id: i64,
    event_public_id: String,
    position: i64,
    joined_event_public_id: Option<String>,
    event_name: Option<String>,
    organizer_account_id: Option<i64>,
}

struct ContinuationSeriesSnapshot {
    series_id: Option<i64>,
    series_name: String,
    tail_event_public_id: String,
    tail_event_name: String,
    next_position: i64,
}

struct OrganizedSeriesHistoryBuilder {
    series_id: i64,
    series_name: String,
    latest_created_at: String,
    latest_public_id: String,
    events: Vec<OrganizedEventHistoryItem>,
}

#[derive(FromRow)]
struct AccountEventTraceRow {
    name: String,
    organizer_note: Option<String>,
    time_zone: String,
    is_organizer: i64,
    is_participant: i64,
}

#[derive(FromRow)]
struct AccountEventTraceResponseRow {
    id: i64,
    respondent_name: String,
    respondent_comment: Option<String>,
    is_current_account: i64,
}

/// Whether an idempotent response write created a row or found the same aggregate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseWriteOutcome {
    Created,
    AlreadyRecorded,
}

/// Account-session result observed inside an event or response write transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionWriteStatus {
    NotPresented,
    Active,
    Inactive,
}

/// A committed write paired with the session state resolved in that same transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionWrite<T> {
    pub value: T,
    pub session_status: SessionWriteStatus,
}

/// Expected failures from the anonymous response transaction boundary.
#[derive(Debug)]
pub enum ResponseStorageError {
    EventNotFound,
    CandidateSetMismatch,
    EventDecided,
    CapabilityConflict,
    Database(sqlx::Error),
}

impl fmt::Display for ResponseStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventNotFound => write!(formatter, "event not found"),
            Self::CandidateSetMismatch => {
                write!(formatter, "response candidate set does not match event")
            }
            Self::EventDecided => write!(formatter, "event already has a final decision"),
            Self::CapabilityConflict => {
                write!(formatter, "response capability already has another payload")
            }
            Self::Database(error) => write!(formatter, "response database failure: {error}"),
        }
    }
}

impl std::error::Error for ResponseStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for ResponseStorageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// Expected failures while projecting data safe for a public-by-link route.
#[derive(Debug)]
pub enum PublicEventStorageError {
    DataInvariantViolation,
    Database(sqlx::Error),
}

impl fmt::Display for PublicEventStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataInvariantViolation => {
                write!(formatter, "public event data invariant violation")
            }
            Self::Database(error) => write!(formatter, "public event database failure: {error}"),
        }
    }
}

impl std::error::Error for PublicEventStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::DataInvariantViolation => None,
        }
    }
}

impl From<sqlx::Error> for PublicEventStorageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// Whether an idempotent response-comment write created a value or found the same value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseCommentWriteOutcome {
    Created,
    AlreadyRecorded,
}

/// Expected failures from the authorized response-comment transaction boundary.
#[derive(Debug)]
pub enum ResponseCommentStorageError {
    ResponseNotFound,
    CommentConflict,
    Database(sqlx::Error),
}

impl fmt::Display for ResponseCommentStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResponseNotFound => write!(formatter, "response not found"),
            Self::CommentConflict => write!(formatter, "response already has another comment"),
            Self::Database(error) => {
                write!(formatter, "response comment database failure: {error}")
            }
        }
    }
}

impl std::error::Error for ResponseCommentStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for ResponseCommentStorageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// Whether an immutable event decision was created or replayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventDecisionWriteOutcome {
    Created,
    AlreadyDecided,
}

/// Expected failures from the organizer-only event decision transaction boundary.
#[derive(Debug)]
pub enum OrganizerDecisionStorageError {
    NotFound,
    CandidateMismatch,
    Conflict,
    Database(sqlx::Error),
}

impl fmt::Display for OrganizerDecisionStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "organizer event not found"),
            Self::CandidateMismatch => write!(formatter, "candidate does not belong to event"),
            Self::Conflict => write!(formatter, "event already has another decision"),
            Self::Database(error) => {
                write!(formatter, "organizer decision database failure: {error}")
            }
        }
    }
}

impl std::error::Error for OrganizerDecisionStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for OrganizerDecisionStorageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// Expected failures from the organizer-only response summary boundary.
#[derive(Debug)]
pub enum OrganizerSummaryStorageError {
    NotFound,
    DataInvariantViolation,
    Database(sqlx::Error),
}

impl fmt::Display for OrganizerSummaryStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "organizer summary not found"),
            Self::DataInvariantViolation => {
                write!(formatter, "organizer summary data invariant violation")
            }
            Self::Database(error) => {
                write!(formatter, "organizer summary database failure: {error}")
            }
        }
    }
}

impl std::error::Error for OrganizerSummaryStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for OrganizerSummaryStorageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// Expected failures while authorizing or reconstructing a complete response matrix.
#[derive(Debug)]
pub enum OrganizerResponseMatrixStorageError {
    NotFound,
    DataInvariantViolation,
    Database(sqlx::Error),
}

impl fmt::Display for OrganizerResponseMatrixStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "organizer response matrix not found"),
            Self::DataInvariantViolation => {
                write!(
                    formatter,
                    "organizer response matrix data invariant violation"
                )
            }
            Self::Database(error) => {
                write!(
                    formatter,
                    "organizer response matrix database failure: {error}"
                )
            }
        }
    }
}

impl std::error::Error for OrganizerResponseMatrixStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for OrganizerResponseMatrixStorageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// Neutral name used by participant and organizer authorization boundaries.
pub type ResponseMatrixStorageError = OrganizerResponseMatrixStorageError;

#[derive(Debug)]
enum OrganizerEventSnapshotError {
    NotFound,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for OrganizerEventSnapshotError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<OrganizerEventSnapshotError> for OrganizerSummaryStorageError {
    fn from(error: OrganizerEventSnapshotError) -> Self {
        match error {
            OrganizerEventSnapshotError::NotFound => Self::NotFound,
            OrganizerEventSnapshotError::Database(error) => Self::Database(error),
        }
    }
}

impl From<OrganizerEventSnapshotError> for OrganizerResponseMatrixStorageError {
    fn from(error: OrganizerEventSnapshotError) -> Self {
        match error {
            OrganizerEventSnapshotError::NotFound => Self::NotFound,
            OrganizerEventSnapshotError::Database(error) => Self::Database(error),
        }
    }
}

/// Expected failures while creating an account and its first session.
#[derive(Debug)]
pub enum AccountStorageError {
    LoginIdTaken,
    AccountNotFound,
    Database(sqlx::Error),
}

impl fmt::Display for AccountStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoginIdTaken => write!(formatter, "account could not be created"),
            Self::AccountNotFound => write!(formatter, "account not found"),
            Self::Database(error) => write!(formatter, "account database failure: {error}"),
        }
    }
}

impl std::error::Error for AccountStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

/// Expected failures while reading one authenticated history projection.
#[derive(Debug)]
pub enum AccountHistoryStorageError {
    Unauthenticated,
    DataInvariantViolation,
    Database(sqlx::Error),
}

impl fmt::Display for AccountHistoryStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthenticated => write!(formatter, "account session is not active"),
            Self::DataInvariantViolation => {
                write!(formatter, "account history invariant violation")
            }
            Self::Database(error) => write!(formatter, "account history database failure: {error}"),
        }
    }
}

impl std::error::Error for AccountHistoryStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for AccountHistoryStorageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// Expected failures while planning or atomically creating an organizer-owned continuation.
#[derive(Debug)]
pub enum EventContinuationStorageError {
    Unauthenticated,
    NotFound,
    Stale,
    DataInvariantViolation,
    Database(sqlx::Error),
}

impl fmt::Display for EventContinuationStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthenticated => write!(formatter, "account session is not active"),
            Self::NotFound => write!(formatter, "event continuation not found"),
            Self::Stale => write!(formatter, "event continuation plan is stale"),
            Self::DataInvariantViolation => {
                write!(formatter, "event continuation data invariant violation")
            }
            Self::Database(error) => {
                write!(formatter, "event continuation database failure: {error}")
            }
        }
    }
}

impl std::error::Error for EventContinuationStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for EventContinuationStorageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// Expected failures while authorizing and reading one private event trace.
#[derive(Debug)]
pub enum AccountEventTraceStorageError {
    Unauthenticated,
    NotFound,
    DataInvariantViolation,
    Database(sqlx::Error),
}

impl fmt::Display for AccountEventTraceStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthenticated => write!(formatter, "account session is not active"),
            Self::NotFound => write!(formatter, "account event trace not found"),
            Self::DataInvariantViolation => {
                write!(formatter, "account event trace invariant violation")
            }
            Self::Database(error) => {
                write!(formatter, "account event trace database failure: {error}")
            }
        }
    }
}

impl std::error::Error for AccountEventTraceStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for AccountEventTraceStorageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// Create one account and its initial server-side session atomically.
pub async fn create_account_with_session(
    pool: &SqlitePool,
    login_id: &str,
    password_hash_phc: &str,
    token_hash: &[u8; 32],
    now: i64,
) -> Result<AccountRecord, AccountStorageError> {
    create_account_with_session_replacing(pool, login_id, password_hash_phc, token_hash, None, now)
        .await
}

/// Create one account while atomically replacing the session presented by this browser.
pub async fn create_account_with_session_replacing(
    pool: &SqlitePool,
    login_id: &str,
    password_hash_phc: &str,
    token_hash: &[u8; 32],
    previous_token_hash: Option<&[u8; 32]>,
    now: i64,
) -> Result<AccountRecord, AccountStorageError> {
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(AccountStorageError::Database)?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO accounts (login_id, password_hash_phc, created_at)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(login_id)
    .bind(password_hash_phc)
    .bind(now)
    .execute(transaction.as_mut())
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|database| database.is_unique_violation())
        {
            AccountStorageError::LoginIdTaken
        } else {
            AccountStorageError::Database(error)
        }
    })?;
    let account_id = inserted.last_insert_rowid();
    delete_account_session_in_transaction(&mut transaction, previous_token_hash)
        .await
        .map_err(AccountStorageError::Database)?;
    insert_account_session(&mut transaction, account_id, token_hash, now)
        .await
        .map_err(AccountStorageError::Database)?;
    transaction
        .commit()
        .await
        .map_err(AccountStorageError::Database)?;
    Ok(AccountRecord {
        id: account_id,
        login_id: login_id.to_owned(),
    })
}

/// Load one normalized login ID and password PHC string for verification.
pub async fn find_account_credentials(
    pool: &SqlitePool,
    login_id: &str,
) -> Result<Option<StoredAccountCredentials>, sqlx::Error> {
    sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, login_id, password_hash_phc FROM accounts WHERE login_id = ?",
    )
    .bind(login_id)
    .fetch_optional(pool)
    .await
    .map(|row| {
        row.map(
            |(id, login_id, password_hash_phc)| StoredAccountCredentials {
                id,
                login_id,
                password_hash_phc,
            },
        )
    })
}

/// Add a new session for an already verified account.
pub async fn create_account_session(
    pool: &SqlitePool,
    account_id: i64,
    token_hash: &[u8; 32],
    now: i64,
) -> Result<(), AccountStorageError> {
    create_account_session_replacing(pool, account_id, token_hash, None, now).await
}

/// Add a verified account session and revoke the prior browser session atomically.
pub async fn create_account_session_replacing(
    pool: &SqlitePool,
    account_id: i64,
    token_hash: &[u8; 32],
    previous_token_hash: Option<&[u8; 32]>,
    now: i64,
) -> Result<(), AccountStorageError> {
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(AccountStorageError::Database)?;
    let account_exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM accounts WHERE id = ?")
        .bind(account_id)
        .fetch_optional(transaction.as_mut())
        .await
        .map_err(AccountStorageError::Database)?;
    if account_exists.is_none() {
        return Err(AccountStorageError::AccountNotFound);
    }
    delete_account_session_in_transaction(&mut transaction, previous_token_hash)
        .await
        .map_err(AccountStorageError::Database)?;
    insert_account_session(&mut transaction, account_id, token_hash, now)
        .await
        .map_err(AccountStorageError::Database)?;
    transaction
        .commit()
        .await
        .map_err(AccountStorageError::Database)
}

async fn delete_account_session_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    token_hash: Option<&[u8; 32]>,
) -> Result<(), sqlx::Error> {
    if let Some(token_hash) = token_hash {
        sqlx::query("DELETE FROM account_sessions WHERE token_hash = ?")
            .bind(token_hash.as_slice())
            .execute(transaction.as_mut())
            .await?;
    }
    Ok(())
}

async fn insert_account_session(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    account_id: i64,
    token_hash: &[u8; 32],
    now: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO account_sessions (
            token_hash, account_id, created_at, last_seen_at, expires_at
        ) VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(token_hash.as_slice())
    .bind(account_id)
    .bind(now)
    .bind(now)
    .bind(now.saturating_add(SESSION_ABSOLUTE_SECONDS))
    .execute(transaction.as_mut())
    .await?;
    Ok(())
}

/// Resolve and occasionally touch one active session; expired rows are removed.
pub async fn resolve_account_session(
    pool: &SqlitePool,
    token_hash: &[u8; 32],
    now: i64,
) -> Result<Option<AccountRecord>, sqlx::Error> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let account = resolve_account_session_in_transaction(&mut transaction, token_hash, now).await?;
    transaction.commit().await?;
    Ok(account)
}

async fn resolve_account_session_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    token_hash: &[u8; 32],
    now: i64,
) -> Result<Option<AccountRecord>, sqlx::Error> {
    let stored = sqlx::query_as::<_, SessionAccountRow>(
        r#"
        SELECT a.id, a.login_id, s.last_seen_at, s.expires_at
        FROM account_sessions AS s
        JOIN accounts AS a ON a.id = s.account_id
        WHERE s.token_hash = ?
        "#,
    )
    .bind(token_hash.as_slice())
    .fetch_optional(transaction.as_mut())
    .await?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    let idle_deadline = stored.last_seen_at.saturating_add(SESSION_IDLE_SECONDS);
    if stored.expires_at <= now || idle_deadline <= now {
        sqlx::query("DELETE FROM account_sessions WHERE token_hash = ?")
            .bind(token_hash.as_slice())
            .execute(transaction.as_mut())
            .await?;
        return Ok(None);
    }
    if stored
        .last_seen_at
        .saturating_add(SESSION_TOUCH_INTERVAL_SECONDS)
        <= now
    {
        sqlx::query("UPDATE account_sessions SET last_seen_at = ? WHERE token_hash = ?")
            .bind(now)
            .bind(token_hash.as_slice())
            .execute(transaction.as_mut())
            .await?;
    }
    Ok(Some(AccountRecord {
        id: stored.id,
        login_id: stored.login_id,
    }))
}

async fn resolve_write_session_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    token_hash: Option<&[u8; 32]>,
    now: i64,
) -> Result<(Option<i64>, SessionWriteStatus), sqlx::Error> {
    let Some(token_hash) = token_hash else {
        return Ok((None, SessionWriteStatus::NotPresented));
    };
    match resolve_account_session_in_transaction(transaction, token_hash, now).await? {
        Some(account) => Ok((Some(account.id), SessionWriteStatus::Active)),
        None => Ok((None, SessionWriteStatus::Inactive)),
    }
}

/// Invalidate one session. Repeated logout is intentionally successful.
pub async fn delete_account_session(
    pool: &SqlitePool,
    token_hash: &[u8; 32],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM account_sessions WHERE token_hash = ?")
        .bind(token_hash.as_slice())
        .execute(pool)
        .await?;
    Ok(())
}

/// Return the lazily initialized application pool.
pub fn database_pool() -> &'static SqlitePool {
    &DATABASE
}

/// Open an isolated SQLite database and apply all migrations.
pub async fn open_in_memory() -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// Open a file-backed SQLite database and apply all migrations.
pub async fn open_file(path: &Path) -> anyhow::Result<SqlitePool> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create SQLite directory {}", parent.display()))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// Persist the event and every candidate in one transaction.
pub async fn create_event_record(
    pool: &SqlitePool,
    public_id: &str,
    organizer_capability_hash: &str,
    input: &NewEventInput,
) -> anyhow::Result<PublicEvent> {
    Ok(
        create_event_record_for_session(pool, public_id, organizer_capability_hash, input, None, 0)
            .await?
            .value,
    )
}

/// Persist an event and link it only when the supplied session is active in the same transaction.
pub async fn create_event_record_for_session(
    pool: &SqlitePool,
    public_id: &str,
    organizer_capability_hash: &str,
    input: &NewEventInput,
    session_token_hash: Option<&[u8; 32]>,
    now: i64,
) -> anyhow::Result<SessionWrite<PublicEvent>> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let (organizer_account_id, session_status) =
        resolve_write_session_in_transaction(&mut transaction, session_token_hash, now).await?;

    sqlx::query(
        r#"
        INSERT INTO events (
            public_id,
            name,
            organizer_note,
            time_zone,
            organizer_capability_hash,
            organizer_account_id
        ) VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(public_id)
    .bind(&input.name)
    .bind(input.organizer_note.as_deref())
    .bind(&input.time_zone)
    .bind(organizer_capability_hash)
    .bind(organizer_account_id)
    .execute(transaction.as_mut())
    .await?;

    let mut candidates = Vec::with_capacity(input.candidates.len());
    for (position, candidate) in input.candidates.iter().enumerate() {
        let inserted = sqlx::query(
            r#"
            INSERT INTO candidates (
                event_public_id,
                position,
                local_date,
                local_time
            ) VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(public_id)
        .bind(i64::try_from(position).context("candidate position exceeds SQLite integer")?)
        .bind(&candidate.local_date)
        .bind(&candidate.local_time)
        .execute(transaction.as_mut())
        .await?;

        candidates.push(PublicCandidate {
            id: inserted.last_insert_rowid(),
            local_date: candidate.local_date.clone(),
            local_time: candidate.local_time.clone(),
        });
    }

    transaction.commit().await?;
    Ok(SessionWrite {
        value: PublicEvent {
            public_id: public_id.to_owned(),
            name: input.name.clone(),
            organizer_note: input.organizer_note.clone(),
            time_zone: input.time_zone.clone(),
            candidates,
            decision: None,
        },
        session_status,
    })
}

/// Authorize one organizer-owned origin and return its current series tail.
pub async fn find_event_continuation_plan_by_session(
    pool: &SqlitePool,
    token_hash: &[u8; 32],
    origin_event_public_id: &str,
    now: i64,
) -> Result<EventContinuationPlan, EventContinuationStorageError> {
    let account = resolve_account_session(pool, token_hash, now)
        .await?
        .ok_or(EventContinuationStorageError::Unauthenticated)?;
    let mut transaction = pool.begin().await?;
    let origin_event_name =
        authorize_continuation_origin(&mut transaction, account.id, origin_event_public_id).await?;
    let series = load_continuation_series_snapshot(
        &mut transaction,
        account.id,
        origin_event_public_id,
        &origin_event_name,
    )
    .await?;
    let plan = EventContinuationPlan {
        origin_event_public_id: origin_event_public_id.to_owned(),
        origin_event_name,
        series_name: series.series_name,
        tail_event_public_id: series.tail_event_public_id,
        suggested_event_name: suggest_next_event_name(&series.tail_event_name),
    };
    transaction.commit().await?;
    Ok(plan)
}

/// Create one event aggregate and append it to the current series in one write transaction.
pub async fn create_event_continuation_by_session(
    pool: &SqlitePool,
    new_event_public_id: &str,
    organizer_capability_hash: &str,
    input: &EventContinuationCreateInput,
    token_hash: &[u8; 32],
    now: i64,
) -> Result<PublicEvent, EventContinuationStorageError> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let Some(account) =
        resolve_account_session_in_transaction(&mut transaction, token_hash, now).await?
    else {
        transaction.commit().await?;
        return Err(EventContinuationStorageError::Unauthenticated);
    };
    let origin_event_name =
        authorize_continuation_origin(&mut transaction, account.id, &input.origin_event_public_id)
            .await?;
    let series = load_continuation_series_snapshot(
        &mut transaction,
        account.id,
        &input.origin_event_public_id,
        &origin_event_name,
    )
    .await?;
    if series.tail_event_public_id != input.expected_tail_event_public_id {
        return Err(EventContinuationStorageError::Stale);
    }

    let series_id = match series.series_id {
        Some(series_id) => series_id,
        None => {
            let inserted = sqlx::query(
                r#"
                INSERT INTO event_series (owner_account_id, display_name, created_at)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(account.id)
            .bind(&series.series_name)
            .bind(now)
            .execute(transaction.as_mut())
            .await?;
            let series_id = inserted.last_insert_rowid();
            sqlx::query(
                r#"
                INSERT INTO event_series_members (
                    series_id, owner_account_id, event_public_id, position
                ) VALUES (?, ?, ?, 0)
                "#,
            )
            .bind(series_id)
            .bind(account.id)
            .bind(&input.origin_event_public_id)
            .execute(transaction.as_mut())
            .await?;
            series_id
        }
    };

    let created = insert_continuation_event_aggregate(
        &mut transaction,
        new_event_public_id,
        organizer_capability_hash,
        account.id,
        &input.event,
    )
    .await?;
    sqlx::query(
        r#"
        INSERT INTO event_series_members (
            series_id, owner_account_id, event_public_id, position
        ) VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(series_id)
    .bind(account.id)
    .bind(new_event_public_id)
    .bind(series.next_position)
    .execute(transaction.as_mut())
    .await?;
    transaction.commit().await?;
    Ok(created)
}

async fn authorize_continuation_origin(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    account_id: i64,
    origin_event_public_id: &str,
) -> Result<String, EventContinuationStorageError> {
    sqlx::query_scalar(
        r#"
        SELECT name
        FROM events
        WHERE public_id = ? AND organizer_account_id = ?
        "#,
    )
    .bind(origin_event_public_id)
    .bind(account_id)
    .fetch_optional(transaction.as_mut())
    .await?
    .ok_or(EventContinuationStorageError::NotFound)
}

async fn load_continuation_series_snapshot(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    account_id: i64,
    origin_event_public_id: &str,
    origin_event_name: &str,
) -> Result<ContinuationSeriesSnapshot, EventContinuationStorageError> {
    let membership = sqlx::query_as::<_, StoredSeriesMembershipRow>(
        r#"
        SELECT
            m.series_id,
            m.owner_account_id,
            s.owner_account_id AS series_owner_account_id,
            s.display_name
        FROM event_series_members AS m
        LEFT JOIN event_series AS s ON s.id = m.series_id
        WHERE m.event_public_id = ?
        "#,
    )
    .bind(origin_event_public_id)
    .fetch_optional(transaction.as_mut())
    .await?;
    let Some(membership) = membership else {
        return Ok(ContinuationSeriesSnapshot {
            series_id: None,
            series_name: derive_event_series_name(origin_event_name),
            tail_event_public_id: origin_event_public_id.to_owned(),
            tail_event_name: origin_event_name.to_owned(),
            next_position: 1,
        });
    };
    let (Some(series_owner_account_id), Some(display_name)) =
        (membership.series_owner_account_id, membership.display_name)
    else {
        return Err(EventContinuationStorageError::DataInvariantViolation);
    };
    if membership.owner_account_id != account_id
        || series_owner_account_id != account_id
        || display_name.trim().is_empty()
    {
        return Err(EventContinuationStorageError::DataInvariantViolation);
    }

    let members = sqlx::query_as::<_, StoredSeriesMemberRow>(
        r#"
        SELECT
            m.owner_account_id,
            m.event_public_id,
            m.position,
            e.public_id AS joined_event_public_id,
            e.name AS event_name,
            e.organizer_account_id
        FROM event_series_members AS m
        LEFT JOIN events AS e ON e.public_id = m.event_public_id
        WHERE m.series_id = ?
        ORDER BY m.position ASC
        "#,
    )
    .bind(membership.series_id)
    .fetch_all(transaction.as_mut())
    .await?;
    if members.len() < 2 {
        return Err(EventContinuationStorageError::DataInvariantViolation);
    }
    for (expected_position, member) in members.iter().enumerate() {
        let expected_position = i64::try_from(expected_position)
            .map_err(|_| EventContinuationStorageError::DataInvariantViolation)?;
        if member.position != expected_position
            || member.owner_account_id != account_id
            || member.joined_event_public_id.as_deref() != Some(&member.event_public_id)
            || member.organizer_account_id != Some(account_id)
            || member.event_name.is_none()
        {
            return Err(EventContinuationStorageError::DataInvariantViolation);
        }
    }
    let tail = members
        .last()
        .ok_or(EventContinuationStorageError::DataInvariantViolation)?;
    let next_position = tail
        .position
        .checked_add(1)
        .ok_or(EventContinuationStorageError::DataInvariantViolation)?;
    Ok(ContinuationSeriesSnapshot {
        series_id: Some(membership.series_id),
        series_name: display_name,
        tail_event_public_id: tail.event_public_id.clone(),
        tail_event_name: tail
            .event_name
            .clone()
            .ok_or(EventContinuationStorageError::DataInvariantViolation)?,
        next_position,
    })
}

async fn insert_continuation_event_aggregate(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    public_id: &str,
    organizer_capability_hash: &str,
    organizer_account_id: i64,
    input: &NewEventInput,
) -> Result<PublicEvent, EventContinuationStorageError> {
    sqlx::query(
        r#"
        INSERT INTO events (
            public_id, name, organizer_note, time_zone,
            organizer_capability_hash, organizer_account_id
        ) VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(public_id)
    .bind(&input.name)
    .bind(input.organizer_note.as_deref())
    .bind(&input.time_zone)
    .bind(organizer_capability_hash)
    .bind(organizer_account_id)
    .execute(transaction.as_mut())
    .await?;

    let mut candidates = Vec::with_capacity(input.candidates.len());
    for (position, candidate) in input.candidates.iter().enumerate() {
        let position = i64::try_from(position)
            .map_err(|_| EventContinuationStorageError::DataInvariantViolation)?;
        let inserted = sqlx::query(
            r#"
            INSERT INTO candidates (
                event_public_id, position, local_date, local_time
            ) VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(public_id)
        .bind(position)
        .bind(&candidate.local_date)
        .bind(&candidate.local_time)
        .execute(transaction.as_mut())
        .await?;
        candidates.push(PublicCandidate {
            id: inserted.last_insert_rowid(),
            local_date: candidate.local_date.clone(),
            local_time: candidate.local_time.clone(),
        });
    }
    Ok(PublicEvent {
        public_id: public_id.to_owned(),
        name: input.name.clone(),
        organizer_note: input.organizer_note.clone(),
        time_zone: input.time_zone.clone(),
        candidates,
        decision: None,
    })
}

/// Load only information that is safe for the public-by-link route.
pub async fn find_public_event(
    pool: &SqlitePool,
    public_id: &str,
) -> Result<Option<PublicEvent>, PublicEventStorageError> {
    let mut transaction = pool.begin().await?;
    let event = sqlx::query_as::<_, EventRow>(
        "SELECT name, organizer_note, time_zone FROM events WHERE public_id = ?",
    )
    .bind(public_id)
    .fetch_optional(transaction.as_mut())
    .await?;

    let Some(event) = event else {
        transaction.commit().await?;
        return Ok(None);
    };

    let candidates = sqlx::query_as::<_, CandidateRow>(
        r#"
        SELECT id, local_date, local_time
        FROM candidates
        WHERE event_public_id = ?
        ORDER BY position ASC
        "#,
    )
    .bind(public_id)
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|candidate| PublicCandidate {
        id: candidate.id,
        local_date: candidate.local_date,
        local_time: candidate.local_time,
    })
    .collect();

    let decision_row = sqlx::query_as::<_, StoredEventDecisionProjectionRow>(
        r#"
        SELECT d.candidate_id, c.local_date, c.local_time
        FROM event_decisions AS d
        LEFT JOIN candidates AS c
          ON c.id = d.candidate_id
         AND c.event_public_id = d.event_public_id
        WHERE d.event_public_id = ?
        "#,
    )
    .bind(public_id)
    .fetch_optional(transaction.as_mut())
    .await?;
    let decision = match decision_row {
        None => None,
        Some(stored) => {
            let (Some(local_date), Some(local_time)) = (stored.local_date, stored.local_time)
            else {
                return Err(PublicEventStorageError::DataInvariantViolation);
            };
            Some(PublicEventDecision {
                candidate_id: stored.candidate_id,
                local_date,
                local_time,
            })
        }
    };

    transaction.commit().await?;
    Ok(Some(PublicEvent {
        public_id: public_id.to_owned(),
        name: event.name,
        organizer_note: event.organizer_note,
        time_zone: event.time_zone,
        candidates,
        decision,
    }))
}

/// Load the two role-specific account histories from one authenticated snapshot.
pub async fn find_account_history_by_session(
    pool: &SqlitePool,
    token_hash: &[u8; 32],
    now: i64,
) -> Result<AccountHistory, AccountHistoryStorageError> {
    let account = resolve_account_session(pool, token_hash, now)
        .await?
        .ok_or(AccountHistoryStorageError::Unauthenticated)?;
    let mut transaction = pool.begin().await?;

    let invalid_owned_membership: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT 1
        FROM events AS e
        JOIN event_series_members AS m ON m.event_public_id = e.public_id
        LEFT JOIN event_series AS s ON s.id = m.series_id
        WHERE e.organizer_account_id = ?
          AND (
              m.owner_account_id <> ?
              OR s.owner_account_id IS NULL
              OR s.owner_account_id <> ?
          )
        LIMIT 1
        "#,
    )
    .bind(account.id)
    .bind(account.id)
    .bind(account.id)
    .fetch_optional(transaction.as_mut())
    .await?;
    if invalid_owned_membership.is_some() {
        return Err(AccountHistoryStorageError::DataInvariantViolation);
    }

    let series_rows = sqlx::query_as::<_, OrganizedSeriesHistoryRow>(
        r#"
        SELECT
            s.id AS series_id,
            s.display_name,
            m.owner_account_id AS member_owner_account_id,
            e.public_id AS event_public_id,
            m.position,
            e.name,
            e.time_zone,
            e.created_at,
            e.organizer_account_id,
            c.local_date,
            c.local_time,
            CASE WHEN e.public_id IS NULL THEN NULL ELSE (
                SELECT COUNT(*) FROM responses AS r
                WHERE r.event_public_id = e.public_id
            ) END AS response_count
        FROM event_series AS s
        LEFT JOIN event_series_members AS m ON m.series_id = s.id
        LEFT JOIN events AS e ON e.public_id = m.event_public_id
        LEFT JOIN event_decisions AS d ON d.event_public_id = e.public_id
        LEFT JOIN candidates AS c
          ON c.id = d.candidate_id
         AND c.event_public_id = d.event_public_id
        WHERE s.owner_account_id = ?
        ORDER BY s.id ASC, m.position ASC
        "#,
    )
    .bind(account.id)
    .fetch_all(transaction.as_mut())
    .await?;
    let mut series_builders: Vec<OrganizedSeriesHistoryBuilder> = Vec::new();
    for row in series_rows {
        let display_name = row.display_name;
        let (
            Some(member_owner_account_id),
            Some(event_public_id),
            Some(position),
            Some(name),
            Some(time_zone),
            Some(created_at),
            Some(organizer_account_id),
            Some(response_count),
        ) = (
            row.member_owner_account_id,
            row.event_public_id,
            row.position,
            row.name,
            row.time_zone,
            row.created_at,
            row.organizer_account_id,
            row.response_count,
        )
        else {
            return Err(AccountHistoryStorageError::DataInvariantViolation);
        };
        if member_owner_account_id != account.id || organizer_account_id != account.id {
            return Err(AccountHistoryStorageError::DataInvariantViolation);
        }
        if series_builders
            .last()
            .is_none_or(|builder| builder.series_id != row.series_id)
        {
            if series_builders
                .last()
                .is_some_and(|builder| builder.events.len() < 2)
            {
                return Err(AccountHistoryStorageError::DataInvariantViolation);
            }
            series_builders.push(OrganizedSeriesHistoryBuilder {
                series_id: row.series_id,
                series_name: display_name.clone(),
                latest_created_at: created_at.clone(),
                latest_public_id: event_public_id.clone(),
                events: Vec::new(),
            });
        }
        let builder = series_builders
            .last_mut()
            .ok_or(AccountHistoryStorageError::DataInvariantViolation)?;
        let expected_position = i64::try_from(builder.events.len())
            .map_err(|_| AccountHistoryStorageError::DataInvariantViolation)?;
        if position != expected_position || builder.series_name != display_name {
            return Err(AccountHistoryStorageError::DataInvariantViolation);
        }
        builder.latest_created_at = created_at;
        builder.latest_public_id = event_public_id.clone();
        builder.events.push(OrganizedEventHistoryItem {
            public_id: event_public_id,
            name,
            time_zone,
            decision: history_decision(row.local_date, row.local_time)?,
            response_count: u64::try_from(response_count)
                .map_err(|_| AccountHistoryStorageError::DataInvariantViolation)?,
        });
    }
    if series_builders
        .last()
        .is_some_and(|builder| builder.events.len() < 2)
    {
        return Err(AccountHistoryStorageError::DataInvariantViolation);
    }
    series_builders.sort_by(|left, right| {
        right
            .latest_created_at
            .cmp(&left.latest_created_at)
            .then_with(|| right.latest_public_id.cmp(&left.latest_public_id))
    });
    let organized_series = series_builders
        .into_iter()
        .map(|mut builder| {
            builder.events.reverse();
            OrganizedEventSeriesHistory {
                series_name: builder.series_name,
                events: builder.events,
            }
        })
        .collect();

    let organized_rows = sqlx::query_as::<_, OrganizedHistoryRow>(
        r#"
        SELECT
            e.public_id,
            e.name,
            e.time_zone,
            c.local_date,
            c.local_time,
            COUNT(r.id) AS response_count
        FROM events AS e
        LEFT JOIN responses AS r ON r.event_public_id = e.public_id
        LEFT JOIN event_decisions AS d ON d.event_public_id = e.public_id
        LEFT JOIN candidates AS c
          ON c.id = d.candidate_id
         AND c.event_public_id = d.event_public_id
        WHERE e.organizer_account_id = ?
          AND NOT EXISTS (
              SELECT 1
              FROM event_series_members AS m
              WHERE m.event_public_id = e.public_id
          )
        GROUP BY e.public_id, e.name, e.time_zone, e.created_at,
                 c.local_date, c.local_time
        ORDER BY e.created_at DESC, e.public_id DESC
        "#,
    )
    .bind(account.id)
    .fetch_all(transaction.as_mut())
    .await?;
    let mut organized_standalone = Vec::with_capacity(organized_rows.len());
    for row in organized_rows {
        organized_standalone.push(OrganizedEventHistoryItem {
            public_id: row.public_id,
            name: row.name,
            time_zone: row.time_zone,
            decision: history_decision(row.local_date, row.local_time)?,
            response_count: u64::try_from(row.response_count)
                .map_err(|_| AccountHistoryStorageError::DataInvariantViolation)?,
        });
    }

    let participated_rows = sqlx::query_as::<_, ParticipatedHistoryRow>(
        r#"
        SELECT
            e.public_id,
            e.name,
            e.time_zone,
            c.local_date,
            c.local_time
        FROM responses AS own_response
        JOIN events AS e ON e.public_id = own_response.event_public_id
        LEFT JOIN event_decisions AS d ON d.event_public_id = e.public_id
        LEFT JOIN candidates AS c
          ON c.id = d.candidate_id
         AND c.event_public_id = d.event_public_id
        WHERE own_response.respondent_account_id = ?
        GROUP BY e.public_id, e.name, e.time_zone,
                 c.local_date, c.local_time
        ORDER BY MAX(own_response.id) DESC, e.public_id DESC
        "#,
    )
    .bind(account.id)
    .fetch_all(transaction.as_mut())
    .await?;
    let mut participated = Vec::with_capacity(participated_rows.len());
    for row in participated_rows {
        participated.push(ParticipatedEventHistoryItem {
            public_id: row.public_id,
            name: row.name,
            time_zone: row.time_zone,
            decision: history_decision(row.local_date, row.local_time)?,
        });
    }

    transaction.commit().await?;
    Ok(AccountHistory {
        login_id: account.login_id,
        organized_standalone,
        organized_series,
        participated,
    })
}

/// Authorize and load one role-scoped event trace from a single read snapshot.
pub async fn find_account_event_trace_by_session(
    pool: &SqlitePool,
    token_hash: &[u8; 32],
    event_public_id: &str,
    now: i64,
) -> Result<AccountEventTrace, AccountEventTraceStorageError> {
    let account = resolve_account_session(pool, token_hash, now)
        .await?
        .ok_or(AccountEventTraceStorageError::Unauthenticated)?;
    let mut transaction = pool.begin().await?;

    let (event, relationship) =
        authorize_account_event_trace_snapshot(&mut transaction, event_public_id, account.id)
            .await?;
    let trace = load_account_event_trace_snapshot(
        &mut transaction,
        event_public_id,
        account.id,
        event,
        relationship,
    )
    .await?;
    transaction.commit().await?;
    Ok(trace)
}

async fn authorize_account_event_trace_snapshot(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    event_public_id: &str,
    account_id: i64,
) -> Result<(AccountEventTraceRow, AccountEventTraceRelationship), AccountEventTraceStorageError> {
    let event = sqlx::query_as::<_, AccountEventTraceRow>(
        r#"
        SELECT
            e.name,
            e.organizer_note,
            e.time_zone,
            CASE WHEN e.organizer_account_id = ? THEN 1 ELSE 0 END AS is_organizer,
            CASE WHEN EXISTS (
                SELECT 1
                FROM responses AS own_response
                WHERE own_response.event_public_id = e.public_id
                  AND own_response.respondent_account_id = ?
            ) THEN 1 ELSE 0 END AS is_participant
        FROM events AS e
        WHERE e.public_id = ?
          AND (
              e.organizer_account_id = ?
              OR EXISTS (
                  SELECT 1
                  FROM responses AS own_response
                  WHERE own_response.event_public_id = e.public_id
                    AND own_response.respondent_account_id = ?
              )
          )
        "#,
    )
    .bind(account_id)
    .bind(account_id)
    .bind(event_public_id)
    .bind(account_id)
    .bind(account_id)
    .fetch_optional(transaction.as_mut())
    .await?
    .ok_or(AccountEventTraceStorageError::NotFound)?;

    let is_organizer = match event.is_organizer {
        0 => false,
        1 => true,
        _ => return Err(AccountEventTraceStorageError::DataInvariantViolation),
    };
    let is_participant = match event.is_participant {
        0 => false,
        1 => true,
        _ => return Err(AccountEventTraceStorageError::DataInvariantViolation),
    };
    let relationship = match (is_organizer, is_participant) {
        (true, true) => AccountEventTraceRelationship::OrganizedAndParticipated,
        (true, false) => AccountEventTraceRelationship::Organized,
        (false, true) => AccountEventTraceRelationship::Participated,
        (false, false) => return Err(AccountEventTraceStorageError::DataInvariantViolation),
    };

    Ok((event, relationship))
}

async fn load_account_event_trace_snapshot(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    event_public_id: &str,
    account_id: i64,
    event: AccountEventTraceRow,
    relationship: AccountEventTraceRelationship,
) -> Result<AccountEventTrace, AccountEventTraceStorageError> {
    let is_organizer = matches!(
        relationship,
        AccountEventTraceRelationship::Organized
            | AccountEventTraceRelationship::OrganizedAndParticipated
    );
    let candidate_rows = sqlx::query_as::<_, CandidateRow>(
        r#"
        SELECT id, local_date, local_time
        FROM candidates
        WHERE event_public_id = ?
        ORDER BY position ASC
        "#,
    )
    .bind(event_public_id)
    .fetch_all(transaction.as_mut())
    .await?;
    let decision_row = sqlx::query_as::<_, StoredEventDecisionProjectionRow>(
        r#"
        SELECT d.candidate_id, c.local_date, c.local_time
        FROM event_decisions AS d
        LEFT JOIN candidates AS c
          ON c.id = d.candidate_id
         AND c.event_public_id = d.event_public_id
        WHERE d.event_public_id = ?
        "#,
    )
    .bind(event_public_id)
    .fetch_optional(transaction.as_mut())
    .await?;
    let decision = match decision_row {
        None => None,
        Some(stored) => {
            let (Some(local_date), Some(local_time)) = (stored.local_date, stored.local_time)
            else {
                return Err(AccountEventTraceStorageError::DataInvariantViolation);
            };
            if !candidate_rows
                .iter()
                .any(|candidate| candidate.id == stored.candidate_id)
            {
                return Err(AccountEventTraceStorageError::DataInvariantViolation);
            }
            Some(HistoryDecision {
                local_date,
                local_time,
            })
        }
    };

    let response_rows = sqlx::query_as::<_, AccountEventTraceResponseRow>(
        r#"
        SELECT
            id,
            respondent_name,
            respondent_comment,
            CASE WHEN respondent_account_id = ? THEN 1 ELSE 0 END AS is_current_account
        FROM responses
        WHERE event_public_id = ?
          AND (? = 1 OR respondent_account_id = ?)
        ORDER BY id ASC
        "#,
    )
    .bind(account_id)
    .bind(event_public_id)
    .bind(i64::from(is_organizer))
    .bind(account_id)
    .fetch_all(transaction.as_mut())
    .await?;
    let availability_rows = sqlx::query_as::<_, StoredMatrixAvailabilityRow>(
        r#"
        SELECT ra.response_id, ra.candidate_id, ra.availability
        FROM response_availabilities AS ra
        LEFT JOIN responses AS r
          ON r.id = ra.response_id
         AND r.event_public_id = ra.event_public_id
        WHERE ra.event_public_id = ?
          AND (? = 1 OR r.respondent_account_id = ?)
        "#,
    )
    .bind(event_public_id)
    .bind(i64::from(is_organizer))
    .bind(account_id)
    .fetch_all(transaction.as_mut())
    .await?;

    let trace = reconstruct_account_event_trace(
        event_public_id,
        event,
        relationship,
        candidate_rows,
        decision,
        response_rows,
        availability_rows,
    )?;
    Ok(trace)
}

fn reconstruct_account_event_trace(
    event_public_id: &str,
    event: AccountEventTraceRow,
    relationship: AccountEventTraceRelationship,
    candidate_rows: Vec<CandidateRow>,
    decision: Option<HistoryDecision>,
    response_rows: Vec<AccountEventTraceResponseRow>,
    availability_rows: Vec<StoredMatrixAvailabilityRow>,
) -> Result<AccountEventTrace, AccountEventTraceStorageError> {
    let candidate_count = candidate_rows.len();
    if candidate_count == 0 {
        return Err(AccountEventTraceStorageError::DataInvariantViolation);
    }
    let response_count = response_rows.len();
    let expected_cell_count = response_count
        .checked_mul(candidate_count)
        .ok_or(AccountEventTraceStorageError::DataInvariantViolation)?;
    if availability_rows.len() != expected_cell_count {
        return Err(AccountEventTraceStorageError::DataInvariantViolation);
    }

    let mut candidate_positions = HashMap::with_capacity(candidate_count);
    for (position, candidate) in candidate_rows.iter().enumerate() {
        if candidate_positions.insert(candidate.id, position).is_some() {
            return Err(AccountEventTraceStorageError::DataInvariantViolation);
        }
    }
    let mut response_positions = HashMap::with_capacity(response_count);
    for (position, response) in response_rows.iter().enumerate() {
        if response_positions.insert(response.id, position).is_some() {
            return Err(AccountEventTraceStorageError::DataInvariantViolation);
        }
    }

    let mut cells = Vec::new();
    cells
        .try_reserve_exact(expected_cell_count)
        .map_err(|_| AccountEventTraceStorageError::DataInvariantViolation)?;
    cells.resize(expected_cell_count, None);
    for stored in availability_rows {
        let response_position = response_positions
            .get(&stored.response_id)
            .copied()
            .ok_or(AccountEventTraceStorageError::DataInvariantViolation)?;
        let candidate_position = candidate_positions
            .get(&stored.candidate_id)
            .copied()
            .ok_or(AccountEventTraceStorageError::DataInvariantViolation)?;
        let cell_position = response_position
            .checked_mul(candidate_count)
            .and_then(|offset| offset.checked_add(candidate_position))
            .ok_or(AccountEventTraceStorageError::DataInvariantViolation)?;
        let cell = cells
            .get_mut(cell_position)
            .ok_or(AccountEventTraceStorageError::DataInvariantViolation)?;
        if cell.is_some() {
            return Err(AccountEventTraceStorageError::DataInvariantViolation);
        }
        *cell = Some(
            Availability::try_from(stored.availability.as_str())
                .map_err(|_| AccountEventTraceStorageError::DataInvariantViolation)?,
        );
    }

    let candidates = candidate_rows
        .into_iter()
        .map(|candidate| AccountEventTraceCandidate {
            local_date: candidate.local_date,
            local_time: candidate.local_time,
        })
        .collect();
    let mut response_cells = cells.into_iter();
    let mut responses = Vec::with_capacity(response_count);
    for response in response_rows {
        let is_current_account = match response.is_current_account {
            0 => false,
            1 => true,
            _ => return Err(AccountEventTraceStorageError::DataInvariantViolation),
        };
        if relationship == AccountEventTraceRelationship::Participated && !is_current_account {
            return Err(AccountEventTraceStorageError::DataInvariantViolation);
        }
        let availabilities = response_cells
            .by_ref()
            .take(candidate_count)
            .collect::<Option<Vec<_>>>()
            .ok_or(AccountEventTraceStorageError::DataInvariantViolation)?;
        responses.push(AccountEventTraceResponse {
            respondent_name: response.respondent_name,
            comment: response.respondent_comment,
            availabilities,
            is_current_account,
        });
    }
    if response_cells.next().is_some()
        || (relationship == AccountEventTraceRelationship::Participated && responses.is_empty())
    {
        return Err(AccountEventTraceStorageError::DataInvariantViolation);
    }

    Ok(AccountEventTrace {
        public_id: event_public_id.to_owned(),
        name: event.name,
        organizer_note: event.organizer_note,
        time_zone: event.time_zone,
        relationship,
        candidates,
        decision,
        responses,
    })
}

fn history_decision(
    local_date: Option<String>,
    local_time: Option<String>,
) -> Result<Option<HistoryDecision>, AccountHistoryStorageError> {
    match (local_date, local_time) {
        (None, None) => Ok(None),
        (Some(local_date), Some(local_time)) => Ok(Some(HistoryDecision {
            local_date,
            local_time,
        })),
        _ => Err(AccountHistoryStorageError::DataInvariantViolation),
    }
}

/// Load one organizer-authorized response summary from a consistent read snapshot.
pub async fn find_organizer_event_summary(
    pool: &SqlitePool,
    event_public_id: &str,
    organizer_capability_hash: &str,
) -> Result<OrganizerEventSummary, OrganizerSummaryStorageError> {
    let mut transaction = pool.begin().await?;
    let event = authorize_organizer_event_snapshot(
        &mut transaction,
        event_public_id,
        organizer_capability_hash,
    )
    .await?;
    let summary = load_organizer_summary_snapshot(&mut transaction, event_public_id, event).await?;
    transaction.commit().await?;
    Ok(summary)
}

async fn authorize_organizer_event_snapshot(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    event_public_id: &str,
    organizer_capability_hash: &str,
) -> Result<EventRow, OrganizerEventSnapshotError> {
    let event = sqlx::query_as::<_, EventRow>(
        r#"
        SELECT name, organizer_note, time_zone
        FROM events
        WHERE public_id = ? AND organizer_capability_hash = ?
        "#,
    )
    .bind(event_public_id)
    .bind(organizer_capability_hash)
    .fetch_optional(transaction.as_mut())
    .await?;
    let Some(event) = event else {
        return Err(OrganizerEventSnapshotError::NotFound);
    };
    Ok(event)
}

/// Load every response/candidate intersection from one organizer-authorized read snapshot.
pub async fn find_organizer_response_matrix(
    pool: &SqlitePool,
    event_public_id: &str,
    organizer_capability_hash: &str,
) -> Result<OrganizerResponseMatrix, OrganizerResponseMatrixStorageError> {
    let mut transaction = pool.begin().await?;
    let event = authorize_organizer_event_snapshot(
        &mut transaction,
        event_public_id,
        organizer_capability_hash,
    )
    .await?;
    let matrix = load_response_matrix_snapshot(&mut transaction, event_public_id, event).await?;
    transaction.commit().await?;
    Ok(matrix)
}

/// Load the complete matrix only when one response capability belongs to this event.
pub async fn find_participant_response_matrix(
    pool: &SqlitePool,
    event_public_id: &str,
    response_capability_hash: &str,
) -> Result<ParticipantResponseMatrix, ResponseMatrixStorageError> {
    let mut transaction = pool.begin().await?;
    let event = sqlx::query_as::<_, EventRow>(
        r#"
        SELECT e.name, e.organizer_note, e.time_zone
        FROM events e
        WHERE e.public_id = ?
          AND EXISTS (
              SELECT 1
              FROM responses r
              WHERE r.event_public_id = e.public_id
                AND r.response_capability_hash = ?
          )
        "#,
    )
    .bind(event_public_id)
    .bind(response_capability_hash)
    .fetch_optional(transaction.as_mut())
    .await?;
    let Some(event) = event else {
        return Err(ResponseMatrixStorageError::NotFound);
    };
    let matrix = load_response_matrix_snapshot(&mut transaction, event_public_id, event).await?;
    transaction.commit().await?;
    Ok(matrix)
}

async fn load_response_matrix_snapshot(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    event_public_id: &str,
    event: EventRow,
) -> Result<OrganizerResponseMatrix, OrganizerResponseMatrixStorageError> {
    let candidate_rows = sqlx::query_as::<_, CandidateRow>(
        r#"
        SELECT id, local_date, local_time
        FROM candidates
        WHERE event_public_id = ?
        ORDER BY position ASC
        "#,
    )
    .bind(event_public_id)
    .fetch_all(transaction.as_mut())
    .await?;

    let response_rows = sqlx::query_as::<_, StoredMatrixResponseRow>(
        r#"
        SELECT id, respondent_name
        FROM responses
        WHERE event_public_id = ?
        ORDER BY id ASC
        "#,
    )
    .bind(event_public_id)
    .fetch_all(transaction.as_mut())
    .await?;
    let availability_rows = sqlx::query_as::<_, StoredMatrixAvailabilityRow>(
        r#"
        SELECT response_id, candidate_id, availability
        FROM response_availabilities
        WHERE event_public_id = ?
        "#,
    )
    .bind(event_public_id)
    .fetch_all(transaction.as_mut())
    .await?;

    reconstruct_response_matrix(event, candidate_rows, response_rows, availability_rows)
}

fn reconstruct_response_matrix(
    event: EventRow,
    candidate_rows: Vec<CandidateRow>,
    response_rows: Vec<StoredMatrixResponseRow>,
    availability_rows: Vec<StoredMatrixAvailabilityRow>,
) -> Result<OrganizerResponseMatrix, OrganizerResponseMatrixStorageError> {
    let candidate_count = candidate_rows.len();
    if candidate_count == 0 {
        return Err(OrganizerResponseMatrixStorageError::DataInvariantViolation);
    }
    let response_count = response_rows.len();
    let expected_cell_count = response_count
        .checked_mul(candidate_count)
        .ok_or(OrganizerResponseMatrixStorageError::DataInvariantViolation)?;
    if availability_rows.len() != expected_cell_count {
        return Err(OrganizerResponseMatrixStorageError::DataInvariantViolation);
    }

    let mut candidate_positions = HashMap::with_capacity(candidate_count);
    for (position, candidate) in candidate_rows.iter().enumerate() {
        if candidate_positions.insert(candidate.id, position).is_some() {
            return Err(OrganizerResponseMatrixStorageError::DataInvariantViolation);
        }
    }
    let mut response_positions = HashMap::with_capacity(response_count);
    for (position, response) in response_rows.iter().enumerate() {
        if response_positions.insert(response.id, position).is_some() {
            return Err(OrganizerResponseMatrixStorageError::DataInvariantViolation);
        }
    }

    let mut cells = Vec::new();
    cells
        .try_reserve_exact(expected_cell_count)
        .map_err(|_| OrganizerResponseMatrixStorageError::DataInvariantViolation)?;
    cells.resize(expected_cell_count, None);
    for stored in availability_rows {
        let response_position = response_positions
            .get(&stored.response_id)
            .copied()
            .ok_or(OrganizerResponseMatrixStorageError::DataInvariantViolation)?;
        let candidate_position = candidate_positions
            .get(&stored.candidate_id)
            .copied()
            .ok_or(OrganizerResponseMatrixStorageError::DataInvariantViolation)?;
        let cell_position = response_position
            .checked_mul(candidate_count)
            .and_then(|offset| offset.checked_add(candidate_position))
            .ok_or(OrganizerResponseMatrixStorageError::DataInvariantViolation)?;
        let cell = cells
            .get_mut(cell_position)
            .ok_or(OrganizerResponseMatrixStorageError::DataInvariantViolation)?;
        if cell.is_some() {
            return Err(OrganizerResponseMatrixStorageError::DataInvariantViolation);
        }
        *cell = Some(
            Availability::try_from(stored.availability.as_str())
                .map_err(|_| OrganizerResponseMatrixStorageError::DataInvariantViolation)?,
        );
    }

    let candidates = candidate_rows
        .into_iter()
        .map(|candidate| ResponseMatrixCandidate {
            local_date: candidate.local_date,
            local_time: candidate.local_time,
        })
        .collect();
    let mut response_cells = cells.into_iter();
    let mut responses = Vec::with_capacity(response_count);
    for response in response_rows {
        let availabilities = response_cells
            .by_ref()
            .take(candidate_count)
            .collect::<Option<Vec<_>>>()
            .ok_or(OrganizerResponseMatrixStorageError::DataInvariantViolation)?;
        responses.push(ResponseMatrixRow {
            respondent_name: response.respondent_name,
            availabilities,
        });
    }
    if response_cells.next().is_some() {
        return Err(OrganizerResponseMatrixStorageError::DataInvariantViolation);
    }

    Ok(OrganizerResponseMatrix {
        name: event.name,
        time_zone: event.time_zone,
        candidates,
        responses,
    })
}

async fn load_organizer_summary_snapshot(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    event_public_id: &str,
    event: EventRow,
) -> Result<OrganizerEventSummary, OrganizerSummaryStorageError> {
    let response_count = checked_count(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM responses WHERE event_public_id = ?")
            .bind(event_public_id)
            .fetch_one(transaction.as_mut())
            .await?,
    )?;

    let candidate_rows = sqlx::query_as::<_, CandidateResponseSummaryRow>(
        r#"
        SELECT
            c.id,
            c.local_date,
            c.local_time,
            SUM(CASE WHEN ra.availability = 'available' THEN 1 ELSE 0 END)
                AS available_count,
            SUM(CASE WHEN ra.availability = 'maybe' THEN 1 ELSE 0 END)
                AS maybe_count,
            SUM(CASE WHEN ra.availability = 'unavailable' THEN 1 ELSE 0 END)
                AS unavailable_count
        FROM candidates AS c
        LEFT JOIN response_availabilities AS ra
          ON ra.candidate_id = c.id
         AND ra.event_public_id = c.event_public_id
        WHERE c.event_public_id = ?
        GROUP BY c.id, c.position, c.local_date, c.local_time
        ORDER BY c.position ASC
        "#,
    )
    .bind(event_public_id)
    .fetch_all(transaction.as_mut())
    .await?;

    if candidate_rows.is_empty() {
        return Err(OrganizerSummaryStorageError::DataInvariantViolation);
    }
    let mut candidates = Vec::with_capacity(candidate_rows.len());
    for candidate in candidate_rows {
        let available_count = checked_count(candidate.available_count)?;
        let maybe_count = checked_count(candidate.maybe_count)?;
        let unavailable_count = checked_count(candidate.unavailable_count)?;
        let candidate_response_count = available_count
            .checked_add(maybe_count)
            .and_then(|count| count.checked_add(unavailable_count));
        if candidate_response_count != Some(response_count) {
            return Err(OrganizerSummaryStorageError::DataInvariantViolation);
        }
        candidates.push(CandidateResponseSummary {
            id: candidate.id,
            local_date: candidate.local_date,
            local_time: candidate.local_time,
            available_count,
            maybe_count,
            unavailable_count,
            fact: None,
        });
    }

    let comment_count = checked_count(
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM responses
            WHERE event_public_id = ? AND respondent_comment IS NOT NULL
            "#,
        )
        .bind(event_public_id)
        .fetch_one(transaction.as_mut())
        .await?,
    )?;
    let comment_previews = sqlx::query_as::<_, ResponseCommentPreviewRow>(
        r#"
        SELECT respondent_name, respondent_comment AS comment
        FROM responses
        WHERE event_public_id = ? AND respondent_comment IS NOT NULL
        ORDER BY id DESC
        LIMIT 3
        "#,
    )
    .bind(event_public_id)
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|preview| ResponseCommentPreview {
        respondent_name: preview.respondent_name,
        comment: preview.comment,
    })
    .collect::<Vec<_>>();

    let preview_count = u64::try_from(comment_previews.len())
        .map_err(|_| OrganizerSummaryStorageError::DataInvariantViolation)?;
    if comment_count > response_count || preview_count > comment_count {
        return Err(OrganizerSummaryStorageError::DataInvariantViolation);
    }

    let decision_row = sqlx::query_as::<_, StoredEventDecisionProjectionRow>(
        r#"
        SELECT d.candidate_id, c.local_date, c.local_time
        FROM event_decisions AS d
        LEFT JOIN candidates AS c
          ON c.id = d.candidate_id
         AND c.event_public_id = d.event_public_id
        WHERE d.event_public_id = ?
        "#,
    )
    .bind(event_public_id)
    .fetch_optional(transaction.as_mut())
    .await?;
    let decision = match decision_row {
        None => None,
        Some(stored) => {
            let (Some(local_date), Some(local_time)) = (stored.local_date, stored.local_time)
            else {
                return Err(OrganizerSummaryStorageError::DataInvariantViolation);
            };
            Some(OrganizerEventDecision {
                candidate_id: stored.candidate_id,
                local_date,
                local_time,
            })
        }
    };

    derive_candidate_summary_facts(response_count, &mut candidates);

    Ok(OrganizerEventSummary {
        public_id: event_public_id.to_owned(),
        name: event.name,
        organizer_note: event.organizer_note,
        time_zone: event.time_zone,
        response_count,
        candidates,
        comment_count,
        comment_previews,
        decision,
    })
}

fn checked_count(value: i64) -> Result<u64, OrganizerSummaryStorageError> {
    u64::try_from(value).map_err(|_| OrganizerSummaryStorageError::DataInvariantViolation)
}

/// Save one complete anonymous response, or replay an identical committed write.
pub async fn record_availability_response(
    pool: &SqlitePool,
    event_public_id: &str,
    response_capability_hash: &str,
    response: &PreparedAvailabilityResponse,
) -> Result<ResponseWriteOutcome, ResponseStorageError> {
    Ok(record_availability_response_for_session(
        pool,
        event_public_id,
        response_capability_hash,
        response,
        None,
        0,
    )
    .await?
    .value)
}

/// Save an answer and link its account only on the first successful write transaction.
pub async fn record_availability_response_for_session(
    pool: &SqlitePool,
    event_public_id: &str,
    response_capability_hash: &str,
    response: &PreparedAvailabilityResponse,
    session_token_hash: Option<&[u8; 32]>,
    now: i64,
) -> Result<SessionWrite<ResponseWriteOutcome>, ResponseStorageError> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let (respondent_account_id, session_status) =
        resolve_write_session_in_transaction(&mut transaction, session_token_hash, now).await?;

    let event_exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM events WHERE public_id = ?")
        .bind(event_public_id)
        .fetch_optional(transaction.as_mut())
        .await?;
    if event_exists.is_none() {
        return Err(ResponseStorageError::EventNotFound);
    }

    let mut event_candidate_ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM candidates WHERE event_public_id = ? ORDER BY id")
            .bind(event_public_id)
            .fetch_all(transaction.as_mut())
            .await?;
    let mut requested_candidate_ids = response
        .availabilities
        .iter()
        .map(|choice| choice.candidate_id)
        .collect::<Vec<_>>();
    event_candidate_ids.sort_unstable();
    requested_candidate_ids.sort_unstable();
    requested_candidate_ids.dedup();
    if requested_candidate_ids.len() != response.availabilities.len()
        || requested_candidate_ids != event_candidate_ids
    {
        return Err(ResponseStorageError::CandidateSetMismatch);
    }

    let existing = sqlx::query_as::<_, StoredResponseRow>(
        r#"
        SELECT id, event_public_id, respondent_name
        FROM responses
        WHERE response_capability_hash = ?
        "#,
    )
    .bind(response_capability_hash)
    .fetch_optional(transaction.as_mut())
    .await?;
    if let Some(existing) = existing {
        let stored = sqlx::query_as::<_, StoredAvailabilityRow>(
            r#"
            SELECT candidate_id, availability
            FROM response_availabilities
            WHERE response_id = ?
            ORDER BY candidate_id
            "#,
        )
        .bind(existing.id)
        .fetch_all(transaction.as_mut())
        .await?;
        let mut requested = response
            .availabilities
            .iter()
            .map(|choice| (choice.candidate_id, choice.availability.storage_value()))
            .collect::<Vec<_>>();
        requested.sort_unstable_by_key(|(candidate_id, _)| *candidate_id);
        let same_availabilities = stored.len() == requested.len()
            && stored
                .iter()
                .zip(requested.iter())
                .all(|(stored, (candidate_id, availability))| {
                    stored.candidate_id == *candidate_id && stored.availability == *availability
                });

        if existing.event_public_id == event_public_id
            && existing.respondent_name == response.respondent_name
            && same_availabilities
        {
            transaction.commit().await?;
            return Ok(SessionWrite {
                value: ResponseWriteOutcome::AlreadyRecorded,
                session_status,
            });
        }

        return Err(ResponseStorageError::CapabilityConflict);
    }

    let event_decided: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM event_decisions WHERE event_public_id = ?")
            .bind(event_public_id)
            .fetch_optional(transaction.as_mut())
            .await?;
    if event_decided.is_some() {
        return Err(ResponseStorageError::EventDecided);
    }

    let inserted = sqlx::query(
        r#"
        INSERT INTO responses (
            event_public_id,
            respondent_name,
            response_capability_hash,
            respondent_account_id
        ) VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(event_public_id)
    .bind(&response.respondent_name)
    .bind(response_capability_hash)
    .bind(respondent_account_id)
    .execute(transaction.as_mut())
    .await?;

    let response_id = inserted.last_insert_rowid();
    for choice in &response.availabilities {
        sqlx::query(
            r#"
            INSERT INTO response_availabilities (
                response_id,
                candidate_id,
                event_public_id,
                availability
            ) VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(response_id)
        .bind(choice.candidate_id)
        .bind(event_public_id)
        .bind(choice.availability.storage_value())
        .execute(transaction.as_mut())
        .await?;
    }

    transaction.commit().await?;
    Ok(SessionWrite {
        value: ResponseWriteOutcome::Created,
        session_status,
    })
}

/// Add one comment to the response authorized by an event and capability hash.
pub async fn record_response_comment(
    pool: &SqlitePool,
    event_public_id: &str,
    response_capability_hash: &str,
    comment: &str,
) -> Result<ResponseCommentWriteOutcome, ResponseCommentStorageError> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let response = sqlx::query_as::<_, StoredResponseCommentRow>(
        r#"
        SELECT id, respondent_comment
        FROM responses
        WHERE event_public_id = ? AND response_capability_hash = ?
        "#,
    )
    .bind(event_public_id)
    .bind(response_capability_hash)
    .fetch_optional(transaction.as_mut())
    .await?;

    let Some(response) = response else {
        return Err(ResponseCommentStorageError::ResponseNotFound);
    };

    match response.respondent_comment.as_deref() {
        None => {
            sqlx::query(
                r#"
                UPDATE responses
                SET respondent_comment = ?
                WHERE id = ? AND respondent_comment IS NULL
                "#,
            )
            .bind(comment)
            .bind(response.id)
            .execute(transaction.as_mut())
            .await?;
            transaction.commit().await?;
            Ok(ResponseCommentWriteOutcome::Created)
        }
        Some(stored) if stored == comment => {
            transaction.commit().await?;
            Ok(ResponseCommentWriteOutcome::AlreadyRecorded)
        }
        Some(_) => Err(ResponseCommentStorageError::CommentConflict),
    }
}

/// Commit the first organizer-selected candidate, or replay the same decision.
pub async fn record_event_decision(
    pool: &SqlitePool,
    event_public_id: &str,
    organizer_capability_hash: &str,
    candidate_id: i64,
) -> Result<(EventDecisionWriteOutcome, OrganizerEventDecision), OrganizerDecisionStorageError> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;

    let authorized: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT 1
        FROM events
        WHERE public_id = ? AND organizer_capability_hash = ?
        "#,
    )
    .bind(event_public_id)
    .bind(organizer_capability_hash)
    .fetch_optional(transaction.as_mut())
    .await?;
    if authorized.is_none() {
        return Err(OrganizerDecisionStorageError::NotFound);
    }

    let candidate = sqlx::query_as::<_, CandidateRow>(
        r#"
        SELECT id, local_date, local_time
        FROM candidates
        WHERE id = ? AND event_public_id = ?
        "#,
    )
    .bind(candidate_id)
    .bind(event_public_id)
    .fetch_optional(transaction.as_mut())
    .await?;
    let Some(candidate) = candidate else {
        return Err(OrganizerDecisionStorageError::CandidateMismatch);
    };

    let existing_candidate_id: Option<i64> =
        sqlx::query_scalar("SELECT candidate_id FROM event_decisions WHERE event_public_id = ?")
            .bind(event_public_id)
            .fetch_optional(transaction.as_mut())
            .await?;
    if let Some(existing_candidate_id) = existing_candidate_id {
        if existing_candidate_id != candidate_id {
            return Err(OrganizerDecisionStorageError::Conflict);
        }

        transaction.commit().await?;
        return Ok((
            EventDecisionWriteOutcome::AlreadyDecided,
            OrganizerEventDecision {
                candidate_id: candidate.id,
                local_date: candidate.local_date,
                local_time: candidate.local_time,
            },
        ));
    }

    sqlx::query("INSERT INTO event_decisions (event_public_id, candidate_id) VALUES (?, ?)")
        .bind(event_public_id)
        .bind(candidate_id)
        .execute(transaction.as_mut())
        .await?;
    transaction.commit().await?;

    Ok((
        EventDecisionWriteOutcome::Created,
        OrganizerEventDecision {
            candidate_id: candidate.id,
            local_date: candidate.local_date,
            local_time: candidate.local_time,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Availability, CandidateAvailabilityInput, CandidateInput};

    #[tokio::test]
    async fn account_event_trace_keeps_the_snapshot_established_by_authorization() {
        let database_path = std::env::temp_dir().join(format!(
            "tsunoru-account-trace-snapshot-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        let pool = open_file(&database_path)
            .await
            .expect("open file-backed WAL fixture");
        let token_hash = [7_u8; 32];
        let now = 1_800_000_000;
        let account = create_account_with_session(
            &pool,
            "snapshot-reader",
            "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA",
            &token_hash,
            now,
        )
        .await
        .expect("create snapshot account");
        let event = create_event_record_for_session(
            &pool,
            "account-trace-snapshot-event",
            &"e".repeat(64),
            &NewEventInput {
                name: "同じsnapshotの履歴詳細".to_owned(),
                organizer_note: None,
                time_zone: "Asia/Tokyo".to_owned(),
                candidates: vec![CandidateInput {
                    local_date: "2027-01-15".to_owned(),
                    local_time: "19:00".to_owned(),
                }],
            },
            Some(&token_hash),
            now,
        )
        .await
        .expect("persist account-linked event")
        .value;

        let mut reader = pool.begin().await.expect("begin deferred reader");
        let (authorized_event, relationship) =
            authorize_account_event_trace_snapshot(&mut reader, &event.public_id, account.id)
                .await
                .expect("authorize and establish the trace reader snapshot");

        record_availability_response_for_session(
            &pool,
            &event.public_id,
            &"f".repeat(64),
            &PreparedAvailabilityResponse {
                respondent_name: "後から回答した人".to_owned(),
                availabilities: vec![CandidateAvailabilityInput {
                    candidate_id: event.candidates[0].id,
                    availability: Availability::Available,
                }],
            },
            Some(&token_hash),
            now + 1,
        )
        .await
        .expect("commit a response from another WAL connection");

        let snapshot = load_account_event_trace_snapshot(
            &mut reader,
            &event.public_id,
            account.id,
            authorized_event,
            relationship,
        )
        .await
        .expect("finish the trace from the established snapshot");
        assert!(snapshot.responses.is_empty());
        reader.commit().await.expect("finish reader transaction");

        let current =
            find_account_event_trace_by_session(&pool, &token_hash, &event.public_id, now + 2)
                .await
                .expect("a new transaction sees the committed response");
        assert_eq!(current.responses.len(), 1);
        assert_eq!(current.responses[0].respondent_name, "後から回答した人");

        pool.close().await;
        for path in [
            database_path.clone(),
            database_path.with_extension("sqlite3-shm"),
            database_path.with_extension("sqlite3-wal"),
        ] {
            if path.exists() {
                std::fs::remove_file(path).expect("remove isolated trace test database");
            }
        }
    }

    #[tokio::test]
    async fn organizer_summary_keeps_the_snapshot_established_by_authorization() {
        let database_path = std::env::temp_dir().join(format!(
            "tsunoru-organizer-summary-snapshot-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        let pool = open_file(&database_path)
            .await
            .expect("open file-backed WAL fixture");
        let organizer_hash = "a".repeat(64);
        let event = create_event_record(
            &pool,
            "snapshot-event",
            &organizer_hash,
            &NewEventInput {
                name: "同じsnapshotの会".to_owned(),
                organizer_note: None,
                time_zone: "Asia/Tokyo".to_owned(),
                candidates: vec![CandidateInput {
                    local_date: "2026-09-18".to_owned(),
                    local_time: "19:00".to_owned(),
                }],
            },
        )
        .await
        .expect("persist fixture event");

        let mut reader = pool.begin().await.expect("begin deferred reader");
        let authorized_event =
            authorize_organizer_event_snapshot(&mut reader, &event.public_id, &organizer_hash)
                .await
                .expect("authorize and establish the reader snapshot");

        record_availability_response(
            &pool,
            &event.public_id,
            &"b".repeat(64),
            &PreparedAvailabilityResponse {
                respondent_name: "後から回答した人".to_owned(),
                availabilities: vec![CandidateAvailabilityInput {
                    candidate_id: event.candidates[0].id,
                    availability: Availability::Available,
                }],
            },
        )
        .await
        .expect("commit a response from another WAL connection");

        let snapshot =
            load_organizer_summary_snapshot(&mut reader, &event.public_id, authorized_event)
                .await
                .expect("finish the summary from the established snapshot");
        assert_eq!(snapshot.response_count, 0);
        assert_eq!(snapshot.candidates[0].available_count, 0);
        reader.commit().await.expect("finish reader transaction");

        let current = find_organizer_event_summary(&pool, &event.public_id, &organizer_hash)
            .await
            .expect("a new transaction sees the committed response");
        assert_eq!(current.response_count, 1);
        assert_eq!(current.candidates[0].available_count, 1);

        pool.close().await;
        for path in [
            database_path.clone(),
            database_path.with_extension("sqlite3-shm"),
            database_path.with_extension("sqlite3-wal"),
        ] {
            if path.exists() {
                std::fs::remove_file(path).expect("remove isolated test database");
            }
        }
    }

    #[tokio::test]
    async fn organizer_response_matrix_keeps_the_snapshot_established_by_authorization() {
        let database_path = std::env::temp_dir().join(format!(
            "tsunoru-organizer-matrix-snapshot-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        let pool = open_file(&database_path)
            .await
            .expect("open file-backed WAL fixture");
        let organizer_hash = "c".repeat(64);
        let event = create_event_record(
            &pool,
            "matrix-snapshot-event",
            &organizer_hash,
            &NewEventInput {
                name: "同じsnapshotの集計表".to_owned(),
                organizer_note: None,
                time_zone: "Asia/Tokyo".to_owned(),
                candidates: vec![CandidateInput {
                    local_date: "2026-09-18".to_owned(),
                    local_time: "19:00".to_owned(),
                }],
            },
        )
        .await
        .expect("persist matrix fixture event");

        let mut reader = pool.begin().await.expect("begin deferred reader");
        let authorized_event =
            authorize_organizer_event_snapshot(&mut reader, &event.public_id, &organizer_hash)
                .await
                .expect("authorize and establish the matrix reader snapshot");

        record_availability_response(
            &pool,
            &event.public_id,
            &"d".repeat(64),
            &PreparedAvailabilityResponse {
                respondent_name: "後から回答した人".to_owned(),
                availabilities: vec![CandidateAvailabilityInput {
                    candidate_id: event.candidates[0].id,
                    availability: Availability::Available,
                }],
            },
        )
        .await
        .expect("commit a response from another WAL connection");

        let snapshot =
            load_response_matrix_snapshot(&mut reader, &event.public_id, authorized_event)
                .await
                .expect("finish the matrix from the established snapshot");
        assert!(snapshot.responses.is_empty());
        reader.commit().await.expect("finish reader transaction");

        let current = find_organizer_response_matrix(&pool, &event.public_id, &organizer_hash)
            .await
            .expect("a new transaction sees the committed response");
        assert_eq!(current.responses.len(), 1);
        assert_eq!(current.responses[0].respondent_name, "後から回答した人");

        pool.close().await;
        for path in [
            database_path.clone(),
            database_path.with_extension("sqlite3-shm"),
            database_path.with_extension("sqlite3-wal"),
        ] {
            if path.exists() {
                std::fs::remove_file(path).expect("remove isolated matrix test database");
            }
        }
    }
}
