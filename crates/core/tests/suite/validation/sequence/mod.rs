//! What an image sequence plays, and the stills it cannot.

mod stills;

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::{
    Asset, AssetField, AssetKind, AssetProblem, Frames, ImageSequence, Project, ProjectPath,
    SequenceProblem as S,
};

/// The fixture with three imported frames and a sequence playing them.
pub(crate) fn sequenced(edit: impl FnOnce(&mut ImageSequence)) -> Project {
    let mut p = project();
    for n in 1..=3 {
        let (id, path) = (asset_id(&format!("f{n}")), format!("assets/spin/{n}.png"));
        p.assets.push(Asset::imported(
            id,
            AssetKind::Image,
            ProjectPath::new(path),
        ));
    }
    let mut sequence = ImageSequence::new(["f1", "f2", "f3"].map(asset_id).into());
    edit(&mut sequence);
    p.assets
        .push(Asset::image_sequence(asset_id("spin"), sequence));
    p
}

#[test]
fn a_run_of_imported_stills_is_a_sequence() {
    assert_eq!(problems(&sequenced(|_| {})), vec![]);
    let looped = sequenced(|s| {
        s.hold = Frames(4);
        s.looping = true;
        s.stills.push(asset_id("f1"));
    });
    assert_eq!(problems(&looped), vec![]);
}

#[test]
fn a_sequence_needs_its_block_and_nothing_else_may_carry_one() {
    let mut p = sequenced(|_| {});
    asset_mut(&mut p, "spin").sequence = None;
    assert_only_problem(
        &p,
        AssetProblem::MissingField {
            asset: asset_id("spin"),
            field: AssetField::Sequence,
            kind: AssetKind::ImageSequence,
        },
    );
    let mut p = sequenced(|_| {});
    asset_mut(&mut p, "logo").sequence = Some(ImageSequence::new(vec![asset_id("f1")]));
    assert_only_problem(
        &p,
        AssetProblem::StrayField {
            asset: asset_id("logo"),
            field: AssetField::Sequence,
            kind: AssetKind::Image,
        },
    );
}

#[test]
fn no_stills_and_no_hold_are_both_refused() {
    let p = sequenced(|s| {
        s.stills.clear();
        s.hold = Frames(0);
    });
    let spin = asset_id("spin");
    assert_eq!(
        problems(&p),
        vec![
            S::NoStills {
                asset: spin.clone()
            }
            .into(),
            S::NoHold { asset: spin }.into()
        ]
    );
}
