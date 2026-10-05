//! The `html` kind: a page under `pages/`, a path like any file's, and no brief.

use crate::common::{assert_only_problem, asset_id, problems, project};
use scorsese_core::{
    Asset, AssetField as F, AssetKind, AssetProblem as E, GenerationState, PathProblem, Project,
    ProjectPath,
};

/// The fixture with one page added to its table.
fn with_page(path: &str) -> Project {
    let mut p = project();
    p.assets.push(Asset::imported(
        asset_id("page"),
        AssetKind::Html,
        ProjectPath::new(path),
    ));
    p
}

#[test]
fn a_page_under_pages_validates() {
    assert_eq!(problems(&with_page("pages/title.html")), vec![]);
}

/// The extension is what the browser and a reader go by, in any case.
#[test]
fn a_page_s_extension_may_be_upper_case() {
    assert_eq!(problems(&with_page("pages/Title.HTML")), vec![]);
}

#[test]
fn a_page_that_is_not_html_is_refused_by_name() {
    assert_only_problem(
        &with_page("pages/title.txt"),
        E::PageNotHtml {
            asset: asset_id("page"),
            path: ProjectPath::new("pages/title.txt"),
        },
    );
}

/// A bare `.html` is a hidden file with no name, not a page.
#[test]
fn an_extension_alone_is_not_a_page() {
    assert!(
        problems(&with_page(".html"))
            .iter()
            .any(|problem| problem.to_string().contains("must end in `.html`")),
    );
}

/// A page can never reach outside the project, so it still survives `scp -r`.
#[test]
fn a_page_obeys_the_project_path_rules() {
    assert_only_problem(
        &with_page("../elsewhere/title.html"),
        E::BadPath {
            asset: asset_id("page"),
            path: ProjectPath::new("../elsewhere/title.html"),
            problem: PathProblem::ParentEscape,
        },
    );
}

#[test]
fn a_page_needs_a_path() {
    let mut p = with_page("pages/title.html");
    p.assets.last_mut().expect("the page").path = None;
    assert_only_problem(
        &p,
        E::MissingField {
            asset: asset_id("page"),
            field: F::Path,
            kind: AssetKind::Html,
        },
    );
}

/// Authored, not generated: a lifecycle state on a page is a field nothing
/// reads.
#[test]
fn a_page_has_no_lifecycle() {
    let mut p = with_page("pages/title.html");
    p.assets.last_mut().expect("the page").state = Some(GenerationState::Sketch);
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("page"),
            field: F::State,
            kind: AssetKind::Html,
        },
    );
}

/// Only a page is held to the extension: an image named `.html` is some other
/// mistake, and this rule is not the one to report it.
#[test]
fn the_extension_rule_is_a_page_s_alone() {
    let mut p = project();
    p.assets.push(Asset::imported(
        asset_id("odd"),
        AssetKind::Image,
        ProjectPath::new("assets/odd.png"),
    ));
    assert_eq!(problems(&p), vec![]);
}

#[test]
fn a_page_is_visual_and_not_generated() {
    let kind = AssetKind::Html;
    assert!(kind.is_visual() && kind.is_file_backed());
    assert!(!kind.is_generated() && !kind.is_prompted() && !kind.is_synthesized());
    assert!(!kind.is_media() && !kind.is_still() && !kind.is_audible());
}
