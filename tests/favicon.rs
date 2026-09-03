#[test]
fn application_declares_a_bundled_tsunoru_favicon() {
    let app_source = std::fs::read_to_string("src/lib.rs").expect("read the application shell");

    assert!(
        app_source.contains("const FAVICON: Asset = asset!(\"/assets/favicon.png\");"),
        "the favicon should pass through the Dioxus asset pipeline"
    );
    assert!(
        app_source.contains("document::Link { rel: \"icon\", href: FAVICON }"),
        "the shared document head should declare the favicon"
    );
}

#[test]
fn favicon_is_a_64_pixel_rgb_png() {
    let png = std::fs::read("assets/favicon.png").expect("read the checked-in favicon");

    assert!(png.len() >= 26, "the favicon should contain a PNG header");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "the favicon should be PNG");
    assert_eq!(&png[12..16], b"IHDR", "the first PNG chunk should be IHDR");
    assert_eq!(
        u32::from_be_bytes(png[16..20].try_into().expect("PNG width")),
        64,
        "the favicon should be 64 pixels wide"
    );
    assert_eq!(
        u32::from_be_bytes(png[20..24].try_into().expect("PNG height")),
        64,
        "the favicon should be 64 pixels high"
    );
    assert_eq!(png[24], 8, "the favicon should use 8-bit channels");
    assert_eq!(png[25], 2, "the favicon should use opaque RGB color");
}
