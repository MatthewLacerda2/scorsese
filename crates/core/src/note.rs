//! Notes: why a thing in the project is the way it is.
//!
//! Every reason behind an edit used to live outside the project and die with
//! the conversation that produced it. Reading a document cold, nothing said
//! that a title was quoted copy rather than written, that a shot was tried
//! full-bleed first and moved back because white text over a bright sky did not
//! read, or that some footage was a stand-in and must never be described as the
//! real thing. The next person or agent to touch the project re-litigates all
//! of it and gets at least one wrong.
//!
//! So **anything with an `id` may carry a `note`**: an asset, a track, a clip.
//! One uniform rule, no argument about which things deserve one, and keyframe
//! tracks get none because they have no id — which falls out rather than being
//! decided.
//!
//! **A note is attached to an element and never to a span of time.** Most of
//! them are not about time at all: a note about an asset is true of the file in
//! every use of it, at any position, in any order. And a timed note
//! desynchronises silently the first time the cut is retimed — "make it 20%
//! faster" moves every clip and would leave every timed note pointing at the
//! wrong moment with nothing to notice it. A note on a clip moves with the clip
//! for free; a note on an asset does not move because it was never about a
//! moment.
//!
//! **A note dies with its element, and that is the feature.** Deleting a clip
//! deletes the reasoning for a clip that no longer exists — nothing to garbage
//! collect and nothing left dangling.
//!
//! The other half of this is [`crate::Project::script`], and the two are not
//! interchangeable: a script is one document about what the whole thing has to
//! be, read start to finish and surviving a total re-cut; a note is one
//! sentence about one element, read only when looking at that element, and gone
//! when it is.
//!
//! **Neither ever renders.** See [`crate::Track::note`].

use std::fmt;

use crate::asset::Asset;
use crate::timeline::{Clip, Track};

/// What sort of thing a note is attached to.
///
/// Carried alongside the id because ids are unique within their own table and
/// not across the document: a reader told only `03-mosaic` cannot tell whether
/// that is the asset or a clip named after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Annotated {
    /// A row of the assets table — a note about the media itself.
    Asset,
    /// A lane — a note about what belongs in it and why.
    Track,
    /// One placement on a lane — a note about this shot's own choices.
    Clip,
}

/// One note, with enough context to say what it is about.
///
/// Borrowed rather than owned: this is produced by walking a project that the
/// caller already has, and copying every note out of a document to list them
/// would be copying the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Noted<'a> {
    /// Which table the element lives in.
    pub on: Annotated,
    /// The element's `id`, exactly as the document spells it.
    pub id: &'a str,
    /// What was written about it.
    pub note: &'a str,
}

impl<'a> Noted<'a> {
    pub(crate) fn on_asset(asset: &'a Asset) -> Option<Self> {
        Some(Self {
            on: Annotated::Asset,
            id: asset.id.as_str(),
            note: asset.note.as_deref()?,
        })
    }

    pub(crate) fn on_track(track: &'a Track) -> Option<Self> {
        Some(Self {
            on: Annotated::Track,
            id: track.id.as_str(),
            note: track.note.as_deref()?,
        })
    }

    pub(crate) fn on_clip(clip: &'a Clip) -> Option<Self> {
        Some(Self {
            on: Annotated::Clip,
            id: clip.id.as_str(),
            note: clip.note.as_deref()?,
        })
    }
}

impl fmt::Display for Annotated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `pad` rather than `write_str`, so `{:<5}` in a table actually aligns.
        f.pad(match self {
            Self::Asset => "asset",
            Self::Track => "track",
            Self::Clip => "clip",
        })
    }
}

impl fmt::Display for Noted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} `{}`: {}", self.on, self.id, self.note)
    }
}
