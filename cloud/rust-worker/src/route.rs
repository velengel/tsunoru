//! HTTP route boundary.

/// Classifies the API path before authorization and dispatch.
pub(crate) fn route(path: &str) -> Option<(&str, &str)> {
    let path = path.strip_prefix("/api/")?;
    let (resource, remainder) = path.split_once('/').unwrap_or((path, ""));
    Some((resource, remainder))
}
