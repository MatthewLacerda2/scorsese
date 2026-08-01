//! The field vocabulary the two collapsed errors carry.
//!
//! Both of them put a field name into a sentence, so what a field calls itself
//! is not decoration — it is the difference between an agent repairing the
//! right key and hunting for one that is not in the document.

use scorsese_core::{AssetField, AssetId, AssetKind, AssetProblem as E};

/// Every field spells itself the way `project.json` does. A rename here is a
/// message pointing at a key nobody can find.
#[test]
fn a_field_is_named_as_the_document_writes_it() {
    let named = [
        (AssetField::Path, "path"),
        (AssetField::Text, "text"),
        (AssetField::Style, "style"),
        (AssetField::Prompt, "prompt"),
        (AssetField::Recipe, "recipe"),
        (AssetField::State, "state"),
        (AssetField::Color, "color"),
    ];
    for (field, name) in named {
        assert_eq!(field.to_string(), name);
    }
}

#[test]
fn a_missing_field_says_which_one_and_why() {
    let problem = E::MissingField {
        asset: AssetId::new("shot-city"),
        field: AssetField::Prompt,
        kind: AssetKind::GeneratedVideo,
    };
    assert_eq!(
        problem.to_string(),
        "asset `shot-city` is a GeneratedVideo and needs a `prompt`"
    );
}

#[test]
fn a_stray_field_says_which_one_and_where_it_does_not_belong() {
    let problem = E::StrayField {
        asset: AssetId::new("logo"),
        field: AssetField::Recipe,
        kind: AssetKind::Image,
    };
    assert_eq!(
        problem.to_string(),
        "asset `logo` is a Image, so `recipe` does not apply to it"
    );
}
