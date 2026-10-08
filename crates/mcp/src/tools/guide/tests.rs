use scorsese_core::guide;

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
