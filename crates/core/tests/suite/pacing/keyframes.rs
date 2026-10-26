//! Keyframes go with the clip they animate when its length scales (#1101).

use scorsese_core::{Clip, Frames, Keyframe, KeyframeTrack, Project, PropertyPath, pacing};

use crate::pacing::{project, selection};

/// `head` is 120 frames and fades out over its last fifteen.
fn faded() -> Project {
    let mut project = project();
    let head = clip(&mut project, "head");
    head.keyframes
        .push(fade(&[(0, 1.0), (105, 1.0), (120, 0.0)]));
    project
}

fn fade(points: &[(u64, f64)]) -> KeyframeTrack {
    let keys = points
        .iter()
        .map(|&(t, value)| Keyframe {
            t: Frames(t),
            value,
            easing: Default::default(),
        })
        .collect();
    KeyframeTrack::new(PropertyPath::new("opacity"), keys)
}

fn clip<'a>(project: &'a mut Project, id: &str) -> &'a mut Clip {
    project
        .tracks
        .iter_mut()
        .flat_map(|track| &mut track.clips)
        .find(|clip| clip.id.as_str() == id)
        .expect("the clip is in this project")
}

/// What the fade reads at each of the clip's own frames that matter.
fn reads(project: &mut Project, id: &str, at: &[u64]) -> Vec<f64> {
    let track = &clip(project, id).keyframes[0];
    at.iter()
        .map(|&t| track.value_at(Frames(t)).expect("a track with points"))
        .collect()
}

/// Spread out, the title still fades over the end of its now longer life
/// rather than going dark at frame 120 and staying blank for two seconds.
#[test]
fn a_fade_out_survives_spreading_the_cut() {
    let mut project = faded();
    let paced = pacing::scale(
        &mut project,
        &selection(&["head", "tail"]),
        Frames::ZERO,
        1.5,
    )
    .expect("there is room for a slower cut");
    assert_eq!(paced.rescaled, ["head"].map(scorsese_core::ClipId::new));
    assert_eq!(clip(&mut project, "head").duration, Frames(180));
    let [still_up, fading, gone] = reads(&mut project, "head", &[150, 169, 180])[..] else {
        unreachable!()
    };
    assert_eq!(still_up, 1.0, "the fade has not begun two thirds in");
    assert!(fading > 0.0 && fading < 1.0, "it is part way at {fading}");
    assert_eq!(gone, 0.0, "and it is done exactly at the new end");
}

/// Closed up, the fade still plays — it is not left beyond the clip's end.
#[test]
fn a_fade_out_survives_tightening_the_cut() {
    let mut project = faded();
    pacing::scale(
        &mut project,
        &selection(&["head", "tail"]),
        Frames::ZERO,
        0.5,
    )
    .expect("a tighter cut always fits");
    assert_eq!(clip(&mut project, "head").duration, Frames(60));
    let [before, fading, gone] = reads(&mut project, "head", &[52, 56, 60])[..] else {
        unreachable!()
    };
    assert_eq!(before, 1.0);
    assert!(fading > 0.0 && fading < 1.0, "it is part way at {fading}");
    assert_eq!(gone, 0.0);
}

/// A tool's track is not ours to guess at: it stays put and the tool is named.
#[test]
fn a_signed_track_is_left_alone_and_its_tool_named() {
    let mut project = project();
    clip(&mut project, "head")
        .keyframes
        .push(fade(&[(0, 0.0), (12, 1.0)]).generated_by("dissolve"));
    let paced = pacing::scale(&mut project, &selection(&["head"]), Frames::ZERO, 0.5)
        .expect("a tighter clip fits");
    assert!(paced.rescaled.is_empty());
    assert_eq!(paced.signed.iter().collect::<Vec<_>>(), ["dissolve"]);
    let head = clip(&mut project, "head");
    assert_eq!(head.keyframes[0].keyframes[1].t, Frames(12));
}

/// A clip whose length its media owns keeps its points: nothing stretched.
#[test]
fn a_clip_that_keeps_its_length_keeps_its_keyframes() {
    let mut project = project();
    clip(&mut project, "plate")
        .keyframes
        .push(fade(&[(0, 1.0), (60, 0.0)]));
    let paced = pacing::scale(&mut project, &selection(&["plate"]), Frames(240), 2.0)
        .expect("nothing stands in its way");
    assert!(paced.rescaled.is_empty());
    assert_eq!(
        clip(&mut project, "plate").keyframes[0].keyframes[1].t,
        Frames(60)
    );
}
