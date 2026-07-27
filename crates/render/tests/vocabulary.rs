//! The animatable properties this build publishes, against the page that
//! promises them.
//!
//! The compositor and the mixer each publish what they animate, next to the
//! code that implements it. `docs/project-format.md` carries the same list in
//! prose, for the agent authoring a keyframe track — and nothing but habit was
//! keeping the two the same. A property the code animates and the page never
//! mentions is a capability nobody can reach; a row for a property nothing
//! animates is an instruction that silently does nothing.
//!
//! This crate is where the two vocabularies meet, so it is where they can be
//! compared to the page.

use scorsese_render::ANIMATABLE;

const DOC: &str = include_str!("../../../docs/project-format.md");

/// The heading of the table, matched literally: if it is renamed, this test
/// says so rather than quietly finding no rows and passing.
const TABLE: &str = "| path | means |";

/// The first column of the animatable-properties table.
fn documented() -> Vec<String> {
    let table = DOC
        .split_once(TABLE)
        .unwrap_or_else(|| panic!("no animatable-properties table (`{TABLE}`) in the page"))
        .1;
    table
        .lines()
        // What is left of the header row, then the `| --- |` separator.
        .skip(1)
        .skip_while(|line| line.starts_with("| ---"))
        .take_while(|line| line.starts_with('|'))
        .filter_map(|line| line.split('|').nth(1))
        .map(|cell| cell.trim().trim_matches('`').to_owned())
        .collect()
}

#[test]
fn every_animatable_property_is_documented() {
    let documented = documented();
    assert!(!documented.is_empty(), "the table parsed as no rows at all");
    for property in ANIMATABLE.properties() {
        assert!(
            documented.iter().any(|path| path == property.path),
            "`{}` ({}) is animatable but is not in the animatable-properties \
             table in `docs/project-format.md`",
            property.path,
            property.describes
        );
    }
}

#[test]
fn the_table_documents_nothing_imaginary() {
    for path in documented() {
        assert!(
            ANIMATABLE
                .properties()
                .any(|property| property.path == path),
            "`docs/project-format.md` lists `{path}` as animatable, but nothing \
             in this build animates it"
        );
    }
}
