#[test]
fn event_trace_is_a_private_post_with_role_scoped_repository_errors() {
    let server = include_str!("../src/server.rs");
    for expected in [
        "#[post(\"/api/account/history/event-detail\")]",
        "pub async fn get_account_event_trace(",
        "AccountEventTraceState::Guest",
        "AccountEventTraceState::Expired",
        "AccountEventTraceStorageError::NotFound",
        "AccountEventTraceStorageError::DataInvariantViolation",
        "message: \"記録が見つかりません。\".to_owned()",
    ] {
        assert!(
            server.contains(expected),
            "missing server boundary {expected:?}"
        );
    }
}

#[test]
fn account_detail_and_html_history_are_non_cacheable_without_cookie_mutation() {
    let server = include_str!("../src/server.rs");
    assert!(server.contains("path == \"/history\""));
    assert!(server.contains("path.starts_with(\"/history/\")"));

    let detail_start = server.find("pub async fn get_account_event_trace").unwrap();
    let detail_end = server[detail_start..]
        .find("#[cfg(feature = \"server\")]\nfn add_private_response_headers")
        .map(|offset| detail_start + offset)
        .unwrap();
    let detail = &server[detail_start..detail_end];
    assert!(detail.contains("add_private_response_headers"));
    assert!(!detail.contains("set_session_cookie"));
    assert!(!detail.contains("clear_session_cookie"));
}

#[test]
fn story_ten_and_social_features_do_not_enter_the_trace_contract() {
    let domain = include_str!("../src/domain.rs");
    let start = domain.find("pub struct AccountEventTraceInput").unwrap();
    let end = domain[start..]
        .find("pub struct CandidateInput")
        .map(|offset| start + offset)
        .unwrap();
    let contract = &domain[start..end];
    for forbidden in [
        "series",
        "suggest",
        "next_name",
        "reaction",
        "photo",
        "timeline",
        "activity_log",
    ] {
        assert!(
            !contract.contains(forbidden),
            "Story 9 must not contain {forbidden}"
        );
    }
}
