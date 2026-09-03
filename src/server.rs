//! Typed server functions shared with the browser as generated clients.

use crate::domain::{
    AccountEventContinuationState, AccountEventTraceInput, AccountEventTraceState,
    AccountHistoryState, AccountLoginInput, AccountRegistrationInput, CreatedEvent, CurrentAccount,
    EventContinuationCreateInput, EventContinuationPlanInput, NewAvailabilityResponseInput,
    NewEventInput, NewResponseCommentInput, OrganizerDecisionInput, OrganizerEventDecision,
    OrganizerEventSummary, OrganizerResponseMatrix, OrganizerSummaryInput,
    ParticipantResponseMatrix, PublicEvent,
};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::domain::AccountAuthErrors;
#[cfg(feature = "server")]
use std::{fmt, sync::OnceLock};

#[cfg(feature = "server")]
use dioxus::server::axum::extract::{Path, rejection::PathRejection};

#[cfg(feature = "server")]
static LOGIN_RATE_LIMITER: OnceLock<crate::auth::LoginRateLimiter> = OnceLock::new();
#[cfg(feature = "server")]
static REGISTRATION_RATE_LIMITER: OnceLock<crate::auth::LoginRateLimiter> = OnceLock::new();

#[cfg(feature = "server")]
fn login_rate_limiter() -> &'static crate::auth::LoginRateLimiter {
    LOGIN_RATE_LIMITER.get_or_init(|| crate::auth::LoginRateLimiter::new(5, 15 * 60))
}

#[cfg(feature = "server")]
fn registration_rate_limiter() -> &'static crate::auth::LoginRateLimiter {
    REGISTRATION_RATE_LIMITER.get_or_init(|| crate::auth::LoginRateLimiter::new(100, 15 * 60))
}

/// A newly issued raw bearer exists only until it is written to an HttpOnly cookie.
#[cfg(feature = "server")]
#[derive(PartialEq, Eq)]
pub struct IssuedAccountSession {
    pub account: CurrentAccount,
    pub session_token: String,
}

#[cfg(feature = "server")]
impl fmt::Debug for IssuedAccountSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IssuedAccountSession")
            .field("account", &self.account)
            .field("session_token", &"[REDACTED]")
            .finish()
    }
}

/// Expected registration failures with no password or account-existence details.
#[cfg(feature = "server")]
#[derive(Debug)]
pub enum AccountRegistrationError {
    Validation(AccountAuthErrors),
    Unavailable,
    RateLimited(i64),
    Storage(anyhow::Error),
}

#[cfg(feature = "server")]
impl fmt::Display for AccountRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(errors) => write!(formatter, "{errors}"),
            Self::Unavailable => write!(formatter, "アカウントを作成できませんでした。"),
            Self::RateLimited(_) => write!(
                formatter,
                "アカウント作成をしばらく試せません。時間を置いてお試しください。"
            ),
            Self::Storage(_) => write!(formatter, "アカウントを作成できませんでした。"),
        }
    }
}

#[cfg(feature = "server")]
impl std::error::Error for AccountRegistrationError {}

/// Expected login failures with one response for unknown IDs and wrong passwords.
#[cfg(feature = "server")]
#[derive(Debug)]
pub enum AccountLoginError {
    Validation(AccountAuthErrors),
    InvalidCredentials,
    RateLimited(i64),
    Storage(anyhow::Error),
}

#[cfg(feature = "server")]
impl fmt::Display for AccountLoginError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(errors) => write!(formatter, "{errors}"),
            Self::InvalidCredentials => {
                write!(formatter, "ログインIDまたはpasswordを確認してください。")
            }
            Self::RateLimited(_) => write!(
                formatter,
                "ログインをしばらく試せません。時間を置いてお試しください。"
            ),
            Self::Storage(_) => write!(formatter, "ログインできませんでした。"),
        }
    }
}

#[cfg(feature = "server")]
impl std::error::Error for AccountLoginError {}

/// Validate, hash, and atomically store one account and initial session.
#[cfg(feature = "server")]
pub async fn persist_account_registration(
    pool: &sqlx::SqlitePool,
    input: AccountRegistrationInput,
    session_token: String,
    now: i64,
) -> Result<IssuedAccountSession, AccountRegistrationError> {
    persist_account_registration_replacing_session(pool, input, session_token, None, now).await
}

/// Validate, hash, and create an account while replacing this browser's old session atomically.
#[cfg(feature = "server")]
pub async fn persist_account_registration_replacing_session(
    pool: &sqlx::SqlitePool,
    input: AccountRegistrationInput,
    session_token: String,
    previous_token_hash: Option<&[u8; 32]>,
    now: i64,
) -> Result<IssuedAccountSession, AccountRegistrationError> {
    use crate::{
        auth::{hash_password, hash_session_token},
        storage::{AccountStorageError, create_account_with_session_replacing},
    };

    let prepared = input
        .prepare()
        .map_err(AccountRegistrationError::Validation)?;
    registration_rate_limiter()
        .record_attempt("account-registration", now)
        .map_err(AccountRegistrationError::RateLimited)?;
    let password_hash_phc = hash_password(&prepared.password)
        .await
        .map_err(AccountRegistrationError::Storage)?;
    let token_hash = hash_session_token(&session_token).ok_or_else(|| {
        AccountRegistrationError::Storage(anyhow::anyhow!("generated session token was invalid"))
    })?;
    let account = create_account_with_session_replacing(
        pool,
        &prepared.login_id,
        &password_hash_phc,
        &token_hash,
        previous_token_hash,
        now,
    )
    .await
    .map_err(|error| match error {
        AccountStorageError::LoginIdTaken => AccountRegistrationError::Unavailable,
        other => AccountRegistrationError::Storage(anyhow::Error::new(other)),
    })?;
    Ok(IssuedAccountSession {
        account: CurrentAccount {
            login_id: account.login_id,
        },
        session_token,
    })
}

/// Verify one login through the same password path for known and unknown IDs.
#[cfg(feature = "server")]
pub async fn persist_account_login(
    pool: &sqlx::SqlitePool,
    input: AccountLoginInput,
    session_token: String,
    now: i64,
) -> Result<IssuedAccountSession, AccountLoginError> {
    persist_account_login_replacing_session(pool, input, session_token, None, now).await
}

/// Verify one login and atomically rotate the session presented by this browser.
#[cfg(feature = "server")]
pub async fn persist_account_login_replacing_session(
    pool: &sqlx::SqlitePool,
    input: AccountLoginInput,
    session_token: String,
    previous_token_hash: Option<&[u8; 32]>,
    now: i64,
) -> Result<IssuedAccountSession, AccountLoginError> {
    use crate::{
        auth::{dummy_password_hash, hash_session_token, verify_password},
        storage::{create_account_session_replacing, find_account_credentials},
    };

    let prepared = input.prepare().map_err(AccountLoginError::Validation)?;
    login_rate_limiter()
        .record_attempt(&prepared.login_id, now)
        .map_err(AccountLoginError::RateLimited)?;
    let dummy = dummy_password_hash()
        .await
        .map_err(AccountLoginError::Storage)?;
    let stored = find_account_credentials(pool, &prepared.login_id)
        .await
        .map_err(|error| AccountLoginError::Storage(anyhow::Error::new(error)))?;
    let candidate_hash = stored
        .as_ref()
        .map_or(dummy, |account| account.password_hash_phc.as_str());
    let verified = verify_password(&prepared.password, candidate_hash)
        .await
        .map_err(AccountLoginError::Storage)?;
    let Some(account) = stored.filter(|_| verified) else {
        return Err(AccountLoginError::InvalidCredentials);
    };
    login_rate_limiter().record_success(&prepared.login_id);

    let token_hash = hash_session_token(&session_token).ok_or_else(|| {
        AccountLoginError::Storage(anyhow::anyhow!("generated session token was invalid"))
    })?;
    create_account_session_replacing(pool, account.id, &token_hash, previous_token_hash, now)
        .await
        .map_err(|error| AccountLoginError::Storage(anyhow::Error::new(error)))?;
    Ok(IssuedAccountSession {
        account: CurrentAccount {
            login_id: account.login_id,
        },
        session_token,
    })
}

/// Register an optional account and begin one HttpOnly-cookie session.
#[post("/api/auth/register")]
pub async fn register_account(input: AccountRegistrationInput) -> Result<CurrentAccount> {
    #[cfg(feature = "server")]
    {
        use crate::{auth::issue_session_token, storage::database_pool};

        add_private_response_headers();
        let session = request_session_context()
            .map_err(|_| ServerFnError::new("アカウントの安全な接続を確認できませんでした。"))?;
        let issued = persist_account_registration_replacing_session(
            database_pool(),
            input,
            issue_session_token(),
            session.presented.digest(),
            chrono::Utc::now().timestamp(),
        )
        .await
        .map_err(|error| match error {
            AccountRegistrationError::Validation(errors) => ServerFnError::ServerError {
                message: errors.to_string(),
                code: 422,
                details: None,
            },
            AccountRegistrationError::Unavailable => ServerFnError::ServerError {
                message: "アカウントを作成できませんでした。".to_owned(),
                code: 409,
                details: None,
            },
            AccountRegistrationError::RateLimited(retry_after) => {
                add_retry_after_header(retry_after);
                ServerFnError::ServerError {
                    message: "アカウント作成をしばらく試せません。時間を置いてお試しください。"
                        .to_owned(),
                    code: 429,
                    details: None,
                }
            }
            AccountRegistrationError::Storage(_) => {
                eprintln!("failed to persist account registration");
                ServerFnError::new("アカウントを作成できませんでした。")
            }
        })?;
        set_session_cookie(&session.policy, &issued.session_token)?;
        Ok(issued.account)
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Verify credentials through one generic failure boundary and rotate the browser session.
#[post("/api/auth/login")]
pub async fn login_account(input: AccountLoginInput) -> Result<CurrentAccount> {
    #[cfg(feature = "server")]
    {
        use crate::{auth::issue_session_token, storage::database_pool};

        add_private_response_headers();
        let session = request_session_context()
            .map_err(|_| ServerFnError::new("ログインの安全な接続を確認できませんでした。"))?;
        let issued = persist_account_login_replacing_session(
            database_pool(),
            input,
            issue_session_token(),
            session.presented.digest(),
            chrono::Utc::now().timestamp(),
        )
        .await
        .map_err(|error| match error {
            AccountLoginError::Validation(errors) => ServerFnError::ServerError {
                message: errors.to_string(),
                code: 422,
                details: None,
            },
            AccountLoginError::InvalidCredentials => ServerFnError::ServerError {
                message: "ログインIDまたはpasswordを確認してください。".to_owned(),
                code: 401,
                details: None,
            },
            AccountLoginError::RateLimited(retry_after) => {
                add_retry_after_header(retry_after);
                ServerFnError::ServerError {
                    message: "ログインをしばらく試せません。時間を置いてお試しください。"
                        .to_owned(),
                    code: 429,
                    details: None,
                }
            }
            AccountLoginError::Storage(_) => {
                eprintln!("failed to persist account login");
                ServerFnError::new("ログインできませんでした。")
            }
        })?;
        set_session_cookie(&session.policy, &issued.session_token)?;
        Ok(issued.account)
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Invalidate only the account session, preserving anonymous organizer capabilities.
#[post("/api/auth/logout")]
pub async fn logout_account() -> Result<()> {
    #[cfg(feature = "server")]
    {
        use crate::storage::{database_pool, delete_account_session};

        add_private_response_headers();
        let session = request_session_context()
            .map_err(|_| ServerFnError::new("ログアウトの接続を確認できませんでした。"))?;
        if let Some(token_hash) = session.presented.digest() {
            delete_account_session(database_pool(), token_hash)
                .await
                .map_err(|error| {
                    eprintln!("failed to invalidate account session: {error}");
                    ServerFnError::new("ログアウトできませんでした。")
                })?;
        }
        clear_session_cookie(&session.policy)?;
        Ok(())
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Return private account history without turning a missing cookie into an error page.
#[get("/api/account/history")]
pub async fn get_account_history() -> Result<AccountHistoryState> {
    #[cfg(feature = "server")]
    {
        use crate::storage::{
            AccountHistoryStorageError, database_pool, find_account_history_by_session,
        };

        add_private_response_headers();
        let session = request_session_context()
            .map_err(|_| ServerFnError::new("履歴の安全な接続を確認できませんでした。"))?;
        let token_hash = match session.presented {
            crate::auth::PresentedSession::Absent => return Ok(AccountHistoryState::Guest),
            crate::auth::PresentedSession::Invalid => return Ok(AccountHistoryState::Expired),
            crate::auth::PresentedSession::Digest(token_hash) => token_hash,
        };
        match find_account_history_by_session(
            database_pool(),
            &token_hash,
            chrono::Utc::now().timestamp(),
        )
        .await
        {
            Ok(history) => Ok(AccountHistoryState::Authenticated(history)),
            Err(AccountHistoryStorageError::Unauthenticated) => Ok(AccountHistoryState::Expired),
            Err(AccountHistoryStorageError::DataInvariantViolation) => {
                eprintln!("account history projection violated a data invariant");
                Err(ServerFnError::new("履歴を読み込めませんでした。").into())
            }
            Err(AccountHistoryStorageError::Database(error)) => {
                eprintln!("failed to read account history: {error}");
                Err(ServerFnError::new("履歴を読み込めませんでした。").into())
            }
        }
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Return one account-authorized event trace without exposing or restoring capabilities.
#[post("/api/account/history/event-detail")]
pub async fn get_account_event_trace(
    input: AccountEventTraceInput,
) -> std::result::Result<AccountEventTraceState, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::storage::{
            AccountEventTraceStorageError, database_pool, find_account_event_trace_by_session,
        };

        add_private_response_headers();
        let input =
            input
                .normalized_and_validated()
                .map_err(|errors| ServerFnError::ServerError {
                    message: errors.to_string(),
                    code: 422,
                    details: None,
                })?;
        let session = request_session_context()
            .map_err(|_| ServerFnError::new("記録の安全な接続を確認できませんでした。"))?;
        let token_hash = match session.presented {
            crate::auth::PresentedSession::Absent => return Ok(AccountEventTraceState::Guest),
            crate::auth::PresentedSession::Invalid => return Ok(AccountEventTraceState::Expired),
            crate::auth::PresentedSession::Digest(token_hash) => token_hash,
        };

        match find_account_event_trace_by_session(
            database_pool(),
            &token_hash,
            &input.event_public_id,
            chrono::Utc::now().timestamp(),
        )
        .await
        {
            Ok(trace) => Ok(AccountEventTraceState::Authenticated(trace)),
            Err(AccountEventTraceStorageError::Unauthenticated) => {
                Ok(AccountEventTraceState::Expired)
            }
            Err(AccountEventTraceStorageError::NotFound) => Err(ServerFnError::ServerError {
                message: "記録が見つかりません。".to_owned(),
                code: 404,
                details: None,
            }),
            Err(AccountEventTraceStorageError::DataInvariantViolation) => {
                eprintln!("account event trace projection violated a data invariant");
                Err(ServerFnError::new("記録を読み込めませんでした。"))
            }
            Err(AccountEventTraceStorageError::Database(error)) => {
                eprintln!("failed to read account event trace: {error}");
                Err(ServerFnError::new("記録を読み込めませんでした。"))
            }
        }
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Return an account-authorized plan for continuing one organized event.
#[post("/api/account/history/event-continuation")]
pub async fn get_account_event_continuation_plan(
    input: EventContinuationPlanInput,
) -> std::result::Result<AccountEventContinuationState, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::storage::{
            EventContinuationStorageError, database_pool, find_event_continuation_plan_by_session,
        };

        add_private_response_headers();
        let input =
            input
                .normalized_and_validated()
                .map_err(|errors| ServerFnError::ServerError {
                    message: errors.to_string(),
                    code: 422,
                    details: None,
                })?;
        let session = request_session_context()
            .map_err(|_| ServerFnError::new("続きの情報の安全な接続を確認できませんでした。"))?;
        let token_hash = match session.presented {
            crate::auth::PresentedSession::Absent => {
                return Ok(AccountEventContinuationState::Guest);
            }
            crate::auth::PresentedSession::Invalid => {
                return Ok(AccountEventContinuationState::Expired);
            }
            crate::auth::PresentedSession::Digest(token_hash) => token_hash,
        };

        match find_event_continuation_plan_by_session(
            database_pool(),
            &token_hash,
            &input.origin_event_public_id,
            chrono::Utc::now().timestamp(),
        )
        .await
        {
            Ok(plan) => Ok(AccountEventContinuationState::Authenticated(plan)),
            Err(EventContinuationStorageError::Unauthenticated) => {
                Ok(AccountEventContinuationState::Expired)
            }
            Err(EventContinuationStorageError::NotFound) => Err(ServerFnError::ServerError {
                message: "続きの情報が見つかりません。".to_owned(),
                code: 404,
                details: None,
            }),
            Err(EventContinuationStorageError::Stale) => {
                eprintln!("event continuation plan returned an unexpected stale state");
                Err(ServerFnError::new("続きの情報を読み込めませんでした。"))
            }
            Err(EventContinuationStorageError::DataInvariantViolation) => {
                eprintln!("event continuation plan violated a data invariant");
                Err(ServerFnError::new("続きの情報を読み込めませんでした。"))
            }
            Err(EventContinuationStorageError::Database(_)) => {
                eprintln!("failed to read event continuation plan");
                Err(ServerFnError::new("続きの情報を読み込めませんでした。"))
            }
        }
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Atomically create the next event in an account-owned series.
#[post("/api/account/history/event-continuation/create")]
pub async fn create_account_event_continuation(
    input: EventContinuationCreateInput,
) -> std::result::Result<CreatedEvent, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::storage::{
            EventContinuationStorageError, create_event_continuation_by_session, database_pool,
        };
        use sha2::{Digest, Sha256};
        use uuid::Uuid;

        add_private_response_headers();
        let input =
            input
                .normalized_and_validated()
                .map_err(|errors| ServerFnError::ServerError {
                    message: errors.to_string(),
                    code: 422,
                    details: None,
                })?;
        let session = request_session_context().map_err(|_| {
            ServerFnError::new("続きのイベントの安全な接続を確認できませんでした。")
        })?;
        let token_hash = match session.presented {
            crate::auth::PresentedSession::Absent | crate::auth::PresentedSession::Invalid => {
                return Err(ServerFnError::ServerError {
                    message: "ログインが必要です。".to_owned(),
                    code: 401,
                    details: None,
                });
            }
            crate::auth::PresentedSession::Digest(token_hash) => token_hash,
        };

        let event_public_id = Uuid::new_v4().to_string();
        let organizer_capability =
            format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let organizer_capability_hash =
            format!("{:x}", Sha256::digest(organizer_capability.as_bytes()));
        let event = create_event_continuation_by_session(
            database_pool(),
            &event_public_id,
            &organizer_capability_hash,
            &input,
            &token_hash,
            chrono::Utc::now().timestamp(),
        )
        .await
        .map_err(|error| match error {
            EventContinuationStorageError::Unauthenticated => ServerFnError::ServerError {
                message: "ログインが必要です。".to_owned(),
                code: 401,
                details: None,
            },
            EventContinuationStorageError::NotFound => ServerFnError::ServerError {
                message: "続きの情報が見つかりません。".to_owned(),
                code: 404,
                details: None,
            },
            EventContinuationStorageError::Stale => ServerFnError::ServerError {
                message: "続きの情報が更新されています。最新の情報を読み直してください。"
                    .to_owned(),
                code: 409,
                details: None,
            },
            EventContinuationStorageError::DataInvariantViolation => {
                eprintln!("event continuation creation violated a data invariant");
                ServerFnError::new("続きのイベントを作成できませんでした。")
            }
            EventContinuationStorageError::Database(_) => {
                eprintln!("failed to create event continuation");
                ServerFnError::new("続きのイベントを作成できませんでした。")
            }
        })?;

        Ok(CreatedEvent {
            event,
            organizer_capability,
        })
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

#[cfg(feature = "server")]
fn add_private_response_headers() {
    use dioxus::fullstack::{
        FullstackContext, HeaderValue,
        http::header::{CACHE_CONTROL, X_CONTENT_TYPE_OPTIONS},
    };
    if let Some(context) = FullstackContext::current() {
        context.add_response_header(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        context.add_response_header(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    }
}

#[cfg(feature = "server")]
fn add_retry_after_header(retry_after: i64) {
    use dioxus::fullstack::{HeaderValue, http::header::RETRY_AFTER};
    if let Some(context) = dioxus::fullstack::FullstackContext::current()
        && let Ok(value) = HeaderValue::from_str(&retry_after.to_string())
    {
        context.add_response_header(RETRY_AFTER, value);
    }
}

#[cfg(feature = "server")]
struct RequestSessionContext {
    policy: crate::auth::SessionCookiePolicy,
    presented: crate::auth::PresentedSession,
}

#[cfg(feature = "server")]
fn request_session_context() -> anyhow::Result<RequestSessionContext> {
    use crate::auth::public_origin_for_host;
    use dioxus::fullstack::{FullstackContext, http::header};

    let context = FullstackContext::current()
        .ok_or_else(|| anyhow::anyhow!("account endpoint requires an HTTP request"))?;
    let parts = context.parts_mut();
    let host = parts
        .headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| anyhow::anyhow!("account request is missing a valid Host header"))?;
    let policy = public_origin_for_host(host)?;
    let cookie_header = parts
        .headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok());
    let presented = policy.presented_session_from_cookie_header(cookie_header);
    Ok(RequestSessionContext { policy, presented })
}

#[cfg(feature = "server")]
fn set_session_cookie(
    policy: &crate::auth::SessionCookiePolicy,
    raw_token: &str,
) -> std::result::Result<(), ServerFnError> {
    use dioxus::fullstack::{FullstackContext, HeaderValue, http::header::SET_COOKIE};
    let value = HeaderValue::from_str(&policy.set_cookie_header(raw_token))
        .map_err(|_| ServerFnError::new("session cookieを作成できませんでした。"))?;
    FullstackContext::current()
        .ok_or_else(|| ServerFnError::new("session cookieを返せませんでした。"))?
        .add_response_header(SET_COOKIE, value);
    Ok(())
}

#[cfg(feature = "server")]
fn clear_session_cookie(
    policy: &crate::auth::SessionCookiePolicy,
) -> std::result::Result<(), ServerFnError> {
    use dioxus::fullstack::{FullstackContext, HeaderValue, http::header::SET_COOKIE};
    let value = HeaderValue::from_str(&policy.clear_cookie_header())
        .map_err(|_| ServerFnError::new("session cookieを削除できませんでした。"))?;
    FullstackContext::current()
        .ok_or_else(|| ServerFnError::new("session cookieを返せませんでした。"))?
        .add_response_header(SET_COOKIE, value);
    Ok(())
}

/// Create and persist a new anonymous event.
#[post("/api/events/create")]
pub async fn create_event(input: NewEventInput) -> Result<CreatedEvent> {
    #[cfg(feature = "server")]
    {
        use crate::storage::database_pool;
        use uuid::Uuid;

        let input =
            input
                .normalized_and_validated()
                .map_err(|errors| ServerFnError::ServerError {
                    message: errors.to_string(),
                    code: 422,
                    details: None,
                })?;
        let public_id = Uuid::new_v4().to_string();
        let organizer_capability =
            format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let session = request_session_context()
            .map_err(|_| ServerFnError::new("イベント作成の安全な接続を確認できませんでした。"))?;
        let now = chrono::Utc::now().timestamp();
        let session_token_hash = session.presented.digest().copied();
        let write = persist_created_event_for_session(
            database_pool(),
            input,
            public_id.clone(),
            organizer_capability,
            session_token_hash.as_ref(),
            now,
        )
        .await
        .map_err(|error| {
            eprintln!("failed to create event {public_id}: {error:#}");
            ServerFnError::new("イベントを保存できませんでした。")
        })?;
        Ok(write.value)
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Hash organizer authority and persist one complete creation aggregate.
#[cfg(feature = "server")]
pub async fn persist_created_event(
    pool: &sqlx::SqlitePool,
    input: NewEventInput,
    public_id: String,
    organizer_capability: String,
) -> anyhow::Result<CreatedEvent> {
    Ok(
        persist_created_event_for_session(pool, input, public_id, organizer_capability, None, 0)
            .await?
            .value,
    )
}

/// Hash organizer authority and persist an optionally account-linked creation aggregate.
#[cfg(feature = "server")]
pub async fn persist_created_event_for_session(
    pool: &sqlx::SqlitePool,
    input: NewEventInput,
    public_id: String,
    organizer_capability: String,
    session_token_hash: Option<&[u8; 32]>,
    now: i64,
) -> anyhow::Result<crate::storage::SessionWrite<CreatedEvent>> {
    use crate::storage::create_event_record_for_session;
    use sha2::{Digest, Sha256};

    let organizer_capability_hash =
        format!("{:x}", Sha256::digest(organizer_capability.as_bytes()));
    let event_write = create_event_record_for_session(
        pool,
        &public_id,
        &organizer_capability_hash,
        &input,
        session_token_hash,
        now,
    )
    .await?;

    Ok(crate::storage::SessionWrite {
        value: CreatedEvent {
            event: event_write.value,
            organizer_capability,
        },
        session_status: event_write.session_status,
    })
}

/// Resolve one public-by-link event without returning organizer authority.
#[get("/api/events/get")]
pub async fn get_public_event(public_id: String) -> Result<Option<PublicEvent>> {
    #[cfg(feature = "server")]
    {
        use crate::storage::{database_pool, find_public_event};

        if public_id.len() > 64
            || !public_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Ok(None);
        }

        Ok(find_public_event(database_pool(), &public_id)
            .await
            .map_err(|error| {
                eprintln!("failed to read public event {public_id}: {error:#}");
                ServerFnError::new("イベントを読み込めませんでした。")
            })?)
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Serve one decided public event as a raw iCalendar download.
#[cfg(feature = "server")]
pub async fn download_public_calendar(
    path: std::result::Result<Path<String>, PathRejection>,
) -> dioxus::server::axum::response::Response {
    use crate::{calendar::current_icalendar_timestamp, storage::database_pool};

    let Ok(Path(public_id)) = path else {
        return calendar_not_found_response();
    };
    let generated_at_utc = current_icalendar_timestamp();
    public_calendar_download_response(database_pool(), &public_id, &generated_at_utc).await
}

/// Build the raw HTTP response against an injected pool and generation timestamp.
#[cfg(feature = "server")]
pub async fn public_calendar_download_response(
    pool: &sqlx::SqlitePool,
    public_id: &str,
    generated_at_utc: &str,
) -> dioxus::server::axum::response::Response {
    use crate::{
        calendar::{IcalendarEvent, render_icalendar},
        storage::{PublicEventStorageError, find_public_event},
    };
    use dioxus::server::axum::{
        body::Body,
        http::{StatusCode, header},
        response::Response,
    };

    if uuid::Uuid::parse_str(public_id).is_err() {
        return calendar_not_found_response();
    }

    let event = match find_public_event(pool, public_id).await {
        Ok(Some(event)) => event,
        Ok(None) => {
            return calendar_error_response(StatusCode::NOT_FOUND, "予定が見つかりません。");
        }
        Err(PublicEventStorageError::DataInvariantViolation) => {
            eprintln!("public calendar projection violated a data invariant");
            return calendar_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "カレンダーを作成できませんでした。",
            );
        }
        Err(PublicEventStorageError::Database(_)) => {
            eprintln!("failed to load a public calendar projection");
            return calendar_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "カレンダーを作成できませんでした。",
            );
        }
    };
    let Some(decision) = event.decision else {
        return calendar_error_response(
            StatusCode::CONFLICT,
            "このイベントの日程はまだ決まっていません。",
        );
    };

    let calendar = IcalendarEvent {
        public_id: event.public_id,
        name: event.name,
        organizer_note: event.organizer_note,
        time_zone: event.time_zone,
        local_date: decision.local_date,
        local_time: decision.local_time,
        generated_at_utc: generated_at_utc.to_owned(),
    };
    let body = match render_icalendar(&calendar) {
        Ok(body) => body,
        Err(_) => {
            eprintln!("public event data could not form a safe calendar");
            return calendar_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "カレンダーを作成できませんでした。",
            );
        }
    };
    let filename = format!("attachment; filename=\"tsunoru-{public_id}.ics\"");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/calendar; charset=utf-8")
        .header(header::CONTENT_DISPOSITION, filename)
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(body))
        .expect("calendar response uses validated static headers")
}

#[cfg(feature = "server")]
fn calendar_not_found_response() -> dioxus::server::axum::response::Response {
    calendar_error_response(
        dioxus::server::axum::http::StatusCode::NOT_FOUND,
        "予定が見つかりません。",
    )
}

#[cfg(feature = "server")]
fn calendar_error_response(
    status: dioxus::server::axum::http::StatusCode,
    message: &'static str,
) -> dioxus::server::axum::response::Response {
    use dioxus::server::axum::{body::Body, http::header, response::Response};

    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(message))
        .expect("calendar errors use validated static headers")
}

/// Reject cross-origin or unverifiable unsafe API requests before body decoding.
#[cfg(feature = "server")]
pub async fn require_same_origin_api(
    request: dioxus::server::axum::extract::Request,
    next: dioxus::server::axum::middleware::Next,
) -> dioxus::server::axum::response::Response {
    use crate::auth::{public_origin_for_host, request_origin_is_allowed};
    use dioxus::server::axum::http::{StatusCode, header};

    let method = request.method().as_str();
    let path = request.uri().path().to_owned();
    let unsafe_api = matches!(method, "POST" | "PUT" | "PATCH" | "DELETE")
        && (path == "/api" || path.starts_with("/api/"));
    if unsafe_api {
        let headers = request.headers();
        let Some(host) = headers
            .get(header::HOST)
            .and_then(|value| value.to_str().ok())
        else {
            return api_guard_error(
                StatusCode::FORBIDDEN,
                "request originを確認できませんでした。",
            );
        };
        let policy = match public_origin_for_host(host) {
            Ok(policy) => policy,
            Err(_) => {
                eprintln!("account public-origin configuration is invalid");
                return api_guard_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "server設定を確認できませんでした。",
                );
            }
        };

        let origin_header = headers.get(header::ORIGIN);
        let origin = origin_header.and_then(|value| value.to_str().ok());
        if origin_header.is_some() && origin.is_none() {
            return api_guard_error(
                StatusCode::FORBIDDEN,
                "request originを確認できませんでした。",
            );
        }
        let referer_header = headers.get(header::REFERER);
        let referer = referer_header.and_then(|value| value.to_str().ok());
        if referer_header.is_some() && referer.is_none() {
            return api_guard_error(
                StatusCode::FORBIDDEN,
                "request originを確認できませんでした。",
            );
        }
        if !request_origin_is_allowed(method, &path, origin, referer, policy.origin()) {
            return api_guard_error(
                StatusCode::FORBIDDEN,
                "request originを確認できませんでした。",
            );
        }
    }

    let mut response = next.run(request).await;
    protect_private_api_response(&path, &mut response);
    response
}

/// Apply private-cache headers even when routing or JSON decoding failed before function code.
#[cfg(feature = "server")]
pub fn protect_private_api_response(
    path: &str,
    response: &mut dioxus::server::axum::response::Response,
) {
    use dioxus::server::axum::http::{HeaderValue, header};

    let private = path == "/history"
        || path.starts_with("/history/")
        || path == "/api/auth"
        || path.starts_with("/api/auth/")
        || path == "/api/account"
        || path.starts_with("/api/account/")
        || path == "/api/answers/submit";
    if private {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        response.headers_mut().insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
    }
}

#[cfg(feature = "server")]
fn api_guard_error(
    status: dioxus::server::axum::http::StatusCode,
    message: &'static str,
) -> dioxus::server::axum::response::Response {
    use dioxus::server::axum::{body::Body, http::header, response::Response};
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(message))
        .expect("API guard errors use static headers")
}

/// Resolve one organizer-only response summary without echoing its bearer authority.
#[post("/api/organizer/events/summary")]
pub async fn get_organizer_event_summary(
    input: OrganizerSummaryInput,
) -> std::result::Result<OrganizerEventSummary, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::storage::{OrganizerSummaryStorageError, database_pool};
        use dioxus::fullstack::{FullstackContext, HeaderValue, http::header::CACHE_CONTROL};

        if let Some(context) = FullstackContext::current() {
            context.add_response_header(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        }

        persist_organizer_event_summary(database_pool(), input)
            .await
            .map_err(|error| match error {
                OrganizerSummaryReadError::Validation(errors) => ServerFnError::ServerError {
                    message: errors.to_string(),
                    code: 422,
                    details: None,
                },
                OrganizerSummaryReadError::Storage(OrganizerSummaryStorageError::NotFound) => {
                    ServerFnError::ServerError {
                        message: "回答サマリーが見つかりません。".to_owned(),
                        code: 404,
                        details: None,
                    }
                }
                OrganizerSummaryReadError::Storage(
                    OrganizerSummaryStorageError::DataInvariantViolation,
                ) => {
                    eprintln!("organizer response summary violated a data invariant");
                    ServerFnError::new("回答サマリーを読み込めませんでした。")
                }
                OrganizerSummaryReadError::Storage(OrganizerSummaryStorageError::Database(
                    error,
                )) => {
                    eprintln!("failed to read organizer response summary: {error}");
                    ServerFnError::new("回答サマリーを読み込めませんでした。")
                }
            })
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Resolve one organizer-only response matrix without echoing its bearer authority.
#[post("/api/organizer/events/matrix")]
pub async fn get_organizer_response_matrix(
    input: OrganizerSummaryInput,
) -> std::result::Result<OrganizerResponseMatrix, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::storage::{OrganizerResponseMatrixStorageError, database_pool};
        use dioxus::fullstack::{FullstackContext, HeaderValue, http::header::CACHE_CONTROL};

        if let Some(context) = FullstackContext::current() {
            context.add_response_header(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        }

        persist_organizer_response_matrix(database_pool(), input)
            .await
            .map_err(|error| match error {
                OrganizerResponseMatrixReadError::Validation(errors) => {
                    ServerFnError::ServerError {
                        message: errors.to_string(),
                        code: 422,
                        details: None,
                    }
                }
                OrganizerResponseMatrixReadError::Storage(
                    OrganizerResponseMatrixStorageError::NotFound,
                ) => ServerFnError::ServerError {
                    message: "回答集計表が見つかりません。".to_owned(),
                    code: 404,
                    details: None,
                },
                OrganizerResponseMatrixReadError::Storage(
                    OrganizerResponseMatrixStorageError::DataInvariantViolation,
                ) => {
                    eprintln!("organizer response matrix violated a data invariant");
                    ServerFnError::new("回答集計表を読み込めませんでした。")
                }
                OrganizerResponseMatrixReadError::Storage(
                    OrganizerResponseMatrixStorageError::Database(_),
                ) => {
                    eprintln!("failed to read organizer response matrix");
                    ServerFnError::new("回答集計表を読み込めませんでした。")
                }
            })
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Commit one organizer-selected candidate without echoing its bearer authority.
#[post("/api/organizer/events/decision")]
pub async fn get_organizer_event_decision(
    input: OrganizerDecisionInput,
) -> std::result::Result<OrganizerEventDecision, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::storage::{OrganizerDecisionStorageError, database_pool};
        use dioxus::fullstack::{FullstackContext, HeaderValue, http::header::CACHE_CONTROL};

        if let Some(context) = FullstackContext::current() {
            context.add_response_header(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        }

        persist_organizer_event_decision(database_pool(), input)
            .await
            .map_err(|error| match error {
                OrganizerEventDecisionSubmissionError::Validation(errors) => {
                    ServerFnError::ServerError {
                        message: errors.to_string(),
                        code: 422,
                        details: None,
                    }
                }
                OrganizerEventDecisionSubmissionError::Storage(
                    OrganizerDecisionStorageError::NotFound,
                ) => ServerFnError::ServerError {
                    message: "イベントを確認できませんでした。".to_owned(),
                    code: 404,
                    details: None,
                },
                OrganizerEventDecisionSubmissionError::Storage(
                    OrganizerDecisionStorageError::CandidateMismatch,
                ) => ServerFnError::ServerError {
                    message: "候補日時を確認してください。".to_owned(),
                    code: 422,
                    details: None,
                },
                OrganizerEventDecisionSubmissionError::Storage(
                    OrganizerDecisionStorageError::Conflict,
                ) => ServerFnError::ServerError {
                    message: "別の候補日時がすでに確定されています。".to_owned(),
                    code: 409,
                    details: None,
                },
                OrganizerEventDecisionSubmissionError::Storage(
                    OrganizerDecisionStorageError::Database(_),
                ) => {
                    eprintln!("failed to persist organizer event decision");
                    ServerFnError::new("日程を確定できませんでした。")
                }
            })
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Submit one complete anonymous availability response.
#[post("/api/answers/submit")]
pub async fn submit_availability_response(
    input: NewAvailabilityResponseInput,
) -> std::result::Result<ParticipantResponseMatrix, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::storage::{
            ResponseMatrixStorageError, ResponseStorageError, database_pool,
            find_participant_response_matrix,
        };
        use sha2::{Digest, Sha256};

        let session = request_session_context()
            .map_err(|_| ServerFnError::new("回答送信の安全な接続を確認できませんでした。"))?;
        let now = chrono::Utc::now().timestamp();
        let session_token_hash = session.presented.digest().copied();
        let event_public_id = input.event_public_id.trim().to_owned();
        let response_capability_hash = format!(
            "{:x}",
            Sha256::digest(input.response_capability.trim().as_bytes())
        );
        persist_availability_response_for_session(
            database_pool(),
            input,
            session_token_hash.as_ref(),
            now,
        )
        .await
        .map_err(|error| match error {
            AvailabilityResponseSubmissionError::Validation(errors) => ServerFnError::ServerError {
                message: errors.to_string(),
                code: 422,
                details: None,
            },
            AvailabilityResponseSubmissionError::Storage(ResponseStorageError::EventNotFound) => {
                ServerFnError::ServerError {
                    message: "イベントが見つかりません。".to_owned(),
                    code: 404,
                    details: None,
                }
            }
            AvailabilityResponseSubmissionError::Storage(
                ResponseStorageError::CandidateSetMismatch,
            ) => ServerFnError::ServerError {
                message: "候補日時への回答を確認してください。".to_owned(),
                code: 422,
                details: None,
            },
            AvailabilityResponseSubmissionError::Storage(ResponseStorageError::EventDecided) => {
                ServerFnError::ServerError {
                    message: "日程が確定しています。共有URLを開き直して確認してください。"
                        .to_owned(),
                    code: 409,
                    details: None,
                }
            }
            AvailabilityResponseSubmissionError::Storage(
                ResponseStorageError::CapabilityConflict,
            ) => ServerFnError::ServerError {
                message: "同じ回答を別の内容へ変更できません。".to_owned(),
                code: 409,
                details: None,
            },
            AvailabilityResponseSubmissionError::Storage(ResponseStorageError::Database(error)) => {
                eprintln!("failed to persist anonymous availability response: {error}");
                ServerFnError::new("回答を保存できませんでした。")
            }
        })?;

        find_participant_response_matrix(
            database_pool(),
            &event_public_id,
            &response_capability_hash,
        )
        .await
        .map_err(|error| match error {
            ResponseMatrixStorageError::NotFound => {
                eprintln!("accepted response could not authorize its matrix");
                ServerFnError::new("回答一覧を読み込めませんでした。")
            }
            ResponseMatrixStorageError::DataInvariantViolation => {
                eprintln!("participant response matrix violated a data invariant");
                ServerFnError::new("回答一覧を読み込めませんでした。")
            }
            ResponseMatrixStorageError::Database(_) => {
                eprintln!("failed to read participant response matrix");
                ServerFnError::new("回答一覧を読み込めませんでした。")
            }
        })
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Add one optional comment to the anonymous response authorized by its capability.
#[post("/api/answers/comment")]
pub async fn submit_response_comment(
    input: NewResponseCommentInput,
) -> std::result::Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::storage::{ResponseCommentStorageError, database_pool};

        persist_response_comment(database_pool(), input)
            .await
            .map_err(|error| match error {
                ResponseCommentSubmissionError::Validation(errors) => ServerFnError::ServerError {
                    message: errors.to_string(),
                    code: 422,
                    details: None,
                },
                ResponseCommentSubmissionError::Storage(
                    ResponseCommentStorageError::ResponseNotFound,
                ) => ServerFnError::ServerError {
                    message: "回答を確認できませんでした。".to_owned(),
                    code: 404,
                    details: None,
                },
                ResponseCommentSubmissionError::Storage(
                    ResponseCommentStorageError::CommentConflict,
                ) => ServerFnError::ServerError {
                    message: "異なるひとことがすでに送信されています。".to_owned(),
                    code: 409,
                    details: None,
                },
                ResponseCommentSubmissionError::Storage(ResponseCommentStorageError::Database(
                    error,
                )) => {
                    eprintln!("failed to persist response comment: {error}");
                    ServerFnError::new("ひとことを保存できませんでした。")
                }
            })?;
        Ok(())
    }

    #[cfg(not(feature = "server"))]
    unreachable!("the Dioxus macro replaces this body in the web build")
}

/// Validate, hash, and persist an anonymous response without logging its secrets.
#[cfg(feature = "server")]
pub async fn persist_availability_response(
    pool: &sqlx::SqlitePool,
    input: NewAvailabilityResponseInput,
) -> Result<crate::storage::ResponseWriteOutcome, AvailabilityResponseSubmissionError> {
    Ok(
        persist_availability_response_for_session(pool, input, None, 0)
            .await?
            .value,
    )
}

/// Validate, hash, and persist one optionally account-linked response.
#[cfg(feature = "server")]
pub async fn persist_availability_response_for_session(
    pool: &sqlx::SqlitePool,
    input: NewAvailabilityResponseInput,
    session_token_hash: Option<&[u8; 32]>,
    now: i64,
) -> Result<
    crate::storage::SessionWrite<crate::storage::ResponseWriteOutcome>,
    AvailabilityResponseSubmissionError,
> {
    use crate::storage::record_availability_response_for_session;
    use sha2::{Digest, Sha256};

    let input = input
        .normalized_and_validated()
        .map_err(AvailabilityResponseSubmissionError::Validation)?;
    let response_capability_hash =
        format!("{:x}", Sha256::digest(input.response_capability.as_bytes()));
    record_availability_response_for_session(
        pool,
        &input.event_public_id,
        &response_capability_hash,
        &input.response,
        session_token_hash,
        now,
    )
    .await
    .map_err(AvailabilityResponseSubmissionError::Storage)
}

/// Validate, hash authorization, and persist one optional response comment.
#[cfg(feature = "server")]
pub async fn persist_response_comment(
    pool: &sqlx::SqlitePool,
    input: NewResponseCommentInput,
) -> Result<crate::storage::ResponseCommentWriteOutcome, ResponseCommentSubmissionError> {
    use crate::storage::record_response_comment;
    use sha2::{Digest, Sha256};

    let input = input
        .normalized_and_validated()
        .map_err(ResponseCommentSubmissionError::Validation)?;
    let response_capability_hash =
        format!("{:x}", Sha256::digest(input.response_capability.as_bytes()));
    record_response_comment(
        pool,
        &input.event_public_id,
        &response_capability_hash,
        &input.comment,
    )
    .await
    .map_err(ResponseCommentSubmissionError::Storage)
}

/// Validate and hash organizer authority before reading its private projection.
#[cfg(feature = "server")]
pub async fn persist_organizer_event_summary(
    pool: &sqlx::SqlitePool,
    input: OrganizerSummaryInput,
) -> std::result::Result<OrganizerEventSummary, OrganizerSummaryReadError> {
    use crate::storage::find_organizer_event_summary;
    use sha2::{Digest, Sha256};

    let input = input
        .normalized_and_validated()
        .map_err(OrganizerSummaryReadError::Validation)?;
    let OrganizerSummaryInput {
        event_public_id,
        organizer_capability,
    } = input;
    let organizer_capability_hash =
        format!("{:x}", Sha256::digest(organizer_capability.as_bytes()));
    drop(organizer_capability);

    find_organizer_event_summary(pool, &event_public_id, &organizer_capability_hash)
        .await
        .map_err(OrganizerSummaryReadError::Storage)
}

/// Validate and hash organizer authority before reading its private response matrix.
#[cfg(feature = "server")]
pub async fn persist_organizer_response_matrix(
    pool: &sqlx::SqlitePool,
    input: OrganizerSummaryInput,
) -> std::result::Result<OrganizerResponseMatrix, OrganizerResponseMatrixReadError> {
    use crate::storage::find_organizer_response_matrix;
    use sha2::{Digest, Sha256};

    let input = input
        .normalized_and_validated()
        .map_err(OrganizerResponseMatrixReadError::Validation)?;
    let OrganizerSummaryInput {
        event_public_id,
        organizer_capability,
    } = input;
    let organizer_capability_hash =
        format!("{:x}", Sha256::digest(organizer_capability.as_bytes()));
    drop(organizer_capability);

    find_organizer_response_matrix(pool, &event_public_id, &organizer_capability_hash)
        .await
        .map_err(OrganizerResponseMatrixReadError::Storage)
}

/// Validate and hash organizer authority before committing one candidate decision.
#[cfg(feature = "server")]
pub async fn persist_organizer_event_decision(
    pool: &sqlx::SqlitePool,
    input: OrganizerDecisionInput,
) -> std::result::Result<OrganizerEventDecision, OrganizerEventDecisionSubmissionError> {
    use crate::storage::{EventDecisionWriteOutcome, record_event_decision};
    use sha2::{Digest, Sha256};

    let input = input
        .normalized_and_validated()
        .map_err(OrganizerEventDecisionSubmissionError::Validation)?;
    let OrganizerDecisionInput {
        event_public_id,
        candidate_id,
        organizer_capability,
    } = input;
    let organizer_capability_hash =
        format!("{:x}", Sha256::digest(organizer_capability.as_bytes()));
    drop(organizer_capability);

    let (outcome, decision) = record_event_decision(
        pool,
        &event_public_id,
        &organizer_capability_hash,
        candidate_id,
    )
    .await
    .map_err(OrganizerEventDecisionSubmissionError::Storage)?;

    match outcome {
        EventDecisionWriteOutcome::Created | EventDecisionWriteOutcome::AlreadyDecided => {
            Ok(decision)
        }
    }
}

/// Validation and persistence errors kept typed until the HTTP boundary.
#[cfg(feature = "server")]
#[derive(Debug)]
pub enum AvailabilityResponseSubmissionError {
    Validation(crate::domain::AvailabilityResponseErrors),
    Storage(crate::storage::ResponseStorageError),
}

#[cfg(feature = "server")]
impl std::fmt::Display for AvailabilityResponseSubmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(errors) => write!(formatter, "response validation failed: {errors}"),
            Self::Storage(error) => write!(formatter, "response persistence failed: {error}"),
        }
    }
}

#[cfg(feature = "server")]
impl std::error::Error for AvailabilityResponseSubmissionError {}

/// Response-comment validation and persistence failures kept typed until the HTTP boundary.
#[cfg(feature = "server")]
#[derive(Debug)]
pub enum ResponseCommentSubmissionError {
    Validation(crate::domain::ResponseCommentErrors),
    Storage(crate::storage::ResponseCommentStorageError),
}

#[cfg(feature = "server")]
impl std::fmt::Display for ResponseCommentSubmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(errors) => write!(formatter, "comment validation failed: {errors}"),
            Self::Storage(error) => write!(formatter, "comment persistence failed: {error}"),
        }
    }
}

#[cfg(feature = "server")]
impl std::error::Error for ResponseCommentSubmissionError {}

/// Organizer-summary validation and repository failures kept typed until the HTTP boundary.
#[cfg(feature = "server")]
#[derive(Debug)]
pub enum OrganizerSummaryReadError {
    Validation(crate::domain::OrganizerSummaryErrors),
    Storage(crate::storage::OrganizerSummaryStorageError),
}

#[cfg(feature = "server")]
impl std::fmt::Display for OrganizerSummaryReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(errors) => write!(formatter, "summary validation failed: {errors}"),
            Self::Storage(error) => write!(formatter, "summary read failed: {error}"),
        }
    }
}

#[cfg(feature = "server")]
impl std::error::Error for OrganizerSummaryReadError {}

/// Organizer-matrix validation and repository failures kept typed until the HTTP boundary.
#[cfg(feature = "server")]
pub enum OrganizerResponseMatrixReadError {
    Validation(crate::domain::OrganizerSummaryErrors),
    Storage(crate::storage::OrganizerResponseMatrixStorageError),
}

#[cfg(feature = "server")]
impl std::fmt::Debug for OrganizerResponseMatrixReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(_) => {
                formatter.write_str("OrganizerResponseMatrixReadError::Validation")
            }
            Self::Storage(crate::storage::OrganizerResponseMatrixStorageError::NotFound) => {
                formatter.write_str("OrganizerResponseMatrixReadError::NotFound")
            }
            Self::Storage(
                crate::storage::OrganizerResponseMatrixStorageError::DataInvariantViolation,
            ) => formatter.write_str("OrganizerResponseMatrixReadError::DataInvariantViolation"),
            Self::Storage(crate::storage::OrganizerResponseMatrixStorageError::Database(_)) => {
                formatter.write_str("OrganizerResponseMatrixReadError::Database")
            }
        }
    }
}

#[cfg(feature = "server")]
impl std::fmt::Display for OrganizerResponseMatrixReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(_) => formatter.write_str("matrix validation failed"),
            Self::Storage(crate::storage::OrganizerResponseMatrixStorageError::NotFound) => {
                formatter.write_str("matrix read failed: not found")
            }
            Self::Storage(
                crate::storage::OrganizerResponseMatrixStorageError::DataInvariantViolation,
            ) => formatter.write_str("matrix read failed: data invariant violation"),
            Self::Storage(crate::storage::OrganizerResponseMatrixStorageError::Database(_)) => {
                formatter.write_str("matrix read failed: database failure")
            }
        }
    }
}

#[cfg(feature = "server")]
impl std::error::Error for OrganizerResponseMatrixReadError {}

/// Organizer-decision validation and repository failures kept typed until the HTTP boundary.
#[cfg(feature = "server")]
pub enum OrganizerEventDecisionSubmissionError {
    Validation(crate::domain::OrganizerDecisionErrors),
    Storage(crate::storage::OrganizerDecisionStorageError),
}

#[cfg(feature = "server")]
impl std::fmt::Debug for OrganizerEventDecisionSubmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(_) => {
                formatter.write_str("OrganizerEventDecisionSubmissionError::Validation")
            }
            Self::Storage(crate::storage::OrganizerDecisionStorageError::NotFound) => {
                formatter.write_str("OrganizerEventDecisionSubmissionError::NotFound")
            }
            Self::Storage(crate::storage::OrganizerDecisionStorageError::CandidateMismatch) => {
                formatter.write_str("OrganizerEventDecisionSubmissionError::CandidateMismatch")
            }
            Self::Storage(crate::storage::OrganizerDecisionStorageError::Conflict) => {
                formatter.write_str("OrganizerEventDecisionSubmissionError::Conflict")
            }
            Self::Storage(crate::storage::OrganizerDecisionStorageError::Database(_)) => {
                formatter.write_str("OrganizerEventDecisionSubmissionError::Database")
            }
        }
    }
}

#[cfg(feature = "server")]
impl std::fmt::Display for OrganizerEventDecisionSubmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(_) => formatter.write_str("decision validation failed"),
            Self::Storage(crate::storage::OrganizerDecisionStorageError::NotFound) => {
                formatter.write_str("decision persistence failed: not found")
            }
            Self::Storage(crate::storage::OrganizerDecisionStorageError::CandidateMismatch) => {
                formatter.write_str("decision persistence failed: candidate mismatch")
            }
            Self::Storage(crate::storage::OrganizerDecisionStorageError::Conflict) => {
                formatter.write_str("decision persistence failed: conflict")
            }
            Self::Storage(crate::storage::OrganizerDecisionStorageError::Database(_)) => {
                formatter.write_str("decision persistence failed: database failure")
            }
        }
    }
}

#[cfg(feature = "server")]
impl std::error::Error for OrganizerEventDecisionSubmissionError {}
