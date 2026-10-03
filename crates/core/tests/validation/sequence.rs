//! What an image sequence plays, and the stills it cannot.

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::{
    Asset, AssetField, AssetKind, AssetProblem, Frames, ImageSequence, MediaMetadata, Project,
    ProjectPath, SequenceProblem as S,
};

/// The fixture with three imported frames and a sequence playing them.
fn sequenced(edit: impl FnOnce(&mut ImageSequence)) -> Project {
    let mut p = project();
    for n in 1..=3 {
        let path = ProjectPath::new(format!("assets/spin/{n:04}.png"));
        p.assets.push(Asset::imported(
            asset_id(&format!("f{n}")),
            AssetKind::Image,
            path,
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

#[test]
fn every_still_is_an_imported_picture_in_the_table() {
    let p = sequenced(|s| s.stills.push(asset_id("ghost")));
    assert_only_problem(
        &p,
        S::UnknownStill {
            asset: asset_id("spin"),
            still: asset_id("ghost"),
        },
    );
    let p = sequenced(|s| s.stills.push(asset_id("title")));
    assert_only_problem(
        &p,
        S::NotAnImage {
            asset: asset_id("spin"),
            still: asset_id("title"),
            kind: AssetKind::Text,
        },
    );
}

#[test]
fn a_still_in_another_format_is_named_once() {
    let mut p = sequenced(|_| {});
    for id in ["f2", "f3"] {
        asset_mut(&mut p, id).path = Some(ProjectPath::new(format!("assets/spin/{id}.jpg")));
    }
    assert_only_problem(
        &p,
        S::MixedFormats {
            asset: asset_id("spin"),
            still: asset_id("f2"),
            expected: "png".to_owned(),
            found: "jpg".to_owned(),
        },
    );
    // One decoder reads both spellings, so they are one format.
    let mut p = sequenced(|_| {});
    asset_mut(&mut p, "f1").path = Some(ProjectPath::new("assets/spin/a.JPEG"));
    asset_mut(&mut p, "f2").path = Some(ProjectPath::new("assets/spin/b.jpg"));
    asset_mut(&mut p, "f3").path = Some(ProjectPath::new("assets/spin/c.jpeg"));
    assert_eq!(problems(&p), vec![]);
}

#[test]
fn stills_of_two_measured_sizes_are_refused_and_unmeasured_ones_are_not() {
    let sized = |width, height| MediaMetadata {
        width: Some(width),
        height: Some(height),
        ..MediaMetadata::default()
    };
    let mut p = sequenced(|_| {});
    asset_mut(&mut p, "f1").media = Some(sized(64, 36));
    asset_mut(&mut p, "f3").media = Some(sized(36, 64));
    assert_only_problem(
        &p,
        S::MixedSizes {
            asset: asset_id("spin"),
            still: asset_id("f3"),
            width: 64,
            height: 36,
            found_width: 36,
            found_height: 64,
        },
    );
}
