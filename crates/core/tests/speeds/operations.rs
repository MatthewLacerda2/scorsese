//! Retiming: the operation that rescales `duration` as it sets the rate, and
//! the arithmetic the render plan runs on.

use scorsese_core::{AssetId, Clip, ClipId, Frames, Speed};

/// A clip 100 frames long at frame 200, playing at its source's own rate.
fn clip() -> Clip {
    Clip::new(
        ClipId::new("c"),
        AssetId::new("a"),
        Frames(200),
        Frames(100),
    )
}

/// The document's half of the split: writing a speed changes the rate and
/// nothing else, so the clip keeps its slot and shows more of its source in it.
#[test]
fn writing_a_speed_leaves_the_clip_where_it_is() {
    let mut clip = clip();
    clip.speed = Speed::new(2.0);
    assert_eq!(
        clip.duration,
        Frames(100),
        "the slot on the timeline is the slot"
    );
    assert_eq!(
        clip.source_frames(),
        200.0,
        "and it now holds twice the footage"
    );
}

/// The operation's half: what an editor's 2× does — same footage, half the
/// time.
#[test]
fn retiming_rescales_the_duration_to_cover_the_same_source() {
    let mut clip = clip();
    clip.retime(Speed::new(2.0));
    assert_eq!(clip.duration, Frames(50));
    assert_eq!(clip.source_frames(), 100.0, "the same 100 frames of source");
    assert_eq!(clip.start, Frames(200), "and it has not moved");

    clip.retime(Speed::new(0.5));
    assert_eq!(clip.duration, Frames(200), "and back the other way");
    assert_eq!(clip.source_frames(), 100.0);
}

/// A clip covering no frame is not a clip, so an extreme speed shortens one to
/// a frame rather than deleting it.
#[test]
fn retiming_never_leaves_a_clip_with_no_frames() {
    let mut clip = clip();
    clip.retime(Speed::new(10_000.0));
    assert_eq!(clip.duration, Frames(1));
}

/// There is no length to rescale to, so nothing happens at all — the caller is
/// left with the clip it had rather than one silently mangled.
#[test]
fn retiming_to_a_rate_nothing_plays_at_changes_nothing() {
    let mut clip = clip();
    clip.retime(Speed::new(0.0));
    assert_eq!(clip.speed, Speed::NORMAL);
    assert_eq!(clip.duration, Frames(100));
}

/// Where a stretch of timeline lands in the source, which is the arithmetic the
/// render plan runs on. Fractional on purpose: rounding it is how a resumed
/// segment ends up half a frame from where the last one stopped.
#[test]
fn a_rate_says_how_much_source_a_stretch_of_timeline_consumes() {
    assert_eq!(Speed::new(1.5).source_frames(Frames(5)), 7.5);
    assert_eq!(Speed::new(0.5).source_frames(Frames(30)), 15.0);
    assert_eq!(Speed::new(1.5).timeline_frames(7.5), 5.0);
}
