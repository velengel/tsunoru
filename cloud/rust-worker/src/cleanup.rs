//! Scheduled cleanup boundary.

/// Retention SQL is kept with the scheduled cleanup responsibility.
pub(crate) const EVENT_RETENTION_SQL: &str =
    "DELETE FROM events WHERE created_at IS NOT NULL AND created_at < ?1";
