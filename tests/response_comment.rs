use dioxus::prelude::*;
use std::future::Future;
use tsunoru::{
    domain::{NewResponseCommentInput, RESPONDENT_COMMENT_MAX_CHARS, ResponseCommentDraft},
    server::submit_response_comment,
    ui::{
        AvailabilityResponseSuccess, ResponseCommentOffer, ResponseCommentSuccess,
        response_comment_failure_message,
    },
};

const RAW_RESPONSE_CAPABILITY: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn require_http_error_contract<F>(_: F)
where
    F: Future<Output = std::result::Result<(), ServerFnError>>,
{
}

fn textarea_opening_tag(html: &str) -> &str {
    let start = html
        .find("<textarea")
        .expect("the optional offer should render one textarea");
    let end = html[start..]
        .find('>')
        .map(|relative| start + relative + 1)
        .expect("the textarea opening tag should be complete");
    &html[start..end]
}

#[test]
fn answer_completion_precedes_the_optional_comment_offer() {
    let capability_for_submit = RAW_RESPONSE_CAPABILITY.to_owned();
    let capability_for_skip = RAW_RESPONSE_CAPABILITY.to_owned();
    let html = dioxus_ssr::render_element(rsx! {
        AvailabilityResponseSuccess {}
        ResponseCommentOffer {
            initial_comment: String::new(),
            initial_error: None,
            submitting: false,
            on_submit: move |_: String| {
                let _secret_kept_in_parent_callback = &capability_for_submit;
            },
            on_skip: move |_: ()| {
                let _secret_kept_in_parent_callback = &capability_for_skip;
            },
        }
    });

    let completion = html
        .find("回答を送りました")
        .expect("the availability answer must already be complete");
    let offer = html
        .find("ひとこと添える？")
        .expect("the optional comment offer should follow completion");
    assert!(
        completion < offer,
        "answer completion must be announced before the optional action: {html}"
    );

    for expected in [
        "ここまでで回答は完了",
        "このまま閉じて",
        "ひとこと添える？",
        "任意",
        "調整ありがとう！",
        "楽しみ！",
        "ひとことを送る",
        "今回は送らない",
        "<textarea",
        &format!("maxlength={RESPONDENT_COMMENT_MAX_CHARS}"),
    ] {
        assert!(
            html.contains(expected),
            "the post-answer experience should contain {expected:?}: {html}"
        );
    }

    assert_eq!(
        html.matches("type=\"button\"").count(),
        3,
        "two examples and skip must be non-submitting buttons: {html}"
    );
    let textarea = textarea_opening_tag(&html);
    assert!(
        !textarea.contains("required")
            && !textarea.contains("autofocus")
            && !html.contains("type=\"hidden\"")
            && !html.contains(RAW_RESPONSE_CAPABILITY),
        "the optional field must stay optional and initially unfocused, while the callback secret must not reach HTML: {html}"
    );
}

#[test]
fn comment_failure_keeps_the_body_and_retry_controls() {
    let html = dioxus_ssr::render_element(rsx! {
        ResponseCommentOffer {
            initial_comment: "  餃子は多めでお願いします  ".to_owned(),
            initial_error: Some(
                "ひとことを送れませんでした。内容は残っています。もう一度お試しください。"
                    .to_owned(),
            ),
            submitting: false,
            on_submit: move |_: String| {},
            on_skip: move |_: ()| {},
        }
    });

    assert!(
        html.contains("餃子は多めでお願いします")
            && html.contains("role=\"status\"")
            && html.contains("id=\"response-comment-error\"")
            && html.contains("aria-describedby=\"response-comment-error\"")
            && html.contains("内容は残っています")
            && html.contains("ひとことを送る")
            && html.contains("今回は送らない"),
        "a failed optional request must preserve the draft and both exit paths: {html}"
    );
    assert!(
        !html.contains("ひとことも送りました"),
        "a failed save must not claim comment completion: {html}"
    );
    assert!(
        !textarea_opening_tag(&html).contains("aria-invalid=true")
            && !textarea_opening_tag(&html).contains("aria-invalid=\"true\""),
        "a transport failure must not describe valid retained text as invalid: {html}"
    );
}

#[test]
fn comment_success_is_focusable_without_story_four_output() {
    let html = dioxus_ssr::render_element(rsx! { ResponseCommentSuccess {} });

    assert!(
        html.contains("id=\"response-comment-success-heading\"")
            && html.contains("tabindex=\"-1\"")
            && html.contains("ひとことも送りました"),
        "the second asynchronous success needs a programmatic focus target: {html}"
    );
    for forbidden in [
        "回答サマリー",
        "回答人数",
        "コメント一覧",
        "みんなから",
        "他の回答",
    ] {
        assert!(
            !html.contains(forbidden),
            "Story 3 must not expose Story 4 output {forbidden:?}: {html}"
        );
    }
}

#[test]
fn comment_draft_trims_unicode_whitespace_and_accepts_exactly_five_hundred_characters() {
    let trimmed = ResponseCommentDraft {
        comment: "\u{3000}\n 調整ありがとう！ \t".to_owned(),
    }
    .prepare()
    .expect("a short plain-text utterance should be accepted");
    assert_eq!(trimmed.comment, "調整ありがとう！");

    let boundary = "声".repeat(RESPONDENT_COMMENT_MAX_CHARS);
    let prepared = ResponseCommentDraft {
        comment: boundary.clone(),
    }
    .prepare()
    .expect("the documented 500-character boundary should be accepted");
    assert_eq!(prepared.comment, boundary);
    assert_eq!(prepared.comment.chars().count(), 500);
}

#[test]
fn comment_draft_rejects_blank_oversized_and_nul_text() {
    let blank = ResponseCommentDraft {
        comment: " \n\t\u{3000}".to_owned(),
    }
    .prepare()
    .expect_err("skip must not be persisted as a blank comment");
    assert_eq!(
        blank.comment.as_deref(),
        Some("ひとことを入力してください。")
    );

    let oversized = ResponseCommentDraft {
        comment: "声".repeat(RESPONDENT_COMMENT_MAX_CHARS + 1),
    }
    .prepare()
    .expect_err("anonymous comment storage needs a bounded payload");
    assert_eq!(
        oversized.comment.as_deref(),
        Some("ひとことは500文字以内で入力してください。")
    );

    let nul = ResponseCommentDraft {
        comment: "楽しみ\0！".to_owned(),
    }
    .prepare()
    .expect_err("NUL must not cross the text storage boundary");
    assert_eq!(
        nul.comment.as_deref(),
        Some("ひとことに使用できない文字が含まれています。")
    );
}

#[test]
fn comment_server_input_revalidates_event_capability_and_comment() {
    let valid = NewResponseCommentInput {
        event_public_id: "event-one".to_owned(),
        response_capability: "a1".repeat(32),
        comment: "  楽しみ！  ".to_owned(),
    };
    let normalized = valid
        .normalized_and_validated()
        .expect("a well-shaped comment request should cross the server boundary");
    assert_eq!(normalized.comment, "楽しみ！");

    let mut invalid_event = valid.clone();
    invalid_event.event_public_id = "../other-event".to_owned();
    assert!(invalid_event.normalized_and_validated().is_err());

    let mut invalid_capability = valid.clone();
    invalid_capability.response_capability = "A".repeat(64);
    assert!(invalid_capability.normalized_and_validated().is_err());

    let mut nul_comment = valid;
    nul_comment.comment = "hello\0world".to_owned();
    assert!(nul_comment.normalized_and_validated().is_err());
}

#[test]
fn comment_submission_preserves_application_http_statuses() {
    let input = NewResponseCommentInput {
        event_public_id: "event-one".to_owned(),
        response_capability: "ab".repeat(32),
        comment: "調整ありがとう！".to_owned(),
    };

    require_http_error_contract(submit_response_comment(input));
}

#[test]
fn a_changed_retry_never_claims_the_new_comment_was_saved() {
    let conflict = ServerFnError::ServerError {
        message: "server detail is not shown".to_owned(),
        code: 409,
        details: None,
    };
    assert_eq!(
        response_comment_failure_message(&conflict),
        "先のひとことは送信済みです。変更した内容は保存されていません。回答は完了しています。"
    );

    let unexpected = ServerFnError::new("transport detail is not shown");
    assert_eq!(
        response_comment_failure_message(&unexpected),
        "ひとことを送れませんでした。内容は残っています。もう一度お試しください。"
    );
}
