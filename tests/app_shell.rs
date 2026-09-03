use dioxus::prelude::*;
use tsunoru::App;

#[test]
fn application_starts_with_anonymous_event_creation() {
    let html = dioxus_ssr::render_element(rsx! { App {} });

    assert!(
        html.contains("<h1>日程をつのる</h1>"),
        "the main heading should describe the first anonymous action: {html}"
    );
    assert!(
        html.contains("TSUNORU"),
        "the rendered UI should keep the product name visible: {html}"
    );
    assert!(
        html.contains("<main"),
        "the application shell should expose its primary landmark: {html}"
    );
    assert!(
        !html.contains("ログインしてください"),
        "event creation must not start with a login demand: {html}"
    );
}
