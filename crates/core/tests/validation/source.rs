//! The ceiling on a trim: a clip may not show source that is not there.
//!
//! Every case here turns on one thing — whether the asset carries a *measured*
//! length. The fixture's assets carry none, so each test says what it measured
//! before it says what it expects.

use crate::common::{assert_only_problem, asset_id, asset_mut, clip_id, project};
use scorsese_core::{Frames, MediaMetadata, Project, TimelineProblem as E};

/// The fixture with `shot-city` measured at `seconds`, and its clip — from the
/// head of the source, on a 30fps timeline — reshaped to `source_in` and
/// `duration`.
///
/// `v1` is cleared down to that one clip, so a length these tests choose for
/// its own reasons cannot also collide with the title that follows it: an
/// overlap is a different problem, checked elsewhere.
fn measured(seconds: f64, source_in: u64, duration: u64) -> Project {
    let mut p = project();
    asset_mut(&mut p, "shot-city").media = Some(MediaMetadata {
        duration_seconds: Some(seconds),
        ..MediaMetadata::default()
    });
    p.tracks[0].clips.truncate(1);
    let clip = &mut p.tracks[0].clips[0];
    clip.source_in = Frames(source_in);
    clip.duration = Frames(duration);
    p
}

fn outlasts(reaches: u64, available: u64) -> E {
    E::ClipOutlastsSource {
        clip: clip_id("c-shot"),
        asset: asset_id("shot-city"),
        reaches: Frames(reaches),
        available: Frames(available),
    }
}

#[test]
fn a_clip_reaching_past_the_end_of_its_source_is_refused() {
    assert_only_problem(&measured(8.0, 0, 300), outlasts(300, 240));
}

/// The whole of the source is not past the end of it. An edit that uses every
/// frame there is has to be the ordinary case, or the ceiling is off by one.
#[test]
fn a_clip_that_ends_exactly_where_its_source_does_is_fine() {
    assert_eq!(measured(8.0, 0, 240).validate(), Ok(()));
}

/// The offset counts against the same ceiling: a clip starting two seconds in
/// has two seconds less to play through.
#[test]
fn where_a_clip_starts_in_the_source_counts_against_the_ceiling() {
    assert_only_problem(&measured(8.0, 60, 240), outlasts(300, 240));
    assert_eq!(measured(8.0, 60, 180).validate(), Ok(()));
}

/// A source shot at another rate is conformed, so the ceiling is in timeline
/// frames like everything else a clip is written in: one second of 24fps
/// footage is thirty frames of a 30fps timeline, not twenty-four.
#[test]
fn the_ceiling_is_in_timeline_frames_whatever_the_source_was_shot_at() {
    assert_eq!(measured(1.0, 0, 30).validate(), Ok(()));
    assert_only_problem(&measured(1.0, 0, 31), outlasts(31, 30));
}

/// An asset nobody has measured bounds nothing. This is the case the ceiling
/// was nearly not built for: refusing honest trims on assets we have not
/// looked at is worse than the problem it prevents, and `scorsese probe` is
/// what turns the bound on.
#[test]
fn an_unmeasured_asset_has_no_ceiling() {
    let mut p = project();
    p.tracks[0].clips.truncate(1);
    p.tracks[0].clips[0].duration = Frames(9_000);
    assert_eq!(p.validate(), Ok(()));
}

/// A still is held for as long as you like, and says so by carrying a size and
/// no duration — which is exactly what import writes for one.
#[test]
fn a_still_has_no_length_to_run_out_of() {
    let mut p = project();
    asset_mut(&mut p, "logo").media = Some(MediaMetadata {
        width: Some(512),
        height: Some(512),
        ..MediaMetadata::default()
    });
    p.tracks[1].clips[0].duration = Frames(9_000);
    assert_eq!(p.validate(), Ok(()));
}

/// A clip of a source measured at nothing at all is refused, and that is the
/// honest reading rather than a special case: a file with no length in it has
/// no frame a clip could show.
#[test]
fn a_source_measured_at_zero_holds_nothing() {
    assert_only_problem(&measured(0.0, 0, 30), outlasts(30, 0));
}
