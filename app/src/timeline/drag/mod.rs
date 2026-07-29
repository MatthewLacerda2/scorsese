//! Moving a clip: along its track, onto another one, or by one of its edges.
//!
//! Three jobs, kept apart because only one of them is about a window at all:
//! [`shape`] is the arithmetic, [`snap`] decides what a drag comes to rest
//! against, and [`commit`] writes the result into the document or refuses it.
//! What is left here is the gesture — which part of a clip was taken hold of,
//! and where the pointer has gone since.
//!
//! A drag is always measured from the clip **as it was when the gesture
//! began**. Nothing here accumulates: a pointer position proposes an absolute
//! answer, so a step the document refused costs the drag nothing and letting
//! go in the same place you started leaves the project untouched.

mod commit;
mod shape;
mod snap;

use egui::{CursorIcon, Pos2, Rect};
use scorsese_core::{AssetKind, Clip, ClipId, Frames, Project, TrackId};

use super::lanes;
use super::view::View;
use shape::Handle;
use snap::Targets;

/// How close to an edge, in pixels, counts as taking hold of the edge rather
/// than the body.
const HANDLE: f32 = 6.0;

/// How near a target has to come, in pixels, before a drag is pulled onto it.
///
/// Pixels rather than frames so it feels the same at every magnification, and
/// small because the cost of the two failures is not symmetric: a snap you did
/// not want is one nudge to undo, and a snap you did not get is a one-frame gap
/// that renders as a black flash and is never noticed until playback.
const SNAP: f32 = 8.0;

/// A clip being moved or trimmed.
#[derive(Debug)]
pub(super) struct Drag {
    /// Which clip, by id: the document is rewritten under this gesture on
    /// every pointer move, so a reference could not survive it.
    clip: ClipId,
    /// The clip exactly as it was when taken hold of. Every proposal is
    /// measured from here rather than from where the clip is now.
    origin: Clip,
    /// The track it started on, and where a trim keeps it.
    track: TrackId,
    /// How much source lies before `origin.source_in`, or `None` when nothing
    /// limits how far the head may be pulled back.
    head: Option<Frames>,
    /// Which part of it was taken hold of.
    handle: Handle,
    /// The frame under the pointer at the moment it was.
    from: Frames,
    /// Where the last accepted proposal snapped, for the indicator.
    snapped: Option<Frames>,
}

/// One pointer move, and everything answering it needs.
///
/// Bundled because these travel together and a signature long enough to need
/// counting is one nobody reads.
pub(super) struct Pull<'a> {
    /// The document, which this changes.
    pub(super) project: &'a mut Project,
    /// How far in and how magnified — the only transform there is.
    pub(super) view: View,
    /// The clip area.
    pub(super) area: Rect,
    /// Where the lanes begin inside it.
    pub(super) top: f32,
    /// Where the pointer is now.
    pub(super) at: Pos2,
    /// Where the playhead is, since a drag may land on it.
    pub(super) playhead: Frames,
    /// Alt held: the drag lands where the pointer says and nowhere else. There
    /// has to be a way to place a cut one frame off a neighbour, and a snap you
    /// cannot refuse is a snap that eventually gets in the way.
    pub(super) bypass: bool,
}

impl Drag {
    /// Takes hold of the clip under `at`, or nothing if the document has no
    /// such clip.
    pub(super) fn begin(
        project: &Project,
        hit: &lanes::Hit,
        at: Pos2,
        area: Rect,
        view: View,
    ) -> Option<Self> {
        let (_, clip) = project.clips().find(|(_, clip)| clip.id == hit.clip)?;
        // An asset that resolves to nothing is treated as timed, which is the
        // cautious reading: it refuses a trim that might be inventing source
        // rather than allowing one that might be.
        let timed = project
            .asset(&clip.asset)
            .is_none_or(|asset| has_source_timeline(asset.kind));
        Some(Self {
            clip: hit.clip.clone(),
            origin: clip.clone(),
            track: hit.track.clone(),
            head: timed.then_some(clip.source_in),
            handle: handle_at(hit.rect, at.x),
            from: view.frame_at(at.x - area.left()),
            snapped: None,
        })
    }

    /// Answers a pointer move: proposes, snaps, and writes the result — or
    /// says why it could not, having changed nothing.
    pub(super) fn follow(&mut self, pull: Pull<'_>) -> Result<(), String> {
        let now = pull.view.frame_at(pull.at.x - pull.area.left());
        let delta = now.get() as i64 - self.from.get() as i64;
        let free = shape::propose(&self.origin, self.handle, delta, self.head);

        let taken = (!pull.bypass)
            .then(|| {
                Targets::gather(pull.project, pull.playhead, &self.clip)
                    .nearest(&free.edges(self.handle), pull.view.frames_in(SNAP))
            })
            .flatten()
            // Re-proposed through the same clamps rather than shifted after
            // the fact: a snap that would push one edge through the other, or
            // off the front of the timeline, is a snap that does not happen —
            // and the indicator must not then claim it did.
            .map(|snap| {
                (
                    shape::propose(&self.origin, self.handle, delta + snap.shift, self.head),
                    snap.at,
                )
            })
            .filter(|(shaped, at)| shaped.edges(self.handle).contains(at));
        let (shaped, snapped) = match taken {
            Some((shaped, at)) => (shaped, Some(at)),
            None => (free, None),
        };

        // A trim never changes track. The edge you are pulling belongs to the
        // clip where it is, and letting the row wander while you were aiming
        // at a frame would be a move nobody asked for.
        let onto = match self.handle {
            Handle::Body => lanes::lane_at(pull.project, pull.area, pull.top, pull.at.y)
                .map_or_else(|| self.track.clone(), |(track, _)| track.id.clone()),
            Handle::Left | Handle::Right => self.track.clone(),
        };

        let outcome = commit::place(pull.project, &self.clip, &onto, shaped);
        self.snapped = outcome.is_ok().then_some(snapped).flatten();
        outcome
    }

    /// The frame this drag has come to rest against, if it has.
    pub(super) fn snapped(&self) -> Option<Frames> {
        self.snapped
    }
}

/// Which part of a clip `x` falls on.
///
/// A clip too narrow to divide into three is all body. Otherwise the shortest
/// clips would be the ones impossible to move, and moving is what you want on
/// a short clip most.
fn handle_at(rect: Rect, x: f32) -> Handle {
    if rect.width() < HANDLE * 3.0 {
        Handle::Body
    } else if x < rect.left() + HANDLE {
        Handle::Left
    } else if x > rect.right() - HANDLE {
        Handle::Right
    } else {
        Handle::Body
    }
}

/// What the pointer should look like over `hit` — the only hint that trimming
/// exists, since the end of a clip looks exactly like the rest of it.
pub(super) fn cursor(hit: &lanes::Hit, at: Pos2) -> CursorIcon {
    match handle_at(hit.rect, at.x) {
        Handle::Body => CursorIcon::Grab,
        Handle::Left | Handle::Right => CursorIcon::ResizeHorizontal,
    }
}

/// True when the asset has a timeline of its own to run out of.
///
/// A still, a title and a colour do not: `source_in` means nothing on any of
/// them, so pulling their head back is limited by the start of the timeline
/// and nothing else. Asked here rather than in `scorsese-core` because it is a
/// question about dragging — the model already says what each kind is.
fn has_source_timeline(kind: AssetKind) -> bool {
    !matches!(kind, AssetKind::Image | AssetKind::Text | AssetKind::Color)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: f32, width: f32) -> Rect {
        Rect::from_min_size(egui::pos2(left, 0.0), egui::vec2(width, 30.0))
    }

    #[test]
    fn the_ends_of_a_clip_trim_and_the_middle_moves() {
        let clip = rect(100.0, 200.0);
        assert_eq!(handle_at(clip, 101.0), Handle::Left);
        assert_eq!(handle_at(clip, 150.0), Handle::Body);
        assert_eq!(handle_at(clip, 299.0), Handle::Right);
    }

    /// Otherwise a two-second clip at a wide zoom would be all handle and
    /// nothing else — impossible to move, which is the thing you want to do
    /// with it.
    #[test]
    fn a_clip_too_narrow_to_divide_is_all_body() {
        let sliver = rect(100.0, HANDLE * 2.0);
        assert_eq!(handle_at(sliver, 100.5), Handle::Body);
        assert_eq!(handle_at(sliver, 111.0), Handle::Body);
    }

    #[test]
    fn only_the_kinds_with_a_timeline_of_their_own_limit_a_head_trim() {
        assert!(has_source_timeline(AssetKind::Video));
        assert!(has_source_timeline(AssetKind::Audio));
        assert!(has_source_timeline(AssetKind::GeneratedVideo));
        assert!(!has_source_timeline(AssetKind::Image));
        assert!(!has_source_timeline(AssetKind::Text));
    }
}
