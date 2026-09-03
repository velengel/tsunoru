use tsunoru::domain::{
    AccountAuthErrors, AccountHistory, AccountLoginInput, AccountRegistrationInput,
    HistoryDecision, OrganizedEventHistoryItem, ParticipatedEventHistoryItem,
};

const VALID_PASSWORD: &str = "correct horse battery staple";

#[test]
fn registration_normalizes_only_the_login_id() {
    let prepared = (AccountRegistrationInput {
        login_id: "  Gyoza.Friends_26  ".to_owned(),
        password: VALID_PASSWORD.to_owned(),
        password_confirmation: VALID_PASSWORD.to_owned(),
    })
    .prepare()
    .expect("a bounded ASCII login ID and long password should be accepted");

    assert_eq!(prepared.login_id, "gyoza.friends_26");
    assert_eq!(prepared.password, VALID_PASSWORD);
}

#[test]
fn login_id_shape_and_password_bounds_are_reported_by_field() {
    let malformed = AccountRegistrationInput {
        login_id: "_利用者".to_owned(),
        password: "short".to_owned(),
        password_confirmation: "different".to_owned(),
    }
    .prepare()
    .expect_err("invalid account input must not reach password hashing");

    assert!(malformed.login_id.is_some());
    assert!(malformed.password.is_some());
    assert!(malformed.password_confirmation.is_some());

    let oversized_utf8 = AccountLoginInput {
        login_id: "valid-id".to_owned(),
        password: "界".repeat(171),
    }
    .prepare()
    .expect_err("a password above 512 UTF-8 octets must be bounded");
    assert!(oversized_utf8.password.is_some());
}

#[test]
fn password_whitespace_is_data_and_not_trimmed() {
    let password = "  fifteen chars and spaces  ";
    let prepared = (AccountLoginInput {
        login_id: "reader".to_owned(),
        password: password.to_owned(),
    })
    .prepare()
    .expect("password whitespace is intentional data");

    assert_eq!(prepared.password, password);
}

#[test]
fn password_character_limit_accepts_unicode_up_to_the_separate_byte_limit() {
    let password = "😀".repeat(128);
    let prepared = AccountLoginInput {
        login_id: "reader".to_owned(),
        password: password.clone(),
    }
    .prepare()
    .expect("128 Unicode scalar values and 512 UTF-8 octets are valid");
    assert_eq!(prepared.password, password);

    let too_many = AccountLoginInput {
        login_id: "reader".to_owned(),
        password: "😀".repeat(129),
    }
    .prepare()
    .expect_err("the character limit remains independent from the byte limit");
    assert!(too_many.password.is_some());
}

#[test]
fn account_inputs_redact_every_password_from_debug_output() {
    let registration = AccountRegistrationInput {
        login_id: "reader".to_owned(),
        password: VALID_PASSWORD.to_owned(),
        password_confirmation: VALID_PASSWORD.to_owned(),
    };
    let login = AccountLoginInput {
        login_id: "reader".to_owned(),
        password: VALID_PASSWORD.to_owned(),
    };

    for debug in [format!("{registration:?}"), format!("{login:?}")] {
        assert!(
            debug.contains("[REDACTED]"),
            "debug should mark redaction: {debug}"
        );
        assert!(
            !debug.contains(VALID_PASSWORD),
            "plain passwords must not enter diagnostics: {debug}"
        );
    }
}

#[test]
fn empty_auth_errors_remain_a_small_form_contract() {
    assert!(AccountAuthErrors::default().is_empty());
}

#[test]
fn serialized_history_contains_only_the_minimum_public_projection() {
    let history = AccountHistory {
        login_id: "reader".to_owned(),
        organized_standalone: vec![OrganizedEventHistoryItem {
            public_id: "public-organized".to_owned(),
            name: "餃子会".to_owned(),
            time_zone: "Asia/Tokyo".to_owned(),
            decision: Some(HistoryDecision {
                local_date: "2026-09-18".to_owned(),
                local_time: "19:00".to_owned(),
            }),
            response_count: 3,
        }],
        organized_series: Vec::new(),
        participated: vec![ParticipatedEventHistoryItem {
            public_id: "public-participated".to_owned(),
            name: "読書会".to_owned(),
            time_zone: "Asia/Tokyo".to_owned(),
            decision: None,
        }],
    };

    let serialized = serde_json::to_string(&history).expect("history should cross the typed RPC");
    for expected in ["reader", "public-organized", "餃子会", "response_count"] {
        assert!(
            serialized.contains(expected),
            "missing {expected:?}: {serialized}"
        );
    }
    for forbidden in [
        "password",
        "session",
        "capability",
        "respondent_name",
        "availability",
        "comment",
        "account_id",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "history must not expose {forbidden:?}: {serialized}"
        );
    }
}
