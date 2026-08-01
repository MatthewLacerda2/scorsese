//! What a drag is proposing, in frames — the arithmetic, with no project and
//! no screen anywhere near it.
//!
//! The shapes a gesture speaks in live here; working the proposal out from a
//! pointer's travel is [`propose`], which is where the awkward parts are — the
//! two floors, the source-side ceiling, and the fact that a frame of timeline
//! stops being a frame of source the moment a clip has a speed.
//!
//! Every proposal is computed from the clip **as it was when the gesture
//! began**, plus the whole pointer travel since. Never from the clip's current
//! state plus one more step: a step the document refused would otherwise be
//! paid for twice, once by not happening and once by the next step starting
//! from somewhere the clip never was.

use scorsese_core::{Clip, Frames};

/// Which part of a clip was taken hold of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::timeline) enum Handle {
    /// The middle: the clip moves, keeping its length and its content.
    Body,
    /// The head. Moves where the clip starts **and** where in the source it
    /// starts, which is the difference between trimming and sliding.
    Left,
    /// The tail. Only the length changes; the head stays put.
    Right,
}

/// How much source a clip has on either side of what it is showing, in frames
/// of the timeline grid.
///
/// The two ceilings a trim comes to rest against, and they are absent for
/// different reasons. `None` for the head means the asset has no timeline of
/// its own — a still, a title, a colour — so pulling its head back is limited
/// by the start of the timeline and nothing else. `None` for the tail means
/// nothing has *measured* a length: the same three, plus a sketch with no file
/// yet, plus a file nobody has probed. A ceiling invented from an absence would
/// stop honest trims on footage we simply have not looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::timeline) struct Limits {
    /// How much source lies before the clip's `source_in`.
    pub(in crate::timeline) head: Option<Frames>,
    /// How much source lies past where the clip already ends.
    pub(in crate::timeline) tail: Option<Frames>,
}

/// Where a clip would sit if the pointer were let go now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::timeline) struct Shape {
    /// Where it begins on the timeline.
    pub(in crate::timeline) start: Frames,
    /// How long it runs. Never zero — a clip covering no frame is an edit that
    /// went wrong, and validation says so.
    pub(in crate::timeline) duration: Frames,
    /// Where in the source it begins.
    pub(in crate::timeline) source_in: Frames,
}

impl Shape {
    /// The clip as it stands, untouched — where every proposal starts from.
    pub(in crate::timeline) fn of(clip: &Clip) -> Self {
        Self {
            start: clip.start,
            duration: clip.duration,
            source_in: clip.source_in,
        }
    }

    /// The frame just past the last one it occupies.
    pub(in crate::timeline) fn end(self) -> Frames {
        self.start + self.duration
    }

    /// The edges a snap may take hold of.
    ///
    /// Both of them for a move, because butting a clip up against the one
    /// before it is the same gesture as butting it against the one after —
    /// and only the edge being pulled for a trim, since the other one is not
    /// moving.
    pub(in crate::timeline) fn edges(self, handle: Handle) -> Vec<Frames> {
        match handle {
            Handle::Body => vec![self.start, self.end()],
            Handle::Left => vec![self.start],
            Handle::Right => vec![self.end()],
        }
    }
}

mod propose;

pub(in crate::timeline) use propose::propose;
