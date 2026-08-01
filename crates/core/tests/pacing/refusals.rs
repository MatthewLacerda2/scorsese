//! What a scale will not do — and the promise that comes with every refusal:
//! the document is left exactly as it was, so a live gesture can go on asking.

use scorsese_core::{Frames, PaceError, pacing};

use crate::{at, project, selection};

/// Zero collapses every clip onto the pivot and a negative factor turns the
/// edit back to front. Neither is a thing anyone means by "spread these out".
#[test]
fn a_factor_that_is_not_a_positive_number_is_refused() {
    for factor in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut project = project();
        let refused = pacing::scale(&mut project, &selection(&["head"]), Frames::ZERO, factor)
            .expect_err("not a factor anything could mean");
        assert!(matches!(refused, PaceError::Factor(_)), "{refused}");
        assert_eq!(at(&project, "head"), (0, 120), "and nothing moved");
    }
}

/// Naming clips that are not there is a caller's mistake, and a silent success
/// would let it go on being made.
#[test]
fn a_selection_of_clips_that_are_not_here_is_refused() {
    let mut project = project();
    let refused = pacing::scale(&mut project, &selection(&["gone"]), Frames::ZERO, 2.0)
        .expect_err("no such clip");
    assert!(matches!(refused, PaceError::Nothing), "{refused}");
}

/// The timeline has no frames before zero. Holding a clip against the front
/// while its far edge went on scaling would be a trim nobody asked for, so the
/// scale stops instead — which, during a drag, is the clips ceasing to move.
#[test]
fn a_clip_pushed_off_the_front_of_the_timeline_stops_the_scale() {
    let mut project = project();
    let refused = pacing::scale(&mut project, &selection(&["tail"]), Frames(300), 2.0)
        .expect_err("120 is only 180 frames back from 300");
    assert!(
        matches!(&refused, PaceError::BeforeTheStart { clip } if clip.as_str() == "tail"),
        "{refused}"
    );
    assert_eq!(at(&project, "tail"), (120, 60));
}

/// A clip covering no frame renders nothing. Rounding one away would leave a
/// title that is simply gone from the cut, which is worse than the gesture
/// stopping.
#[test]
fn a_clip_scaled_away_to_nothing_stops_the_scale() {
    let mut project = project();
    let refused = pacing::scale(&mut project, &selection(&["tail"]), Frames(120), 0.001)
        .expect_err("sixty frames of nothing is nothing");
    assert!(
        matches!(&refused, PaceError::Vanishes { clip, .. } if clip.as_str() == "tail"),
        "{refused}"
    );
    assert_eq!(at(&project, "tail"), (120, 60));
}

/// The bound the issue is bound by: a selected clip meeting an **unselected**
/// one, which the scale does not move. Refused by the document's own
/// validation rather than by a second opinion about what is legal here.
#[test]
fn a_scale_that_would_overlap_a_clip_it_does_not_move_is_refused() {
    let mut project = project();
    let refused = pacing::scale(&mut project, &selection(&["head"]), Frames::ZERO, 2.0)
        .expect_err("head would grow to 240 and `tail` still begins at 120");
    assert!(
        matches!(&refused, PaceError::Refused(errors) if errors.to_string().contains("overlap")),
        "{refused}"
    );
    assert_eq!(at(&project, "head"), (0, 120), "and nothing moved");
    assert_eq!(at(&project, "tail"), (120, 60));
}
