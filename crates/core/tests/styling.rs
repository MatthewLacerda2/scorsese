//! How a text asset says what it looks like: fonts, colours, and defaults.

mod common;

use common::{asset_id, document, project};
use scorsese_core::{FontChoice, Project, ProjectPath, Rgba, TextAlign, TextStyle};

#[test]
fn the_worked_example_carries_a_style() {
    let project = project();
    let title = project.asset(&asset_id("title")).expect("the title asset");
    let style = title.text_style();
    assert_eq!(style.font, FontChoice::Serif);
    assert_eq!(style.color, Rgba::opaque(0xf5, 0xf0, 0xe6));
    assert_eq!(style.size, 0.16);
    // Absent in the document, so it is the default rather than nothing.
    assert_eq!(style.align, TextAlign::Center);
    assert_eq!(style.line_height, TextStyle::DEFAULT_LINE_HEIGHT);
}

/// An asset with no `style` at all is not an asset with no appearance: it is
/// every default, so nothing downstream has to decide what a missing font
/// means.
#[test]
fn an_asset_without_a_style_is_the_default_style() {
    let bare = scorsese_core::Asset::text(asset_id("card"), "hello");
    assert_eq!(bare.style, None);
    assert_eq!(bare.text_style(), TextStyle::default());
    assert_eq!(TextStyle::default().font, FontChoice::Sans);
    assert_eq!(TextStyle::default().color, Rgba::WHITE);
}

#[test]
fn a_style_round_trips_through_the_document() {
    let before = project();
    let json = before.to_json().expect("serialises");
    let after = Project::from_json(&json).expect("parses back");
    assert_eq!(after, before, "a style survives a save and a load");
}

/// `sans` and `serif` are reserved; anything else is a path to a font file the
/// project brought with it, which is what keeps the shipped pair defaults
/// rather than the whole vocabulary.
#[test]
fn a_font_is_either_a_reserved_name_or_a_file() {
    let read = |written: &str| {
        let json = format!(r#""{written}""#);
        serde_json::from_str::<FontChoice>(&json).expect("a font name parses")
    };
    assert_eq!(read("sans"), FontChoice::Sans);
    assert_eq!(read("serif"), FontChoice::Serif);
    assert_eq!(
        read("assets/Inter-Regular.ttf"),
        FontChoice::File(ProjectPath::new("assets/Inter-Regular.ttf"))
    );
    for font in [
        FontChoice::Sans,
        FontChoice::Serif,
        FontChoice::File(ProjectPath::new("assets/Inter-Regular.ttf")),
    ] {
        let written = serde_json::to_string(&font).expect("serialises");
        assert_eq!(read(written.trim_matches('"')), font);
    }
}

/// Absent has to *stay* absent through a save. Writing `"weight": null` — or
/// worse, a number nobody chose — would turn "this file has one weight" into a
/// claim the document makes, and the next load would have to honour it.
#[test]
fn an_unweighted_style_says_nothing_about_weight() {
    assert_eq!(TextStyle::default().weight, None);
    let written = serde_json::to_string(&TextStyle::default()).expect("serialises");
    assert!(
        !written.contains("weight"),
        "an absent weight is absent from the document; got {written}"
    );

    let json = document(
        r##""assets": [{ "id": "t", "kind": "text", "text": "hi",
                         "style": { "font": "assets/Manrope[wght].ttf", "weight": 700 } }]"##,
    );
    let project = Project::from_json(&json).expect("parses");
    let style = project
        .asset(&asset_id("t"))
        .expect("the asset")
        .text_style();
    assert_eq!(style.weight, Some(700));
}

#[test]
fn a_colour_is_read_and_written_as_hex() {
    assert_eq!("#ffcc00".parse(), Ok(Rgba::opaque(255, 204, 0)));
    assert_eq!(
        "ffcc00".parse(),
        Ok(Rgba::opaque(255, 204, 0)),
        "# optional"
    );
    assert_eq!("#FFCC00".parse(), Ok(Rgba::opaque(255, 204, 0)), "any case");
    assert_eq!("#ffcc0080".parse(), Ok(Rgba::new(255, 204, 0, 128)));
    assert_eq!(Rgba::opaque(255, 204, 0).to_string(), "#ffcc00");
    assert_eq!(
        Rgba::new(255, 204, 0, 128).to_string(),
        "#ffcc0080",
        "alpha appears only when it is not solid"
    );
}

/// Refused at the parse rather than defaulted to black: a colour nobody can
/// read back is a title that silently comes out wrong.
#[test]
fn something_that_is_not_a_colour_fails_the_load() {
    for bad in ["#fff", "#gggggg", "red", "", "#ffcc00ff00"] {
        assert!(
            bad.parse::<Rgba>().is_err(),
            "`{bad}` should not parse as a colour"
        );
    }
    let json = document(
        r##""assets": [{ "id": "t", "kind": "text", "text": "hi",
                         "style": { "color": "octarine" } }]"##,
    );
    assert!(
        Project::from_json(&json).is_err(),
        "a colour nobody can read fails the load"
    );
}

#[test]
fn an_unknown_style_field_fails_the_load() {
    let json = document(
        r##""assets": [{ "id": "t", "kind": "text", "text": "hi",
                         "style": { "colour": "#ffffff" } }]"##,
    );
    assert!(
        Project::from_json(&json).is_err(),
        "a typo in a style key is a typo, not a field to drop"
    );
}
