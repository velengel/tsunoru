#![cfg(feature = "server")]

use tsunoru::domain::{CandidateInput, NewEventInput};

#[test]
fn server_build_rejects_a_nonexistent_iana_time_zone() {
    let input = NewEventInput {
        name: "架空時差の会".to_owned(),
        organizer_note: None,
        time_zone: "Fake/Zone".to_owned(),
        candidates: vec![CandidateInput {
            local_date: "2026-09-18".to_owned(),
            local_time: "19:00".to_owned(),
        }],
    };

    assert!(
        input.normalized_and_validated().is_err(),
        "the server must validate names against an IANA database, not only their shape"
    );
}
