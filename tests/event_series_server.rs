#[test]
fn continuation_uses_separate_private_post_endpoints_and_explicit_statuses() {
    let server = include_str!("../src/server.rs");
    for expected in [
        "#[post(\"/api/account/history/event-continuation\")]",
        "pub async fn get_account_event_continuation_plan(",
        "#[post(\"/api/account/history/event-continuation/create\")]",
        "pub async fn create_account_event_continuation(",
        "AccountEventContinuationState::Guest",
        "AccountEventContinuationState::Expired",
        "EventContinuationStorageError::NotFound",
        "EventContinuationStorageError::Stale",
        "code: 401",
        "code: 404",
        "code: 409",
        "add_private_response_headers",
    ] {
        assert!(
            server.contains(expected),
            "missing server contract {expected:?}"
        );
    }
}

#[test]
fn anonymous_creation_does_not_gain_series_input_or_fallback_logic() {
    let server = include_str!("../src/server.rs");
    let start = server
        .find("pub async fn create_event(input: NewEventInput)")
        .unwrap();
    let end = server[start..]
        .find("/// Resolve one public-by-link event")
        .map(|offset| start + offset)
        .unwrap();
    let anonymous_create = &server[start..end];

    for forbidden in [
        "series",
        "continuation",
        "expected_tail",
        "EventContinuation",
    ] {
        assert!(
            !anonymous_create.contains(forbidden),
            "anonymous create must stay series-free: {forbidden:?}"
        );
    }
}

#[test]
fn continuation_route_is_private_generic_ssr_and_cookie_neutral() {
    let routes = include_str!("../src/lib.rs");
    assert!(routes.contains("#[route(\"/history/events/:public_id/continue\")]"));
    assert!(routes.contains("ContinueHistoryEvent { public_id: String }"));

    let ui = include_str!("../src/ui.rs");
    assert!(ui.contains("pub fn ContinueHistoryEvent(public_id: String)"));
    assert!(ui.contains("AccountEventContinuationLoading"));
    assert!(ui.contains("use_reactive((&public_id,),"));
    assert!(ui.contains("key: \"{public_id}\""));
    assert!(ui.contains("noindex,nofollow"));

    let server = include_str!("../src/server.rs");
    for function in [
        "pub async fn get_account_event_continuation_plan",
        "pub async fn create_account_event_continuation",
    ] {
        let start = server.find(function).unwrap();
        let end = server[start..]
            .find("\n}\n")
            .map(|offset| start + offset)
            .unwrap();
        let body = &server[start..end];
        assert!(!body.contains("set_session_cookie"));
        assert!(!body.contains("clear_session_cookie"));
    }
}
