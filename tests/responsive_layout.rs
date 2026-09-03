const CSS: &str = include_str!("../assets/main.css");

fn rule_body(selector: &str) -> &str {
    CSS.split_once(selector)
        .and_then(|(_, rest)| rest.split_once('{'))
        .and_then(|(_, rest)| rest.split_once('}'))
        .map(|(body, _)| body)
        .unwrap_or_else(|| panic!("missing CSS rule for {selector}"))
}

fn rule_body_in<'a>(source: &'a str, selector: &str) -> &'a str {
    source
        .split_once(selector)
        .and_then(|(_, rest)| rest.split_once('{'))
        .and_then(|(_, rest)| rest.split_once('}'))
        .map(|(body, _)| body)
        .unwrap_or_else(|| panic!("missing CSS rule for {selector} in scoped stylesheet"))
}

#[test]
fn responsive_layout_prevents_page_and_action_label_overflow() {
    assert!(
        rule_body(".app-shell").contains("overflow-x: clip"),
        "the application shell should prevent horizontal page overflow"
    );
    assert!(
        rule_body(".page-heading h1").contains("white-space: nowrap"),
        "the short creation heading should not split in the middle on desktop"
    );
    assert!(
        rule_body("button,\n.primary-button,\n.secondary-button")
            .contains("overflow-wrap: anywhere"),
        "localized action labels should wrap inside their own control"
    );
    assert!(
        CSS.contains("@media (max-width: 520px)")
            && CSS.contains("grid-template-columns: minmax(0, 1fr)"),
        "narrow controls should collapse to one shrinkable column"
    );
}

#[test]
fn small_muted_text_uses_the_reviewed_accessible_color() {
    assert!(
        CSS.contains("color: #59645b"),
        "small helper text should use the reviewed muted foreground"
    );
    for rejected in ["color: #707970", "color: #737b74", "color: #7a807a"] {
        assert!(
            !CSS.contains(rejected),
            "reviewed low-contrast foreground must be removed: {rejected}"
        );
    }
}

#[test]
fn account_history_reflows_the_intended_selectors_at_phone_width() {
    let account_page = rule_body(".account-page");
    assert!(
        account_page.contains("width: min(100%, 32rem)")
            && account_page.contains("overflow-wrap: anywhere"),
        "the account form card must shrink and wrap long values"
    );
    let history_page = rule_body(".history-page");
    assert!(
        history_page.contains("width: min(100%, 72rem)") && history_page.contains("min-width: 0"),
        "the history surface must fit its viewport"
    );
    assert!(
        rule_body(".history-grid").contains("grid-template-columns: repeat(2, minmax(0, 1fr))"),
        "desktop should compare the two role histories"
    );
    let history_link = rule_body(".history-list a");
    assert!(
        history_link.contains("min-height: 44px")
            && history_link.contains("overflow-wrap: anywhere"),
        "long event names need a wrapping mobile-sized target"
    );

    let narrow = CSS
        .split_once("@media (max-width: 760px)")
        .map(|(_, rest)| rest)
        .expect("the account layout should define its narrow breakpoint");
    assert!(
        rule_body_in(narrow, ".history-grid").contains("grid-template-columns: minmax(0, 1fr)"),
        "the two role histories must stack below 760px"
    );
}

#[test]
fn account_event_trace_reflows_long_private_content_without_a_wide_table() {
    let css = include_str!("../assets/main.css");
    for expected in [
        ".history-trace-page",
        ".trace-response-list",
        ".trace-response",
        ".trace-answers",
        ".trace-comment",
        "overflow-wrap: anywhere",
        "min-width: 0",
        "min-height: 44px",
        ":focus-visible",
        "@media (max-width: 520px)",
    ] {
        assert!(
            css.contains(expected),
            "missing trace responsive rule {expected:?}"
        );
    }

    let ui = include_str!("../src/ui.rs");
    let start = ui.find("pub fn AccountEventTraceView").unwrap();
    let trace = &ui[start..];
    assert!(!trace.contains("<table"));
    assert!(trace.contains("class: \"trace-answers\""));

    let response_rule_start = css.find(".trace-response {").unwrap();
    let response_rule_end = css[response_rule_start..]
        .find("}\n")
        .map(|offset| response_rule_start + offset)
        .unwrap();
    let response_rule = &css[response_rule_start..response_rule_end];
    assert!(
        !response_rule.contains("overflow: clip"),
        "the details border must not clip the summary focus ring"
    );
    assert!(css.contains(".trace-response summary::before"));
    assert!(css.contains(".trace-response[open] summary::before"));
}

#[test]
fn event_continuation_and_series_history_reflow_without_hiding_exit_paths() {
    for expected in [
        ".event-continuation-page",
        ".continuation-context",
        ".continuation-exit-links",
        ".history-series-list",
        ".history-series",
        ".history-series summary",
        "overflow-wrap: anywhere",
        "min-width: 0",
        "min-height: 44px",
    ] {
        assert!(
            CSS.contains(expected),
            "missing continuation responsive contract {expected:?}"
        );
    }

    let page = rule_body(".event-continuation-page");
    assert!(page.contains("width: min(100%, 42rem)") && page.contains("min-width: 0"));
    let exits = rule_body(".continuation-exit-links");
    assert!(exits.contains("display: grid") && exits.contains("min-width: 0"));
    let summary = rule_body(".history-series summary");
    assert!(summary.contains("min-height: 44px") && summary.contains("cursor: pointer"));
    assert!(
        CSS.contains(".history-series summary::before")
            && CSS.contains(".history-series[open] summary::before"),
        "a flex summary needs an explicit visible disclosure marker"
    );
    assert!(
        rule_body(".history-series summary:focus-visible").contains("outline: 3px solid"),
        "the series disclosure must expose keyboard focus"
    );

    let narrow = CSS
        .split_once("@media (max-width: 520px)")
        .map(|(_, rest)| rest)
        .expect("the stylesheet should define the existing narrow breakpoint");
    assert!(
        rule_body_in(narrow, ".continuation-exit-links")
            .contains("grid-template-columns: minmax(0, 1fr)"),
        "continuation exits must stack at phone width"
    );
}

#[test]
fn availability_controls_reflow_as_three_keyboard_visible_choices() {
    let options = rule_body(".availability-options");
    assert!(
        options.contains("grid-template-columns: repeat(3, minmax(0, 1fr))"),
        "the three meanings should stay equally reachable at 320px"
    );
    assert!(
        rule_body(".availability-option-label").contains("min-height: 44px")
            && rule_body(".availability-option-label").contains("border: 1px solid #718675"),
        "each visible choice should retain a mobile-sized target"
    );
    assert!(
        rule_body(".availability-radio:focus-visible + .availability-option-label")
            .contains("outline: 3px solid"),
        "a visually hidden native radio must expose keyboard focus on its label"
    );
    assert!(
        CSS.contains(".availability-radio:checked + .availability-option-label"),
        "selection must have a visible non-default state"
    );
}

#[test]
fn optional_comment_controls_keep_a_shrinkable_mobile_exit_path() {
    let offer = rule_body(".comment-offer");
    assert!(
        offer.contains("width: 100%")
            && offer.contains("min-width: 0")
            && offer.contains("text-align: left"),
        "the optional panel should fill, shrink, and restore reading alignment inside success"
    );

    let suggestions = rule_body(".comment-suggestions");
    assert!(
        suggestions.contains("grid-template-columns: repeat(2, minmax(0, 1fr))")
            && suggestions.contains("min-width: 0"),
        "desktop examples should share a shrinkable two-column row"
    );
    assert!(
        rule_body(".comment-actions").contains("min-width: 0"),
        "send and skip actions must be able to shrink inside the card"
    );

    assert!(
        CSS.contains(".comment-suggestions,\n  .comment-actions")
            && CSS.contains("grid-template-columns: minmax(0, 1fr)"),
        "examples and actions should stack into one column at the 320px layout"
    );
    let suggestion = rule_body(".comment-suggestion");
    assert!(
        suggestion.contains("min-height: 44px") && suggestion.contains("border: 1px solid #718675"),
        "each example needs a mobile-sized target and a distinguishable boundary"
    );
    assert!(
        rule_body(".comment-actions .secondary-button").contains("border-color: #718675"),
        "the optional exit action must remain visibly identifiable against the panel"
    );
    assert!(
        CSS.contains(".response-success-detail") && !CSS.contains(".response-success p:last-child"),
        "nested comment help and errors must not inherit the old last-paragraph styling"
    );
}

#[test]
fn organizer_summary_reflows_cards_counts_comments_and_recovery_at_three_hundred_twenty_pixels() {
    let page = rule_body(".organizer-summary-page");
    assert!(
        page.contains("width: 100%")
            && page.contains("min-width: 0")
            && !page.contains("overflow-x: auto"),
        "the organizer surface must fill and shrink without becoming a horizontal viewport"
    );

    let cards = rule_body(".summary-card-grid");
    assert!(
        cards.contains("grid-template-columns: repeat(2, minmax(0, 1fr))")
            && cards.contains("min-width: 0"),
        "wide screens should compare authored candidate cards in two shrinkable columns"
    );
    let card = rule_body(".summary-candidate-card");
    assert!(
        card.contains("min-width: 0") && card.contains("overflow-wrap: anywhere"),
        "candidate dates and bounded facts must wrap within their own card"
    );

    let counts = rule_body(".summary-count-grid");
    assert!(
        counts.contains("grid-template-columns: repeat(3, minmax(0, 1fr))")
            && counts.contains("min-width: 0"),
        "the three availability meanings should remain equally visible at 320px"
    );

    let disclosure = rule_body(".summary-comment-disclosure > summary");
    assert!(
        disclosure.contains("min-height: 44px")
            && disclosure.contains("overflow-wrap: anywhere")
            && disclosure.contains("cursor: pointer"),
        "the native comment disclosure needs a wrapping mobile-sized keyboard target"
    );
    let comment = rule_body(".comment-preview");
    assert!(
        comment.contains("min-width: 0")
            && comment.contains("overflow-wrap: anywhere")
            && comment.contains("white-space: pre-wrap"),
        "plain-text comments must retain intentional line breaks without horizontal overflow"
    );
    assert!(
        rule_body(".organizer-recovery-form").contains("min-width: 0"),
        "the exceptional recovery path must also shrink at 320px"
    );

    let narrow = CSS
        .split_once("@media (max-width: 520px)")
        .map(|(_, rest)| rest)
        .expect("the stylesheet should define the existing narrow breakpoint");
    assert!(
        rule_body_in(narrow, ".summary-card-grid")
            .contains("grid-template-columns: minmax(0, 1fr)"),
        "candidate cards must stack into one column at the 320px layout"
    );
    assert!(
        rule_body(":focus-visible").contains("outline: 3px solid"),
        "native summary and recovery controls must retain visible keyboard focus"
    );
}

#[test]
fn response_matrix_confines_horizontal_overflow_to_a_named_keyboard_region() {
    let section = rule_body(".response-matrix-section");
    assert!(
        section.contains("width: 100%") && section.contains("min-width: 0"),
        "the matrix section must shrink with the organizer page"
    );

    let toggle = rule_body(".response-matrix-toggle");
    assert!(
        toggle.contains("min-height: 44px") && toggle.contains("overflow-wrap: anywhere"),
        "the explicit disclosure needs a wrapping mobile-sized target"
    );

    let scroll = rule_body(".response-matrix-scroll");
    assert!(
        scroll.contains("width: 100%")
            && scroll.contains("max-width: 100%")
            && scroll.contains("min-width: 0")
            && scroll.contains("overflow-x: auto")
            && !scroll.contains("overflow-y: auto"),
        "only the table region should scroll, without introducing nested vertical scrolling"
    );

    let table = rule_body(".response-matrix-table");
    assert!(
        table.contains("width: max-content") && table.contains("border-collapse: collapse"),
        "candidate columns should preserve a readable two-dimensional table"
    );
    let row_header = rule_body(".response-matrix-table th[scope=\"row\"]");
    assert!(
        row_header.contains("position: sticky")
            && row_header.contains("inset-inline-start: 0")
            && row_header.contains("overflow-wrap: anywhere")
            && !row_header.contains("text-overflow: ellipsis"),
        "only the respondent column stays visible, and long names must not be truncated"
    );

    assert!(
        rule_body(".response-matrix-scroll:focus-visible").contains("outline: 3px solid"),
        "keyboard users need a visible focus indicator on the local scroll region"
    );
}

#[test]
fn inline_calendar_and_post_answer_matrix_stay_inside_the_phone_viewport() {
    let calendar = rule_body(".candidate-calendar");
    assert!(
        calendar.contains("width: 100%") && calendar.contains("min-width: 0"),
        "the inline calendar must shrink with the creation form"
    );
    let grid = rule_body(".candidate-calendar-grid");
    assert!(
        grid.contains("grid-template-columns: repeat(7, minmax(0, 1fr))")
            && grid.contains("min-width: 0"),
        "the seven weekdays must share the available width without page overflow"
    );
    let day = rule_body(".candidate-calendar-day");
    assert!(
        day.contains("min-width: 0")
            && day.contains("min-height: 44px")
            && day.contains("overflow-wrap: anywhere"),
        "each calendar day must remain a shrinkable touch and keyboard target"
    );
    assert!(
        CSS.contains(".candidate-calendar-day[aria-pressed=\"true\"]")
            && rule_body(".candidate-calendar-day:focus-visible").contains("outline: 3px solid"),
        "selected and focused dates must have separate visible states"
    );

    let participant = rule_body(".participant-response-matrix");
    assert!(
        participant.contains("width: 100%")
            && participant.contains("min-width: 0")
            && participant.contains("overflow-wrap: anywhere"),
        "the post-answer heading and local table scroller must not widen the page"
    );
}

#[test]
fn organizer_decision_reflows_explicit_native_choices_at_three_hundred_twenty_pixels() {
    let section = rule_body(".organizer-decision-section");
    assert!(
        section.contains("width: 100%") && section.contains("min-width: 0"),
        "the decision surface must shrink with the organizer route"
    );
    assert!(
        rule_body(".organizer-decision-form").contains("min-width: 0"),
        "the native decision form must not impose page-level horizontal overflow"
    );

    let options = rule_body(".organizer-decision-options");
    assert!(
        options.contains("grid-template-columns: repeat(2, minmax(0, 1fr))")
            && options.contains("min-width: 0"),
        "desktop should compare authored candidates in two shrinkable columns"
    );
    let option = rule_body(".organizer-decision-option-label");
    assert!(
        option.contains("min-height: 44px") && option.contains("overflow-wrap: anywhere"),
        "each candidate needs a wrapping mobile-sized label target"
    );
    assert!(
        rule_body(".organizer-decision-radio:focus-visible + .organizer-decision-option-label")
            .contains("outline: 3px solid")
            && CSS.contains(".organizer-decision-radio:checked + .organizer-decision-option-label"),
        "native radio keyboard focus and checked state must both remain visible"
    );

    let selection = rule_body(".organizer-decision-selection");
    assert!(
        selection.contains("min-width: 0") && selection.contains("overflow-wrap: anywhere"),
        "the selected date context must wrap independently of the submit action"
    );
    let submit = rule_body(".organizer-decision-submit");
    assert!(
        submit.contains("width: 100%")
            && submit.contains("min-height: 44px")
            && submit.contains("overflow-wrap: anywhere"),
        "the irreversible action must remain a full-width wrapping target at 320px"
    );
    let result = rule_body(".organizer-decision-result");
    assert!(
        result.contains("width: 100%")
            && result.contains("min-width: 0")
            && result.contains("overflow-wrap: anywhere"),
        "the confirmed date must also fit the narrow organizer surface"
    );

    let narrow = CSS
        .split_once("@media (max-width: 520px)")
        .map(|(_, rest)| rest)
        .expect("the stylesheet should define the existing narrow breakpoint");
    assert!(
        rule_body_in(narrow, ".organizer-decision-options")
            .contains("grid-template-columns: minmax(0, 1fr)"),
        "candidate radios must stack into one column at the 320px layout"
    );
}

#[test]
fn decided_event_actions_use_two_columns_then_collapse_at_phone_width() {
    let actions = rule_body(".decided-event-actions-grid");
    assert!(
        actions.contains("grid-template-columns: repeat(2, minmax(0, 1fr))")
            && actions.contains("min-width: 0"),
        "decided-event actions need two shrinkable desktop columns"
    );

    for selector in [".calendar-download-link", ".decided-event-share-button"] {
        let control = rule_body(selector);
        assert!(
            control.contains("min-height: 44px") && control.contains("overflow-wrap: anywhere"),
            "{selector} needs a wrapping keyboard-sized target"
        );
    }

    let narrow = CSS
        .split_once("@media (max-width: 520px)")
        .map(|(_, rest)| rest)
        .expect("the stylesheet should define the existing narrow breakpoint");
    assert!(
        rule_body_in(narrow, ".decided-event-actions-grid")
            .contains("grid-template-columns: minmax(0, 1fr)"),
        "calendar and share actions must become one column at phone width"
    );
}
