//! Path rules — the invariant that lets a project survive `scp -r`.

use crate::common::{assert_only_problem, asset_id, asset_mut, project};
use scorsese_core::{AssetProblem as E, PathProblem, ProjectPath, ValidationError};

/// Puts `path` on the logo asset and expects exactly that one complaint.
#[track_caller]
fn expect_problem(path: &str, problem: PathProblem) {
    let mut p = project();
    let path = ProjectPath::new(path);
    asset_mut(&mut p, "logo").path = Some(path.clone());
    assert_only_problem(
        &p,
        E::BadPath {
            asset: asset_id("logo"),
            path,
            problem,
        },
    );
}

#[test]
fn an_absolute_path_is_refused() {
    expect_problem("/home/matthew/logo.png", PathProblem::Absolute);
}

#[test]
fn a_windows_drive_path_is_absolute_on_every_platform() {
    // Backslashes are their own problem, so this uses forward slashes: the
    // point is that `C:` alone already makes the path machine-specific.
    expect_problem("C:/media/logo.png", PathProblem::Absolute);
}

#[test]
fn a_unc_path_is_absolute() {
    expect_problem("\\\\host\\share\\logo.png", PathProblem::Backslash);
}

#[test]
fn a_path_escaping_the_project_is_refused() {
    expect_problem("assets/../../elsewhere/logo.png", PathProblem::ParentEscape);
}

#[test]
fn a_backslash_path_is_refused() {
    expect_problem("assets\\logo.png", PathProblem::Backslash);
}

#[test]
fn an_empty_path_is_refused() {
    expect_problem("", PathProblem::Empty);
}

/// The script is the document's own path and names no asset, so it gets its
/// own complaint — but the rule it breaks is the same rule.
#[test]
fn the_script_obeys_the_rules_every_other_path_obeys() {
    let mut p = project();
    let path = ProjectPath::new("../notes/script.md");
    p.script = Some(path.clone());
    assert_only_problem(
        &p,
        ValidationError::BadScriptPath {
            path,
            problem: PathProblem::ParentEscape,
        },
    );
}

/// Whether the file is really there is `scorsese check`'s question and only
/// ever a warning: a project that has lost its brief should still render.
#[test]
fn a_script_naming_a_file_nobody_has_written_is_still_valid() {
    let mut p = project();
    p.script = Some(ProjectPath::new("script.md"));
    assert_eq!(p.validate(), Ok(()));
}

#[test]
fn a_relative_path_resolves_under_the_project_root() {
    let root = std::path::Path::new("/tmp/demo.scor");
    let resolved = ProjectPath::new("assets/logo.png").resolve(root);
    assert_eq!(resolved, root.join("assets").join("logo.png"));
}
