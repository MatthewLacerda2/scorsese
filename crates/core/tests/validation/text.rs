//! What a `text` asset must and must not carry: its content, and the style it
//! is drawn in.

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::{
    AssetField as F, AssetKind, AssetProblem as E, FontChoice, PathProblem, ProjectPath, TextStyle,
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
