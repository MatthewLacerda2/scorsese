//! What a `weight` can be refused for without opening a font file.
//!
//! Two things only, and the boundary is the point. A reserved name is a face
//! scorsese ships and knows to be static, and OpenType's own `wght` range is a
//! fact about the format — both readable from the document alone. Whether a
//! *file* is variable, and how far its axis runs, is a fact about bytes on
//! disk and belongs to the render, exactly as "is the media actually there"
//! does for `path`.

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::{
    AssetProblem as E, FontChoice, MAX_WEIGHT, MIN_WEIGHT, ProjectPath, TextStyle,
};

fn styled(project: &mut scorsese_core::Project, font: FontChoice, weight: Option<u16>) {
    asset_mut(project, "title").style = Some(TextStyle {
        font,
        weight,
        ..TextStyle::default()
    });
}

fn own_font() -> FontChoice {
    FontChoice::File(ProjectPath::new("assets/Manrope[wght].ttf"))
}

/// The shipped faces are variable, so a weight beside one is an ordinary
/// request and not a mistake. This is the compatibility rule stated as a test:
/// it used to be the one refusal a document could earn without a font file
/// being opened, and every project that ever wrote `"font": "sans"` depends on
/// the other half of it — an unweighted reserved name is still fine.
#[test]
fn a_weight_beside_a_reserved_name_is_accepted() {
    let mut p = project();
    styled(&mut p, FontChoice::Sans, Some(700));
    assert!(problems(&p).is_empty(), "sans reaches 700");
}

#[test]
fn the_serif_name_takes_a_weight_too() {
    let mut p = project();
    styled(&mut p, FontChoice::Serif, Some(400));
    assert!(problems(&p).is_empty(), "serif reaches 400");
}

/// Whether a *particular* face reaches a weight is a fact about bytes and is
/// refused at the render. What the document alone can still say is that a
/// number is not a weight on anybody's scale, and it still says it — for a
/// reserved name as much as for a file.
#[test]
fn a_number_off_every_scale_is_still_refused_beside_a_reserved_name() {
    let mut p = project();
    styled(&mut p, FontChoice::Sans, Some(1200));
    assert_eq!(problems(&p).len(), 1, "1200 is not a weight");
}

/// The bounds are the format's, so this needs no file. A number outside them
/// is not a weight for any face that could ever be pointed at.
#[test]
fn a_number_that_is_not_a_weight_at_all_is_refused() {
    for absurd in [0, MAX_WEIGHT + 1, u16::MAX] {
        let mut p = project();
        styled(&mut p, own_font(), Some(absurd));
        assert_only_problem(
            &p,
            E::WeightOutOfRange {
                asset: asset_id("title"),
                weight: absurd,
                min: MIN_WEIGHT,
                max: MAX_WEIGHT,
            },
        );
    }
}

/// The whole legal span passes here even though no single file offers all of
/// it — narrowing to what a face actually reaches is the render's answer,
/// because only the file knows.
#[test]
fn every_weight_the_format_allows_passes_validation() {
    for weight in [MIN_WEIGHT, 200, 400, 700, MAX_WEIGHT] {
        let mut p = project();
        styled(&mut p, own_font(), Some(weight));
        assert_eq!(problems(&p), Vec::new(), "weight {weight} is a weight");
    }
}

/// Absent is not a validation problem: a static file needs nothing said, and
/// the refusal for a variable one cannot be made without reading it.
#[test]
fn saying_nothing_about_weight_is_not_a_document_problem() {
    let mut p = project();
    styled(&mut p, own_font(), None);
    assert_eq!(problems(&p), Vec::new());
    assert_eq!(MIN_WEIGHT, 1, "the OpenType floor");
    assert_eq!(MAX_WEIGHT, 1000, "the OpenType ceiling");
}
