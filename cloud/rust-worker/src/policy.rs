use crate::{ApiResult, organizer_auth, session};
use worker::{Env, Method, Request};

pub(crate) fn authorize(
    request: &Request,
    env: &Env,
    method: &Method,
    segments: &[&str],
    google_enabled: bool,
) -> ApiResult<()> {
    let organizer_mutation = matches!(
        (method, segments),
        (Method::Post, ["", "api", "events"])
            | (Method::Get, ["", "api", "events", _, "responses"])
            | (Method::Delete, ["", "api", "events", _])
            | (
                Method::Delete,
                ["", "api", "events", _, "responses", _, "capability"]
            )
    );
    if organizer_mutation {
        if google_enabled {
            organizer_auth::authorize(request, env)?;
        } else {
            session::authorize(request, env)?;
        }
    } else if !google_enabled {
        session::authorize(request, env)?;
    }
    Ok(())
}

pub(crate) fn operation(method: &Method, segments: &[&str]) -> Option<&'static str> {
    match (method, segments) {
        (Method::Post, ["", "api", "events"]) => Some("create"),
        (Method::Post, ["", "api", "events", _, "responses"]) => Some("response"),
        (Method::Get, ["", "api", "events", _])
        | (Method::Get, ["", "api", "events", _, "responses"]) => Some("read"),
        _ => None,
    }
}
