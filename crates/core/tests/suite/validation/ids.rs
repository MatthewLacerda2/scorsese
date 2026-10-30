//! What an asset id may hold: it becomes a file and a link name, so only the
//! characters a file name keeps everywhere.

use crate::common::{assert_only_problem, asset_id, project};
use scorsese_core::{
    Asset, AssetId, AssetIdProblem, AssetProblem as E, MAX_ASSET_ID_LENGTH, ValidationError,
};

/// Adds a text asset called `id`, which nothing refers to, and expects exactly
/// that one complaint about it.
#[track_caller]
fn expect_problem(id: &str, problem: AssetIdProblem) {
    let mut p = project();
    p.assets.push(Asset::text(asset_id(id), "Hello"));
    assert_only_problem(
        &p,
        E::BadAssetId {
            id: asset_id(id),
            problem,
        },
    );
}

#[test]
fn an_id_of_letters_digits_dashes_underscores_and_dots_is_valid() {
    for id in ["logo", "Shot_2", "vo-open", "v1.final", "a", "9"] {
        assert_eq!(AssetId::new(id).check(), Ok(()), "{id}");
    }
    let longest = "a".repeat(MAX_ASSET_ID_LENGTH);
    assert_eq!(AssetId::new(longest).check(), Ok(()));
}

#[test]
fn a_slash_a_space_or_a_letter_outside_ascii_is_refused() {
    expect_problem("shots/one", AssetIdProblem::Character('/'));
    expect_problem("shot one", AssetIdProblem::Character(' '));
    expect_problem("praça", AssetIdProblem::Character('ç'));
    expect_problem("a\\b", AssetIdProblem::Character('\\'));
}

#[test]
fn an_id_starting_with_a_dot_is_refused_and_so_is_a_climb() {
    expect_problem(".hidden", AssetIdProblem::LeadingDot);
    expect_problem("..", AssetIdProblem::LeadingDot);
    expect_problem("../../escaped", AssetIdProblem::LeadingDot);
}

#[test]
fn an_empty_id_is_refused() {
    expect_problem("", AssetIdProblem::Empty);
}

#[test]
fn an_id_longer_than_the_cap_is_refused() {
    let long = "a".repeat(MAX_ASSET_ID_LENGTH + 1);
    expect_problem(&long, AssetIdProblem::TooLong);
}

#[test]
fn the_refusal_says_what_is_wrong_in_plain_words() {
    let problem = ValidationError::from(E::BadAssetId {
        id: asset_id("shots/one"),
        problem: AssetIdProblem::Character('/'),
    });
    assert_eq!(
        problem.to_string(),
        "asset id `shots/one` contains `/`: an asset id holds only ASCII letters, digits, \
         `-`, `_` and `.`"
    );
}

/// An id is a file name, so a bad one keeps a document out of a folder just as
/// an escaping path does.
#[test]
fn a_bad_id_is_a_path_problem() {
    let mut p = project();
    p.assets.push(Asset::text(asset_id("a/b"), "Hello"));
    let problems = p.check_paths().expect_err("refused").into_vec();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].is_path_problem());
}
