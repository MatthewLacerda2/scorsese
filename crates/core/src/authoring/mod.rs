//! Writing down what nothing brings in: an asset with no file behind it, and a
//! lane to put it on.
//!
//! Four of the asset kinds exist precisely so that a cut does not need a file
//! for everything — a caption, a colour card, a box, a symbol. Nothing imports
//! one, nothing generates one, and no provider is asked about one: the document
//! *is* the asset. So the only way one has ever come into a project is by
//! somebody writing the whole document again, which for a captioned cut is
//! tens of kilobytes per line of text.
//!
//! That is what these operations are for:
//!
//! - [`add_asset`] puts one inline asset in the table.
//! - [`set_asset`] changes a field on one that is already there.
//! - [`add_track`] adds a lane, because an asset with nowhere to sit is not
//!   yet in the cut and "clips on one track may not overlap" makes a second
//!   lane the answer far more often than a rearrangement does.
//!
//! - [`remove_asset`] and [`remove_track`] take one out again, with the clips
//!   that go with it — and only when the caller has named those clips.
//!
//! **All of them are all-or-nothing**, exactly as [`crate::placing`] is: the
//! change is made on a copy, and only a copy [`crate::Project::validate`]
//! accepts becomes the document. A style that names a font path nobody could
//! resolve, or a shape that would draw nothing at all, leaves the project byte
//! for byte as it was.
//!
//! **Deletion names what it destroys.** Removing an asset has to say what
//! becomes of the clips that show it, and the answer (#396) is that they go
//! with it only when the caller lists them exactly — see [`remove_asset`].

mod apply;
mod block;
mod edit;
mod inline;
mod remove;
mod track;

pub use block::{BlockChange, CounterEdit, RevealEdit};
pub use edit::{Edit, set_asset};
pub use inline::{Inline, add_asset};
pub use remove::{AssetRemoval, remove_asset, remove_track};
pub(crate) use track::numbered;
pub use track::{Lane, add_track};

use crate::asset::{AssetId, AssetKind};
use crate::timeline::{ClipId, TrackId};
use crate::validate::ValidationErrors;

/// Why nothing was written. Every one of these leaves the project untouched.
#[derive(Debug, thiserror::Error)]
pub enum AuthorError {
    /// The id asked for is already an asset's. Refused rather than suffixed:
    /// an id a caller chose is one it is about to write into a clip, and
    /// quietly handing back a different one is how a clip shows the wrong
    /// thing. An id left unsaid is derived and suffixed instead.
    #[error("`{id}` is already the id of an asset in this project")]
    TakenAssetId {
        /// The id that was asked for.
        id: AssetId,
    },
    /// The id asked for is already a track's, and track ids are what clips are
    /// filed under.
    #[error("`{id}` is already the id of a track in this project")]
    TakenTrackId {
        /// The id that was asked for.
        id: TrackId,
    },
    /// No asset of that id. Refused rather than created, for
    /// [`crate::PlaceError::NoSuchTrack`]'s reason: an asset invented from a
    /// typo is one nobody is looking at.
    #[error("no asset in this project is called `{asset}`")]
    NoSuchAsset {
        /// The id that was asked for.
        asset: AssetId,
    },
    /// No track of that id, on the timeline or in any group.
    #[error("no track in this project is called `{track}`")]
    NoSuchTrack {
        /// The id that was asked for.
        track: TrackId,
    },
    /// The clips named are not exactly the clips showing the asset. Removing
    /// it takes every one of those with it, so the caller has to name them —
    /// no more and no fewer — and this says which they are.
    #[error(
        "`{asset}` is shown by {}; removing it removes those clips too, so name exactly \
         them in `clips` (this call named {}) — nothing was removed",
        remove::listed(using),
        remove::listed(named)
    )]
    AssetInUse {
        /// The asset that was named.
        asset: AssetId,
        /// Every clip that shows it, in id order — what the next call names.
        using: Vec<ClipId>,
        /// What this call named instead.
        named: Vec<ClipId>,
    },
    /// The asset is a still an image sequence plays. Not a clip, so naming
    /// clips cannot take it: the sequence has to stop playing it first (or go
    /// itself), or it would name a still that is not there.
    #[error(
        "`{asset}` is a still played by the image sequence {}; change the sequence's \
         stills, or remove the sequence, first — nothing was removed",
        sequences.iter().map(|id| format!("`{id}`")).collect::<Vec<_>>().join(", ")
    )]
    PlayedBySequence {
        /// The still that was named.
        asset: AssetId,
        /// Every sequence that plays it, in table order.
        sequences: Vec<AssetId>,
    },
    /// The clips named are not exactly the clips on the track, for
    /// [`AuthorError::AssetInUse`]'s reason one level up.
    #[error(
        "track `{track}` holds {}; removing it removes those clips too, so name exactly \
         them in `clips` (this call named {}) — nothing was removed",
        remove::listed(holding),
        remove::listed(named)
    )]
    TrackNotEmpty {
        /// The track that was named.
        track: TrackId,
        /// Every clip on it, in id order — what the next call names.
        holding: Vec<ClipId>,
        /// What this call named instead.
        named: Vec<ClipId>,
    },
    /// The asset exists and is not one of the four kinds whose content lives
    /// in the document. What a video or a brief *is* lives elsewhere — in a
    /// file, or in a prompt `rebrief` edits — so there is no field here to set.
    #[error(
        "`{asset}` is a `{kind:?}` asset, and only text, color, shape and icon assets \
         carry their content in the document"
    )]
    NotInline {
        /// The asset that was named.
        asset: AssetId,
        /// What it turned out to be.
        kind: AssetKind,
    },
    /// A field that means nothing to this kind. Refused rather than ignored:
    /// a `fill` set on a caption would look exactly like a fill that had been
    /// applied, and the difference would only ever be found by rendering.
    #[error("`{field}` is not a field of a {kind:?} asset — that kind takes {takes}")]
    NotOnKind {
        /// The field that was asked for.
        field: &'static str,
        /// The kind of the asset it was asked of.
        kind: AssetKind,
        /// What that kind does take, so the next call can name one.
        takes: String,
    },
    /// A field that means nothing to this *outline*. A rectangle has a corner
    /// radius and an arrow has no size at all, so both are refused by name
    /// rather than written into a document nothing would read them back from.
    #[error("`{field}` is not a field of {geometry} — {because}")]
    NotOnGeometry {
        /// The field that was asked for.
        field: &'static str,
        /// The outline it was asked of, as it reads in a sentence.
        geometry: &'static str,
        /// Why that outline has no such field.
        because: &'static str,
    },
    /// A gradient given to a field that is one colour: a caption's or an
    /// icon's. Refused rather than flattened to one of its stops, which would
    /// be a colour nobody chose.
    #[error("`{field}` on a {kind:?} asset is one colour, not a gradient")]
    NotOneColour {
        /// The field that was asked for.
        field: &'static str,
        /// The kind of the asset it was asked of.
        kind: AssetKind,
    },
    /// An edit that names no field at all. Not an empty success: a caller that
    /// meant to change something and spelled the argument wrong would
    /// otherwise be told the write went fine.
    #[error("nothing to change — name at least one field to set")]
    NothingAsked,
    /// The asset's kind and its content disagree — a `shape` asset with no
    /// shape in it. Unreachable through a loaded project, which is validated
    /// on the way in, and said out loud rather than assumed away.
    #[error("`{asset}` is a {kind:?} asset carrying no {kind:?} block, so there is nothing to set")]
    Incoherent {
        /// The asset that is missing its own content.
        asset: AssetId,
        /// What it claims to be.
        kind: AssetKind,
    },
    /// The result was not a document that loads.
    #[error(transparent)]
    Refused(#[from] ValidationErrors),
}

/// One document carrying an asset of each inline kind, so the tests on either
/// side of the split are written against the same four rather than against
/// four fixtures that drift apart.
#[cfg(test)]
mod fixture {
    use crate::project::Project;

    const DOCUMENT: &str = r##"{
      "schema_version": 44,
      "name": "T",
      "timeline_fps": { "num": 30, "den": 1 },
      "assets": [
        { "id": "caption", "kind": "text", "text": "DAWN",
          "style": { "font": "serif", "size": 0.12 } },
        { "id": "card", "kind": "color", "color": "#101820" },
        { "id": "box", "kind": "shape",
          "shape": { "geometry": { "rectangle": { "width": 0.4, "height": 0.2 } },
                     "fill": "#ffffff" } },
        { "id": "mark", "kind": "icon",
          "icon": { "name": "clapperboard", "size": 0.2, "color": "#ffffff" } },
        { "id": "shot", "kind": "video", "path": "assets/shot.mp4" }
      ],
      "tracks": []
    }"##;

    /// A caption in a serif face at 0.12, a colour card, a filled rectangle,
    /// an icon, and one asset that is none of those.
    pub(super) fn project() -> Project {
        Project::from_json(DOCUMENT).expect("the fixture is a project")
    }
}
