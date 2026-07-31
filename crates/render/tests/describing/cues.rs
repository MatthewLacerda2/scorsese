//! Asking for an instant in either unit, and getting the same frame back.

use scorsese_core::{AssetKind, Fps, Frames};
use scorsese_render::{Cue, FrameRange};

use crate::common::{clip, file_asset, project, video_track};
use crate::{described, described_over};

/// Three seconds of one shot on a 30 fps grid.
fn shot() -> scorsese_core::Project {
    project(
        vec![file_asset("one", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "one", 0, 90)])],
    )
}

/// A bare number is a frame; a number with `s` is a time. A decimal with no
/// suffix is refused rather than guessed at — reading `2.5` as seconds would
/// make it and `2` mean instants 2.5 seconds apart.
#[test]
fn a_cue_is_a_frame_or_a_time_and_never_a_guess() {
    assert_eq!("75".parse(), Ok(Cue::Frame(Frames(75))));
    assert_eq!(" 2.5s ".parse(), Ok(Cue::Seconds(2.5)));
    assert_eq!("0s".parse(), Ok(Cue::Seconds(0.0)));
    assert!("2.5".parse::<Cue>().is_err());
    assert!("-1".parse::<Cue>().is_err());
    assert!("-1s".parse::<Cue>().is_err());
    assert!("later".parse::<Cue>().is_err());
}

#[test]
fn both_units_name_the_same_frame_of_the_file() {
    let description = described(&shot());

    assert_eq!(Cue::Frame(Frames(45)).out_frame(&description), 45);
    assert_eq!(Cue::Seconds(1.5).out_frame(&description), 45);
    assert_eq!(Cue::Seconds(0.0).out_frame(&description), 0);
}

/// A range moves the file's frame 0 onto the timeline frame the render starts
/// at. Times stay timeline times, so an instant a description printed can be
/// handed straight back as a cue.
#[test]
fn a_range_shifts_the_frames_of_the_file_but_not_the_times() {
    let range = FrameRange::new(Frames(30), None).expect("a forward range");
    let description = described_over(&shot(), range, Fps::THIRTY);

    assert_eq!(Cue::Seconds(1.0).out_frame(&description), 0);
    assert_eq!(Cue::Frame(Frames(30)).out_frame(&description), 0);
    assert_eq!(Cue::Seconds(2.0).out_frame(&description), 30);
}

/// Rendering at a rate the timeline was not authored at conforms both units the
/// same way, so neither is the special case.
#[test]
fn an_output_rate_of_its_own_conforms_both_units() {
    let description = described_over(&shot(), FrameRange::ALL, Fps::SIXTY);

    assert_eq!(description.out_frames, 180);
    assert_eq!(Cue::Seconds(1.5).out_frame(&description), 90);
    assert_eq!(
        Cue::Frame(Frames(45)).out_frame(&description),
        90,
        "timeline frame 45 is 1.5s, which is output frame 90 at 60 fps"
    );
}

/// An instant past the end lands on the last frame there is, rather than off
/// the end of the file where extraction would simply fail.
#[test]
fn an_instant_past_the_end_lands_on_the_last_frame() {
    let description = described(&shot());

    assert_eq!(Cue::Seconds(99.0).out_frame(&description), 89);
    assert_eq!(Cue::Frame(Frames(90)).out_frame(&description), 89);
}

/// The other end of the same question: a still has no delivered file to index
/// into, so a cue names a frame of the timeline directly. Both units go to the
/// grid the document is authored on, and nothing conforms.
#[test]
fn a_cue_names_a_timeline_frame_when_there_is_no_file_to_count_frames_of() {
    assert_eq!(
        Cue::Frame(Frames(45)).timeline_frame(Fps::THIRTY),
        Frames(45)
    );
    assert_eq!(Cue::Seconds(1.5).timeline_frame(Fps::THIRTY), Frames(45));
    assert_eq!(Cue::Seconds(0.0).timeline_frame(Fps::THIRTY), Frames::ZERO);
    assert_eq!(
        Cue::Frame(Frames(45)).timeline_frame(Fps::SIXTY),
        Frames(45),
        "a frame is already an answer — the rate cannot re-time it"
    );
    assert_eq!(Cue::Seconds(1.5).timeline_frame(Fps::SIXTY), Frames(90));
}
