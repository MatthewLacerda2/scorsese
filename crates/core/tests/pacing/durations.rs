//! What moves, what stretches, and what keeps the length its media gave it.

use scorsese_core::{ClipId, Frames, pacing};

use crate::{at, project, selection};

/// The gesture in its plainest form: the pivot's own frame does not move, what
/// is after it comes back toward it, and what is before it follows.
#[test]
fn positions_scale_about_the_instant_and_the_instant_stays_put() {
    let mut project = project();
    let paced = pacing::scale(
        &mut project,
        &selection(&["head", "tail", "logo"]),
        Frames(120),
        0.5,
    )
    .expect("a tighter cut on a track with air in it");

    assert_eq!(paced.moved, 3);
    assert_eq!(at(&project, "tail"), (120, 30), "it begins on the pivot");
    assert_eq!(
        at(&project, "head"),
        (60, 60),
        "and this one closes up to it"
    );
    assert_eq!(at(&project, "logo"), (210, 30));
}

/// Away from the pivot rather than toward it — the other half of the gesture,
/// and the half that needs somewhere to go.
#[test]
fn spreading_pushes_everything_after_the_instant_later() {
    let mut project = project();
    pacing::scale(
        &mut project,
        &selection(&["tail", "logo"]),
        Frames(120),
        2.0,
    )
    .expect("there is room out past 300");
    assert_eq!(at(&project, "tail"), (120, 120));
    assert_eq!(at(&project, "logo"), (480, 120));
}

/// The property the whole rounding rule exists for: `head` ends exactly where
/// `tail` begins, and it still does afterwards — at every factor, awkward ones
/// included. A run of cuts that touched must never come out of a retime with a
/// one-frame hole in it, because that renders as a black flash nobody notices
/// until playback.
#[test]
fn a_run_of_cuts_that_touched_still_touches() {
    for factor in [0.37, 0.5, 0.83, 1.19, 2.6, 7.0] {
        let mut project = project();
        pacing::scale(
            &mut project,
            &selection(&["head", "tail", "logo"]),
            Frames::ZERO,
            factor,
        )
        .expect("the whole track scales together, so nothing collides");
        let (start, duration) = at(&project, "head");
        assert_eq!(
            start + duration,
            at(&project, "tail").0,
            "a gap opened at ×{factor}"
        );
    }
}

/// A title, a still and a brief nobody has generated are all one case: the
/// duration says nothing but how long the thing is on screen, so scaling it is
/// free and exact. The still is the interesting one — there is a file behind
/// it, and it still has no length of its own.
#[test]
fn a_clip_with_no_media_behind_it_stretches_with_the_cut() {
    let mut project = project();
    let paced = pacing::scale(
        &mut project,
        &selection(&["head", "tail", "logo", "line"]),
        Frames::ZERO,
        0.5,
    )
    .expect("halving a cut leaves room everywhere");

    assert!(
        paced.kept.is_empty(),
        "none of these has a length of its own"
    );
    assert_eq!(at(&project, "logo"), (150, 30), "a still");
    assert_eq!(at(&project, "line"), (30, 30), "a brief, not yet generated");
}

/// The refusal that the whole distinction is for. `plate` is a shot and
/// `score` is a recipe already baked to a file: shortening either means showing
/// less of it or playing it faster, which are two different edits. So they move
/// and keep their length, and the operation **says which ones** rather than
/// leaving it to be discovered.
#[test]
fn a_clip_with_media_behind_it_moves_and_keeps_its_length() {
    let mut project = project();
    let paced = pacing::scale(
        &mut project,
        &selection(&["plate", "score"]),
        Frames(240),
        0.5,
    )
    .expect("each is alone on its track");

    let kept: Vec<&str> = paced.kept.iter().map(ClipId::as_str).collect();
    assert_eq!(kept, ["plate", "score"]);
    assert_eq!(at(&project, "plate"), (240, 60), "on the pivot, so unmoved");
    assert_eq!(at(&project, "score"), (120, 360), "moved, same length");
}

/// A selected clip may pass an unselected one, and a document whose clips are
/// listed out of time order is one nobody can read.
#[test]
fn a_track_comes_out_in_time_order() {
    let mut project = project();
    pacing::scale(&mut project, &selection(&["head"]), Frames(360), 0.5)
        .expect("180 is clear, and touching `tail` is not overlapping it");
    let order: Vec<&str> = project.tracks[0]
        .clips
        .iter()
        .map(|clip| clip.id.as_str())
        .collect();
    assert_eq!(order, ["tail", "head", "logo"]);
}
