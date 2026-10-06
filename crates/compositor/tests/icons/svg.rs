//! Every icon written out as the SVG a page is served (#838).
//!
//! A page and an `icon` asset must draw the same symbol, so the document is
//! held to the contours rather than to upstream's file: `minus` is upstream's
//! `M5 12h14`, which the blob carries as a move and a line.

use scorsese_compositor::icon;

#[test]
fn every_icon_writes_a_document() {
    for icon in icon::all() {
        let svg = icon
            .svg()
            .unwrap_or_else(|| panic!("'{}' writes an SVG", icon.name()));
        assert!(
            svg.contains("<path d=\"M"),
            "'{}' draws something",
            icon.name()
        );
    }
}

#[test]
fn a_document_is_the_contours_in_lucides_vocabulary() {
    let svg = icon::find("minus")
        .and_then(icon::Icon::svg)
        .expect("minus");
    assert_eq!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" \
         viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" \
         stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M5 12L19 12\"/></svg>\n"
    );
}
