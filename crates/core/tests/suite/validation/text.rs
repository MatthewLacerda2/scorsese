//! What a `text` asset must and must not carry: its content, and the style it
//! is drawn in.

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::{
    AssetField as F, AssetKind, AssetProblem as E, FontChoice, PathProblem, ProjectPath, Rgba,
    TextStyle,
};

#[test]
fn a_text_asset_needs_its_content() {
    let mut p = project();
    asset_mut(&mut p, "title").text = None;
    assert_only_problem(
        &p,
        E::MissingText {
            asset: asset_id("title"),
        },
    );
}

#[test]
fn nothing_but_a_text_asset_may_carry_text() {
    let mut p = project();
    asset_mut(&mut p, "logo").text = Some("stray".to_owned());
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("logo"),
            field: F::Text,
            kind: AssetKind::Image,
        },
    );
}

#[test]
fn nothing_but_a_text_asset_may_carry_a_style() {
    let mut p = project();
    asset_mut(&mut p, "logo").style = Some(TextStyle::default());
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("logo"),
            field: F::Style,
            kind: AssetKind::Image,
        },
    );
}

/// A font is a path like every other path in the document, so it obeys the
/// rule that keeps a project portable — caught here rather than at the render,
/// which is minutes later and on someone else's machine.
#[test]
fn a_font_outside_the_project_is_refused() {
    let path = ProjectPath::new("/usr/share/fonts/Helvetica.ttf");
    let mut p = project();
    asset_mut(&mut p, "title").style = Some(TextStyle {
        font: FontChoice::File(path.clone()),
        ..TextStyle::default()
    });
    assert_only_problem(
        &p,
        E::BadFontPath {
            asset: asset_id("title"),
            path,
            problem: PathProblem::Absolute,
        },
    );
}

#[test]
fn a_font_inside_the_project_is_fine() {
    let mut p = project();
    asset_mut(&mut p, "title").style = Some(TextStyle {
        font: FontChoice::File(ProjectPath::new("assets/Inter-Regular.ttf")),
        ..TextStyle::default()
    });
    assert_eq!(problems(&p), Vec::new());
}

/// A rim colour with no thickness behind it is refused rather than ignored.
/// The two readings — *I meant no edge* and *I got the width wrong* — look the
/// same in the frame, and a caption is the worst place to guess: the rim's
/// whole job is to be there over footage nobody has looked at yet.
#[test]
fn a_stroke_with_no_width_is_refused() {
    for width in [0.0, -0.01] {
        let mut p = project();
        asset_mut(&mut p, "title").style = Some(styled(width));
        assert_only_problem(
            &p,
            E::StrokeWithoutWidth {
                asset: asset_id("title"),
                width,
            },
        );
    }
    // A NaN is the third way of writing a thickness nothing could draw, and it
    // needs its own arm because no two NaNs compare equal.
    let mut p = project();
    asset_mut(&mut p, "title").style = Some(styled(f64::NAN));
    let found = problems(&p);
    assert_eq!(found.len(), 1);
    assert!(found[0].to_string().contains("nothing would be drawn"));
}

/// A rim in black, at whatever thickness the case under test is about.
fn styled(stroke_width: f64) -> TextStyle {
    TextStyle {
        stroke: Some(Rgba::opaque(0, 0, 0)),
        stroke_width,
        ..TextStyle::default()
    }
}

/// The width alone is not a problem: it sits at its default on every text
/// asset ever written, and nothing reads it until a colour arrives.
#[test]
fn a_width_with_no_stroke_beside_it_is_nothing_to_report() {
    let mut p = project();
    asset_mut(&mut p, "title").style = Some(TextStyle {
        stroke: None,
        stroke_width: 0.0,
        ..TextStyle::default()
    });
    assert_eq!(problems(&p), Vec::new());
}
