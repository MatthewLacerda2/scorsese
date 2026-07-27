//! What a plan says about the cut it will produce.
//!
//! No ffmpeg anywhere here, which is the point of the feature as much as of
//! the tests: a description is derived from the document, so it costs nothing
//! and can be asked for before a frame has been encoded.

#[path = "../common/mod.rs"]
mod common;

mod cards;
mod cues;
mod cuts;
mod fades;
mod sound;

use scorsese_core::{Easing, Fps, Frames, Keyframe, KeyframeTrack, Project, PropertyPath};
use scorsese_render::{Description, FrameRange, Plan};

/// The description of a whole project, at its own timeline rate.
fn described(project: &Project) -> Description {
    described_over(project, FrameRange::ALL, Fps::THIRTY)
}

/// The description of part of a project, possibly at another output rate.
fn described_over(project: &Project, range: FrameRange, fps: Fps) -> Description {
    let plan = Plan::build(project, fps, range).expect("the fixture timeline sequences");
    Description::of(&plan)
}

/// Every stretch as `(first frame, end frame)`, which is the shape a cut has.
fn shape(description: &Description) -> Vec<(u64, u64)> {
    description
        .stretches
        .iter()
        .map(|stretch| (stretch.from.frame.get(), stretch.to.frame.get()))
        .collect()
}

/// A keyframe track over two points, which is every fade and every move.
fn ramp(property: &str, points: [(u64, f64); 2], easing: Easing) -> KeyframeTrack {
    KeyframeTrack::new(
        PropertyPath::new(property),
        points
            .into_iter()
            .map(|(t, value)| Keyframe {
                t: Frames(t),
                value,
                easing,
            })
            .collect(),
    )
}

/// A clip carrying keyframe tracks.
fn animated(mut clip: scorsese_core::Clip, tracks: Vec<KeyframeTrack>) -> scorsese_core::Clip {
    clip.keyframes = tracks;
    clip
}
