//! A caption's `reveal` and `number` blocks, changed and taken away.

use crate::asset::AssetId;
use crate::authoring::fixture::project;
use crate::authoring::{AuthorError, BlockChange, CounterEdit, Edit, RevealEdit, set_asset};
use crate::project::Project;
use crate::text::{Locale, RevealUnit, TextStyle};

fn caption(project: &mut Project, edit: Edit) -> Result<Vec<String>, AuthorError> {
    set_asset(project, &AssetId::new("caption"), &edit)
}

fn reveal(fields: RevealEdit) -> Edit {
    Edit {
        reveal: Some(BlockChange::Merge(fields)),
        ..Edit::default()
    }
}

fn style(project: &Project) -> TextStyle {
    project
        .asset(&AssetId::new("caption"))
        .expect("the fixture has a caption")
        .text_style()
}

/// A caption with no block starts from the defaults a document's
/// `"reveal": {}` would, and the line names every field so the caller sees
/// what it got without asking.
#[test]
fn one_field_on_a_caption_without_a_block_starts_from_the_defaults() {
    let mut project = project();
    let said = caption(
        &mut project,
        reveal(RevealEdit {
            unit: Some(RevealUnit::Char),
            ..RevealEdit::default()
        }),
    )
    .expect("a typewriter");
    assert_eq!(said, vec!["reveal: none → by char, rise 0.2, stagger 0.5"]);
    assert_eq!(style(&project).font.name(), Some("serif"));
}

/// **The reason a block merges.** A rise and a stagger somebody tuned survive
/// "letter by letter instead of by word".
#[test]
fn changing_the_unit_keeps_the_rise_and_stagger_already_chosen() {
    let mut project = project();
    let tuned = RevealEdit {
        rise: Some(0.0),
        stagger: Some(1.0),
        ..RevealEdit::default()
    };
    caption(&mut project, reveal(tuned)).expect("a tuned reveal");
    let unit = RevealEdit {
        unit: Some(RevealUnit::Line),
        ..RevealEdit::default()
    };
    let said = caption(&mut project, reveal(unit)).expect("by line");
    assert_eq!(
        said,
        vec!["reveal: by word, rise 0, stagger 1 → by line, rise 0, stagger 1"]
    );
}

#[test]
fn a_block_can_be_taken_away() {
    let mut project = project();
    caption(&mut project, reveal(RevealEdit::default())).expect("a reveal");
    let said = caption(
        &mut project,
        Edit {
            reveal: Some(BlockChange::Remove),
            ..Edit::default()
        },
    )
    .expect("no reveal");
    assert_eq!(said, vec!["reveal: by word, rise 0.2, stagger 0.5 → none"]);
    assert_eq!(style(&project).reveal, None);
}

/// Validation is the document's, exactly as for `text_new`: a stagger past `1`
/// is refused and nothing is written.
#[test]
fn a_stagger_the_document_refuses_writes_nothing() {
    let mut project = project();
    let before = project.clone();
    let refused = caption(
        &mut project,
        reveal(RevealEdit {
            stagger: Some(1.5),
            ..RevealEdit::default()
        }),
    );
    assert!(matches!(refused, Err(AuthorError::Refused(_))));
    assert_eq!(project, before);
}

fn count_to(value: f64) -> Option<BlockChange<CounterEdit>> {
    Some(BlockChange::Merge(CounterEdit {
        value: Some(value),
        ..CounterEdit::default()
    }))
}

/// "DAWN" has nowhere to put a figure, so a number on it is refused the way a
/// hand-written document's would be.
#[test]
fn a_number_on_text_without_a_placeholder_is_refused() {
    let mut project = project();
    let before = project.clone();
    let refused = caption(
        &mut project,
        Edit {
            number: count_to(144.0),
            ..Edit::default()
        },
    );
    let problem = refused.expect_err("no `{n}` in DAWN");
    assert!(problem.to_string().contains("{n}"), "{problem}");
    assert_eq!(project, before);
}

/// Rewording and counting in one call is checked as the document it makes,
/// then the figure moves on its own and the locale chosen stays.
#[test]
fn a_reworded_caption_takes_a_number_and_keeps_its_locale_when_it_changes() {
    let mut project = project();
    let said = caption(
        &mut project,
        Edit {
            text: Some("{n} partitions".to_owned()),
            number: Some(BlockChange::Merge(CounterEdit {
                value: Some(140.0),
                locale: Some(Locale::PtBr),
                ..CounterEdit::default()
            })),
            ..Edit::default()
        },
    )
    .expect("a counting caption");
    assert_eq!(said[1], "number: none → 140, 0 decimals, pt-BR, grouped");
    let said = caption(
        &mut project,
        Edit {
            number: count_to(144.0),
            ..Edit::default()
        },
    )
    .expect("count to 144");
    assert_eq!(
        said,
        vec!["number: 140, 0 decimals, pt-BR, grouped → 144, 0 decimals, pt-BR, grouped"]
    );
}

#[test]
fn a_block_on_a_kind_that_has_no_text_is_refused_by_name() {
    let mut project = project();
    let refused = set_asset(
        &mut project,
        &AssetId::new("card"),
        &reveal(RevealEdit::default()),
    );
    assert!(matches!(
        refused,
        Err(AuthorError::NotOnKind {
            field: "reveal",
            ..
        })
    ));
}
