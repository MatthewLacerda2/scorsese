use scorsese_core::style::STYLES;
use scorsese_guide as guide;

use super::Read;
use crate::tools::Tool;

/// Every guide is a choice in the schema and is named in the description, so
/// a guide added to the table is one a client can find.
#[test]
fn every_guide_is_offered_and_described() {
    let schema = Read.schema();
    let offered = schema["properties"]["name"]["enum"]
        .as_array()
        .expect("`name` lists the guides");
    assert_eq!(offered.len(), guide::names().count());
    for name in guide::names() {
        assert!(offered.iter().any(|offer| offer == name), "{name}");
        assert!(
            Read.description().contains(&format!("{name}:")),
            "the description says what `{name}` is"
        );
    }
}

#[test]
fn a_section_may_be_sent_as_a_bare_number() {
    let by_number = Read
        .call(&serde_json::json!({ "project": "x.scor", "name": "pages", "section": 1 }))
        .unwrap();
    assert!(by_number.parts[0].text.starts_with("## The contract"));
}

/// The style library's recipes are read through the guide, in any project
/// (#1531): a style's id or a placement's is a section of `guide styles`.
#[test]
fn a_style_id_is_a_section_of_the_styles_guide() {
    let style = &STYLES[0];
    let reply = Read
        .call(&serde_json::json!({ "project": "x.scor", "name": "styles", "section": style.id }))
        .unwrap();
    assert!(reply.parts[0].text.contains(style.prompt));
    let reply = Read
        .call(&serde_json::json!({ "project": "x.scor", "name": "styles", "section": "tiktok_ad" }))
        .unwrap();
    assert!(reply.parts[0].text.contains("1080x1920"));
}

/// A style's prompt sends the editor to a guide's section by its words, so the
/// words must find exactly one section of that guide. It lives here, beside
/// the tool that serves the guide, because this crate sees both: the guides
/// left `core` for a leaf crate of their own (#1129), and the leaf sees nothing.
#[test]
fn every_section_a_prompt_names_is_one_the_guide_finds() {
    for style in STYLES {
        for name in guide::names() {
            let pointer = format!("`guide {name}`, section \"");
            for (at, _) in style.prompt.match_indices(&pointer) {
                let rest = &style.prompt[at + pointer.len()..];
                let words = &rest[..rest.find('"').expect("a closing quote")];
                let found = guide::read(name, Some(words));
                assert!(
                    found.is_ok(),
                    "{} names {name} \"{words}\": {found:?}",
                    style.id
                );
            }
        }
    }
}
