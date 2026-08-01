//! What a `weight` can be refused for without opening a font file.
//!
//! Two things only, and the boundary is the point. A reserved name is a face
//! scorsese ships and knows to be static, and OpenType's own `wght` range is a
//! fact about the format — both readable from the document alone. Whether a
//! *file* is variable, and how far its axis runs, is a fact about bytes on
//! disk and belongs to the render, exactly as "is the media actually there"
//! does for `path`.

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::AssetProblem as E;
use scorsese_core::{FontChoice, MAX_WEIGHT, MIN_WEIGHT, ProjectPath, TextStyle};

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

/// The shipped faces are one static weight each. Refusing says so and points
/// at the only way to get a weight at all; ignoring it would look exactly like
/// honouring it.
#[test]
fn a_weight_beside_a_reserved_name_is_refused() {
    let mut p = project();
    styled(&mut p, FontChoice::Sans, Some(700));
    assert_only_problem(
        &p,
        E::WeightOnReservedFont {
            asset: asset_id("title"),
            font: "sans".to_owned(),
            weight: 700,
        },
    );
}

#[test]
fn the_serif_name_is_refused_a_weight_too() {
    let mut p = project();
    styled(&mut p, FontChoice::Serif, Some(400));
    assert_eq!(problems(&p).len(), 1, "the reserved name is the problem");
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
