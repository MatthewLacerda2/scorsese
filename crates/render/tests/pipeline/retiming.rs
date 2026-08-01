//! Rendering a clip that runs at a rate of its own.
//!
//! The fixture is three seconds of source, one colour a second, so which colour
//! comes out of a given output frame says exactly how far into the source the
//! puller had walked by then. A puller that still assumed one source frame per
//! timeline frame would be a whole band out.

use scorsese_core::Fps;
use scorsese_render::{FrameRange, Note};

use crate::common::ffmpeg::{fixture_dir, inspect, mean_rgb, tools};
use crate::common::{clip, clip_from, project, sped, video_track};
use crate::{BLACK, BLUE, GREEN, RED, assert_colour, banded_source, colour_asset, render};

/// At 2× the render reaches the second band halfway through the first second
/// and the third band at the end of it — the whole three-second source inside a
/// second and a half of timeline.
#[test]
fn a_sped_clip_walks_its_source_at_the_rate_it_asks_for() {
    let tools = tools();
    let dir = fixture_dir("speed-up");
    let bands = banded_source(&tools, &dir);
    let project = project(
        vec![bands],
        vec![video_track(
            "v1",
            vec![sped(2.0, clip("c1", "bands", 0, 45))],
        )],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_eq!(report.frames, 45, "the clip's slot on the timeline is 45");
    assert_eq!(inspect(&tools, &out).frames, 45);
    assert_colour(mean_rgb(&tools, &out, 2), RED, "source frame 4");
    assert_colour(mean_rgb(&tools, &out, 20), GREEN, "source frame 40");
    assert_colour(mean_rgb(&tools, &out, 40), BLUE, "source frame 80");
    std::fs::remove_dir_all(&dir).ok();
}

/// And the other way: at 0.5× the first band lasts two seconds of timeline, so
/// a frame that would have been green at 1× is still red.
#[test]
fn a_slowed_clip_holds_each_part_of_its_source_for_longer() {
    let tools = tools();
    let dir = fixture_dir("slow-down");
    let bands = banded_source(&tools, &dir);
    let project = project(
        vec![bands],
        vec![video_track(
            "v1",
            vec![sped(0.5, clip("c1", "bands", 0, 90))],
        )],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_eq!(report.frames, 90);
    assert_colour(mean_rgb(&tools, &out, 40), RED, "still the first band");
    assert_colour(mean_rgb(&tools, &out, 75), GREEN, "source frame 37");
    std::fs::remove_dir_all(&dir).ok();
}

/// `source_in` is where playback begins; the rate says how fast it moves from
/// there. So the two compose, and neither scales the other.
#[test]
fn a_rate_starts_from_the_clips_own_source_in() {
    let tools = tools();
    let dir = fixture_dir("speed-seek");
    let bands = banded_source(&tools, &dir);
    let project = project(
        vec![bands],
        // Opens on the green band, and at 2× is into the blue one 15 frames in.
        vec![video_track(
            "v1",
            vec![sped(2.0, clip_from("c1", "bands", 0, 30, 30))],
        )],
    );

    let (out, _) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_colour(mean_rgb(&tools, &out, 2), GREEN, "where it opens");
    assert_colour(mean_rgb(&tools, &out, 20), BLUE, "40 source frames later");
    std::fs::remove_dir_all(&dir).ok();
}

/// Speeding a clip up consumes more of its footage in the same slot, so it can
/// run off the end exactly as an over-long trim can. Same ceiling, same answer:
/// the render carries on, the picture goes blank, and the report says which clip
/// came up short.
#[test]
fn a_clip_sped_past_the_end_of_its_source_finishes_blank_and_says_so() {
    let tools = tools();
    let dir = fixture_dir("speed-short");
    let red = colour_asset(&tools, &dir, "red", "64x64", 1);
    // Thirty frames of source, asked for at 4× over thirty frames of timeline:
    // the source is spent after eight of them.
    let project = project(
        vec![red],
        vec![video_track("v1", vec![sped(4.0, clip("c1", "red", 0, 30))])],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_eq!(report.frames, 30, "the clip's length is what was asked for");
    let missing = report.notes.iter().find_map(|note| match note {
        Note::ClipRanShort { clip, missing } if clip == "c1" => Some(*missing),
        _ => None,
    });
    let missing =
        missing.unwrap_or_else(|| panic!("expected a short-source note: {:?}", report.notes));
    assert!(
        (18..=25).contains(&missing),
        "about 22 frames should be missing, not {missing}"
    );
    assert_colour(
        mean_rgb(&tools, &out, 28),
        BLACK,
        "past the end of the source",
    );
    std::fs::remove_dir_all(&dir).ok();
}
