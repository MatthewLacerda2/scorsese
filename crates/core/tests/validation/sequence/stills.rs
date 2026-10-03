//! Which assets a sequence's stills may be: imported pictures, one format,
//! one size.

use super::sequenced;
use crate::common::{assert_only_problem, asset_id, asset_mut, problems, reports};
use scorsese_core::{AssetKind, MediaMetadata, ProjectPath, SequenceProblem as S};

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
fn a_still_that_can_hold_an_animation_is_not_a_frame() {
    let mut p = sequenced(|_| {});
    for id in ["f1", "f2", "f3"] {
        asset_mut(&mut p, id).path = Some(ProjectPath::new(format!("assets/{id}.gif")));
    }
    assert_eq!(problems(&p).len(), 3, "one finding per still");
    assert!(reports(
        &p,
        S::NotOnePicture {
            asset: asset_id("spin"),
            still: asset_id("f1"),
            format: "gif".to_owned(),
        }
    ));
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
