#[test]
fn browser_document_declares_japanese_content() {
    let html = std::fs::read_to_string("index.html")
        .expect("the Dioxus web shell should be checked into the repository");

    assert!(
        html.contains("<html lang=\"ja\">"),
        "assistive technology should receive the page language: {html}"
    );
}

#[test]
fn public_event_route_declares_itself_outside_search_discovery() {
    let ui_source = std::fs::read_to_string("src/ui.rs").expect("read the route components");

    assert!(
        ui_source.contains("document::Meta")
            && ui_source.contains("name: \"robots\"")
            && ui_source.contains("noindex, nofollow"),
        "the public-by-link surface should add a route-scoped robots directive"
    );
}
