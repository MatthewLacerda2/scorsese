//! The panel with more than one clip selected.
//!
//! The honest answer, and the smallest one: **say how many, say what they have
//! in common, and offer no field at all.**
//!
//! Not offering fields is the decision worth defending. A field over several
//! clips has to mean one of two things — write this value onto every one of
//! them, or write it onto the ones that already agree — and both are edits a
//! person would have to be told about before using. That is a multi-edit
//! inspector, a much larger idea than a wider selection, and the thing a
//! selection is actually *for* is the operations that act on it as a whole:
//! scaling a run of clips about the playhead, deleting them, nudging them.
//! Those arrive as gestures and as sentences to an assistant, not as a column
//! of boxes.
//!
//! So what is left is a summary. It exists because a selection you cannot see
//! the extent of is one you cannot trust — "four clips, all on `v1`, covering
//! 0:00.0 to 0:14.0" is the sentence that tells you the click you just made
//! did what you meant.

use std::collections::BTreeSet;

use egui::{RichText, Ui};
use scorsese_core::{AssetId, AssetKind, ClipId, Frames, Project, TrackId};

use super::one::kind_name;
use super::time;

/// Several selected clips, read out of the document in one pass.
///
/// A snapshot for the same reason [`super::selected::Selected`] is one: the
/// panel that draws it must not be holding a borrow into a project that a
/// neighbouring panel may be about to change.
pub(super) struct Several {
    /// How many clips are selected. Never one — that is [`super::one`]'s case.
    count: usize,
    /// The asset every one of them shows, when they all show the same one.
    asset: Option<AssetId>,
    /// What they are, when they agree on the kind even if not on the asset —
    /// "four text clips" is a useful thing to know about a run of titles.
    kind: Option<AssetKind>,
    /// The track they all sit on, when it is one track.
    track: Option<TrackId>,
    /// How many tracks they are spread over, for when it is not one.
    tracks: usize,
    /// The first frame any of them occupies, and the frame just past the last.
    /// The extent of the selection, which is the fact a person is checking.
    span: (Frames, Frames),
}

impl Several {
    /// Reads the selected clips out of `project`, or `None` when none of them
    /// is in it — which a pruned selection makes unreachable, and which is
    /// still said rather than assumed away.
    pub(super) fn of(project: &Project, clips: &BTreeSet<ClipId>) -> Option<Self> {
        let found: Vec<_> = project
            .clips()
            .filter(|(_, clip)| clips.contains(&clip.id))
            .collect();
        let first = found.first()?;
        let tracks: BTreeSet<&TrackId> = found.iter().map(|(track, _)| &track.id).collect();
        Some(Self {
            count: found.len(),
            asset: agreed(found.iter().map(|(_, clip)| Some(clip.asset.clone()))),
            kind: agreed(
                found
                    .iter()
                    .map(|(_, clip)| project.asset(&clip.asset).map(|asset| asset.kind)),
            ),
            track: (tracks.len() == 1).then(|| first.0.id.clone()),
            tracks: tracks.len(),
            span: (
                found.iter().map(|(_, clip)| clip.start).min()?,
                found.iter().map(|(_, clip)| clip.end()).max()?,
            ),
        })
    }
}

/// Draws the summary.
pub(super) fn show(ui: &mut Ui, project: &Project, clips: &BTreeSet<ClipId>) {
    let Some(several) = Several::of(project, clips) else {
        return;
    };
    let fps = project.timeline_fps;
    ui.add_space(4.0);
    ui.label(RichText::new(format!("{} clips selected", several.count)).strong());
    ui.label(RichText::new(where_they_are(&several)).weak().small());
    if let Some(what) = what_they_are(&several) {
        ui.label(RichText::new(what).weak().small());
    }
    ui.label(
        RichText::new(format!(
            "{} – {}",
            time::readable(several.span.0, fps),
            time::readable(several.span.1, fps)
        ))
        .weak()
        .small(),
    );
    ui.add_space(10.0);
    ui.label(
        RichText::new(
            "nothing here changes several clips at once — that is a sentence to an assistant",
        )
        .weak()
        .italics()
        .small(),
    );
}

/// One track named, or how many they are spread across.
fn where_they_are(several: &Several) -> String {
    match &several.track {
        Some(track) => format!("on track {track}"),
        None => format!("across {} tracks", several.tracks),
    }
}

/// What they all are, when there is one answer. The asset when it is the same
/// asset, the kind when it is not — and nothing at all when a selection is a
/// title, a shot and a piece of music, because "mixed" is not information.
fn what_they_are(several: &Several) -> Option<String> {
    match (&several.asset, several.kind) {
        (Some(asset), Some(kind)) => Some(format!("all {asset} · {}", kind_name(kind))),
        (Some(asset), None) => Some(format!("all {asset}")),
        (None, Some(kind)) => Some(format!("all {}", kind_name(kind))),
        (None, None) => None,
    }
}

/// The one value everything agrees on, or `None` — which a single disagreement
/// or a single missing answer is enough to earn.
fn agreed<T: PartialEq>(values: impl IntoIterator<Item = Option<T>>) -> Option<T> {
    let mut values = values.into_iter();
    let first = values.next()??;
    values
        .all(|value| value.as_ref() == Some(&first))
        .then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspector::fixture::project;

    fn selection(ids: &[&str]) -> BTreeSet<ClipId> {
        ids.iter().map(|id| ClipId::new(id)).collect()
    }

    /// The fact a person is checking after a shift-click: how many, and how far
    /// they reach. `c1` runs 0–90 and `c2` 120–180, so the span is the outside
    /// of both and not the sum of them.
    #[test]
    fn the_span_reaches_from_the_first_start_to_the_last_end() {
        let several = Several::of(&project(), &selection(&["c1", "c2"])).expect("both are there");
        assert_eq!(several.count, 2);
        assert_eq!(several.span, (Frames(0), Frames(180)));
        assert_eq!(several.track.as_ref().map(TrackId::as_str), Some("v1"));
    }

    /// Two clips of the same asset agree on it; two of different kinds agree on
    /// nothing, and saying "mixed" would be saying nothing.
    #[test]
    fn what_they_are_is_said_only_when_there_is_one_answer() {
        let both = Several::of(&project(), &selection(&["c1", "c2"])).expect("both are there");
        assert_eq!(what_they_are(&both).as_deref(), Some("all shot · video"));

        let across = Several::of(&project(), &selection(&["c1", "c3"])).expect("both are there");
        assert!(across.asset.is_none() && across.kind.is_none());
        assert_eq!(what_they_are(&across), None);
        assert_eq!(where_they_are(&across), "across 2 tracks");
    }

    /// A selection naming nothing that exists has nothing to summarise, and
    /// says so by being `None` rather than by drawing an empty panel.
    #[test]
    fn a_selection_of_clips_that_are_gone_is_nothing() {
        assert!(Several::of(&project(), &selection(&["gone", "also-gone"])).is_none());
    }

    /// One dissenter is enough. A helper that reported agreement on a majority
    /// would put a value on screen that is false of some clip in the selection.
    #[test]
    fn agreement_takes_every_one_of_them() {
        assert_eq!(agreed([Some(1), Some(1), Some(1)]), Some(1));
        assert_eq!(agreed([Some(1), Some(1), Some(2)]), None);
        assert_eq!(agreed([Some(1), None]), None);
        assert_eq!(agreed::<u8>([]), None);
    }
}
