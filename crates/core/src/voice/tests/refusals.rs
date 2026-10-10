//! What a cut refuses, and that a refusal changes nothing.

use super::{project, voicing};
use crate::voice::{VoiceError, cut_to_voice};
use crate::{Asset, AssetId, AssetKind, ClipId, MediaMetadata, ProjectPath};

#[test]
fn a_media_backed_visual_that_would_outrun_its_source_is_refused() {
    let (dir, mut project) = project("outrun");
    let mut shot = Asset::imported(
        AssetId::new("shot"),
        AssetKind::Video,
        ProjectPath::new("assets/shot.mp4"),
    );
    shot.media = Some(MediaMetadata {
        duration_seconds: Some(2.0),
        ..MediaMetadata::default()
    });
    project.assets.push(shot);
    project.tracks[0].clips[2].asset = AssetId::new("shot");
    let before = project.clone();
    // Scene 3 is 66 frames long now, and the shot has 60.
    let refused = cut_to_voice(&mut project, &dir, &voicing()).unwrap_err();
    assert!(matches!(refused, VoiceError::Refused(_)), "{refused}");
    assert!(refused.to_string().contains("page-3"), "{refused}");
    assert_eq!(project, before);
}

#[test]
fn a_rider_pulled_before_the_first_frame_is_refused() {
    let (dir, mut project) = project("front");
    let mut voicing = voicing();
    voicing.scenes[1].riders.push(ClipId::new("bed"));
    let before = project.clone();
    let refused = cut_to_voice(&mut project, &dir, &voicing).unwrap_err();
    assert!(
        matches!(refused, VoiceError::BeforeTheStart(_)),
        "{refused}"
    );
    assert_eq!(project, before);
}

#[test]
fn a_call_that_names_something_impossible_is_refused_by_name() {
    let (dir, project) = project("names");
    let refuse = |change: &dyn Fn(&mut crate::voice::Voicing)| {
        let mut voicing = voicing();
        change(&mut voicing);
        cut_to_voice(&mut project.clone(), &dir, &voicing)
            .unwrap_err()
            .to_string()
    };
    assert!(refuse(&|v| v.scenes[0].riders.push(ClipId::new("page-2"))).contains("more than once"));
    assert!(refuse(&|v| v.scenes[0].line = ClipId::new("page-1")).contains("audio track"));
    assert!(refuse(&|v| v.scenes[0].visuals = vec![ClipId::new("bed")]).contains("video track"));
    assert!(refuse(&|v| v.scenes[0].visuals.clear()).contains("no visual"));
    assert!(refuse(&|v| v.scenes[0].riders.push(ClipId::new("nope"))).contains("no clip `nope`"));
    assert!(refuse(&|v| v.gap = -0.1).contains("`gap`"));
    assert!(refuse(&|v| v.scenes[2].lead_in = Some(f64::NAN)).contains("`lead_in`"));
    assert!(refuse(&|v| v.scenes.clear()).contains("at least one scene"));
}
