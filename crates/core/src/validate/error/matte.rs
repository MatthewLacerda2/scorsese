//! What the matte checks can find: a matte reference that names nothing a
//! matte can be.

use crate::timeline::ClipId;

/// One thing wrong with a clip's `matte`.
///
/// Its own catalogue because a matte is a reference between two clips, and
/// what can be wrong with it is about the pair rather than either clip alone.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MatteProblem {
    /// A matte naming a clip that is nowhere in the document.
    #[error("clip `{clip}` is masked by clip `{matte}`, which does not exist")]
    Missing {
        /// The masked clip.
        clip: ClipId,
        /// The id it names.
        matte: ClipId,
    },

    /// A clip masked by itself. It would have to be drawn to know where to be
    /// drawn, and is never drawn because it is a matte.
    #[error("clip `{clip}` is masked by itself; a matte is another clip whose picture is the mask")]
    Itself {
        /// The clip.
        clip: ClipId,
    },

    /// A matte on the other side of a group's edge from the clip it masks.
    ///
    /// **Refused, for the reason an arrow across the edge is.** Inside a group
    /// a matte is drawn in the group's own space, before the group's
    /// transform; outside, on the frame, after it — and the same group can be
    /// placed twice, so "the member" would not even name one place. Put the
    /// matte beside the clip it masks, or mask the group clip itself, which
    /// reveals the whole group.
    #[error(
        "clip `{clip}` is masked by clip `{matte}` across a group's edge — a matte must be on \
         the same timeline as the clip it masks: both inside one group, or both outside it"
    )]
    Across {
        /// The masked clip.
        clip: ClipId,
        /// The matte, on the other side.
        matte: ClipId,
    },

    /// A matte on an audio track, or a masked clip on one. A matte is about
    /// picture: a clip on an audio track has none to mask or to be a mask.
    #[error(
        "clip `{clip}` is masked by clip `{matte}`, but `{on_sound}` is on an audio track — a \
         matte and the clip it masks are both picture, on video tracks"
    )]
    NotPicture {
        /// The masked clip.
        clip: ClipId,
        /// The matte.
        matte: ClipId,
        /// Which of the two is on an audio track.
        on_sound: ClipId,
    },

    /// A matte that has a matte of its own.
    ///
    /// **Refused rather than resolved**: a chain of mattes on mattes is a
    /// compositing graph, and the one thing a person reaching for "show this
    /// through that" never meant. Draw the two masks as one group and use the
    /// group clip as the matte instead.
    #[error(
        "clip `{clip}` is masked by clip `{matte}`, which is masked by `{next}` in turn — a \
         matte cannot have a matte of its own; group the shapes and use the group clip instead"
    )]
    Chained {
        /// The masked clip.
        clip: ClipId,
        /// Its matte.
        matte: ClipId,
        /// The matte's own matte.
        next: ClipId,
    },
}
