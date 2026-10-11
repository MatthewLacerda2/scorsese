//! How much of its source a clip at a speed reads, and what validation makes
//! of it running out (#1098): the render reads `source_in` plus speed times
//! the frames elapsed, so the ceiling is held to that and not to `duration`.

use crate::common::{asset_id, asset_mut, clip_id, problems, project};
use scorsese_core::{
    Frames, MediaMetadata, PastSource, Project, Speed, TimelineProblem, ValidationError,
};

/// The fixture with `shot-city` measured at ten seconds — 300 frames of a
/// 30fps timeline — and its clip, alone on `v1`, at `speed` for `duration`.
fn at(speed: f64, duration: u64) -> Project {
    let mut p = project();
    asset_mut(&mut p, "shot-city").media = Some(MediaMetadata {
        duration_seconds: Some(10.0),
        ..MediaMetadata::default()
    });
    p.tracks[0].clips.truncate(1);
    let clip = &mut p.tracks[0].clips[0];
    (clip.speed, clip.duration) = (Speed::new(speed), Frames(duration));
    p
}

#[test]
fn a_clip_reads_its_source_at_its_speed() {
    let p = at(2.0, 100);
    assert_eq!(p.tracks[0].clips[0].source_end(), Frames(200));
    let p = at(0.5, 100);
    assert_eq!(p.tracks[0].clips[0].source_end(), Frames(50));
}

/// A frame the clip reaches into at all is one it shows, so a fraction rounds
/// up — and a float a hair over a whole frame does not.
#[test]
fn a_fraction_of_a_source_frame_counts_as_the_frame() {
    assert_eq!(at(0.5, 101).tracks[0].clips[0].source_end(), Frames(51));
    assert_eq!(at(1.1, 100).tracks[0].clips[0].source_end(), Frames(110));
}

/// The plain request the old arithmetic refused: slowing a whole shot down.
/// 600 frames of timeline at 0.5× read exactly the 300 there are.
#[test]
fn slowing_a_whole_clip_down_is_accepted() {
    let mut p = at(1.0, 300);
    p.tracks[0].clips[0].retime(Speed::new(0.5));
    assert_eq!(p.tracks[0].clips[0].duration, Frames(600));
    assert_eq!(p.validate(), Ok(()));
    assert!(p.past_source().is_empty());
}

/// Slowed down and still too long is refused, and reports the frame it really
/// reaches rather than its length on the timeline.
#[test]
fn a_slowed_clip_still_past_its_source_is_refused() {
    assert_eq!(
        problems(&at(0.5, 700)),
        vec![ValidationError::from(TimelineProblem::ClipOutlastsSource {
            clip: clip_id("c-shot"),
            asset: asset_id("shot-city"),
            reaches: Frames(350),
            available: Frames(300),
        })]
    );
}

/// The reverse, which loaded before #1098 and must go on loading: a warning,
/// never a refusal.
#[test]
fn a_sped_up_clip_past_its_source_is_warned_about_not_refused() {
    let p = at(2.0, 200);
    assert_eq!(p.validate(), Ok(()));
    let past = p.past_source();
    assert_eq!(
        past,
        vec![PastSource {
            clip: clip_id("c-shot"),
            asset: asset_id("shot-city"),
            reaches: Frames(400),
            available: Frames(300),
        }]
    );
    assert!(past[0].to_string().contains("plays past the footage"));
    assert!(at(2.0, 150).past_source().is_empty(), "300 is all of it");
}

/// Sped up past what even one frame a frame would allow was refused before
/// and still is: the warning only covers what used to load.
#[test]
fn a_sped_up_clip_too_long_at_any_speed_is_still_refused() {
    let p = at(2.0, 320);
    assert!(p.validate().is_err());
    assert!(p.past_source().is_empty(), "reported once, as a problem");
}
