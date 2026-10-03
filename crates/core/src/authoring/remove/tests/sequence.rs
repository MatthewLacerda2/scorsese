//! A still an image sequence plays is not a clip, so naming clips never takes
//! it — the sequence has to let go of it first.

use std::collections::BTreeSet;

use super::super::remove_asset;
use super::{named, project};
use crate::asset::{Asset, AssetId, AssetKind, ImageSequence};
use crate::authoring::AuthorError;
use crate::path::ProjectPath;
use crate::project::Project;
use crate::time::Frames;
use crate::timeline::ClipId;

/// The fixture, plus two imported frames, a sequence playing both, and a
/// clip on `v1` that shows the first frame on its own as well.
fn sequenced() -> Project {
    let mut project = project();
    for n in 1..=2 {
        project.assets.push(Asset::imported(
            AssetId::new(format!("f{n}")),
            AssetKind::Image,
            ProjectPath::new(format!("assets/spin/{n}.png")),
        ));
    }
    let stills = vec![AssetId::new("f1"), AssetId::new("f2")];
    project.assets.push(Asset::image_sequence(
        AssetId::new("spin"),
        ImageSequence::new(stills),
    ));
    let mut alone = project.tracks[0].clips[0].clone();
    alone.id = ClipId::new("alone");
    alone.asset = AssetId::new("f1");
    alone.start = Frames(120);
    project.tracks[0].clips.push(alone);
    project.validate().expect("the fixture is a valid project");
    project
}

#[test]
fn a_still_a_sequence_plays_is_refused_whatever_is_named() {
    let mut project = sequenced();
    let before = project.clone();
    for clips in [named(&[]), named(&["alone"])] {
        let error =
            remove_asset(&mut project, &AssetId::new("f1"), &clips).expect_err("`spin` plays it");
        let AuthorError::PlayedBySequence { sequences, .. } = &error else {
            panic!("got {error}");
        };
        assert_eq!(sequences, &vec![AssetId::new("spin")]);
        assert!(error.to_string().contains("`spin`"), "{error}");
        assert_eq!(project, before, "a refusal changes nothing");
    }
}

/// Once the sequence is gone, the still is an ordinary asset again.
#[test]
fn the_still_goes_once_the_sequence_has() {
    let mut project = sequenced();
    remove_asset(&mut project, &AssetId::new("spin"), &BTreeSet::new()).expect("unplaced");
    let removal = remove_asset(&mut project, &AssetId::new("f1"), &named(&["alone"]))
        .expect("nothing plays it now");
    assert_eq!(removal.clips.len(), 1);
    assert_eq!(removal.clips[0].clip.id.as_str(), "alone");
}
