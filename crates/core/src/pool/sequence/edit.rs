//! Making an image sequence from stills already in the pool, or changing one.
//!
//! The other half of a sequence's life from [`super::import_sequence`]: the
//! stills may have arrived any way at all — imported one by one, brought in
//! from a hosted library — and this is what turns a set of them into a
//! sequence, or retimes one that exists. One verb for both, because they are
//! the same three fields, and "make this a loop at four frames a drawing" is
//! one sentence whether or not the sequence existed before it was said.
//!
//! **Nothing is changed unless the whole document still validates**, so a
//! still that is not a picture, or a hold of nothing, is refused here with the
//! project as it was.

use crate::asset::{Asset, AssetId, AssetKind, ImageSequence};
use crate::project::Project;
use crate::time::Frames;
use crate::validate::ValidationErrors;

/// What to change on a sequence — or, for one that does not exist yet, what
/// to make it from. A field left `None` is left as it is.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SequenceChange {
    /// The stills, in the order they play — replacing the whole list.
    pub stills: Option<Vec<AssetId>>,
    /// How many timeline frames each still is held.
    pub hold: Option<Frames>,
    /// Whether it starts again when it runs out.
    pub looping: Option<bool>,
}

/// What [`change_sequence`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceChanged {
    /// The sequence as it was, or `None` when this call made it.
    pub before: Option<ImageSequence>,
    /// The sequence as it is now.
    pub after: ImageSequence,
}

/// Why a sequence could not be made or changed. The project is untouched.
#[derive(Debug, thiserror::Error)]
pub enum SequenceError {
    /// The id names an asset of another kind.
    #[error("asset `{asset}` is a {kind:?}, not an image sequence")]
    NotASequence {
        /// The asset.
        asset: AssetId,
        /// What it is.
        kind: AssetKind,
    },
    /// A new sequence with no stills named.
    #[error("there is no asset `{asset}` yet, and making it needs its stills")]
    NoStills {
        /// The id that would have been made.
        asset: AssetId,
    },
    /// The change leaves a document that does not validate.
    #[error("{0}")]
    Invalid(ValidationErrors),
}

/// Changes the sequence `id`, or makes it when no asset answers to that id.
pub fn change_sequence(
    project: &mut Project,
    id: &AssetId,
    change: SequenceChange,
) -> Result<SequenceChanged, SequenceError> {
    let mut proposed = project.clone();
    let before = match proposed.assets.iter().find(|asset| &asset.id == id) {
        Some(asset) if asset.kind != AssetKind::ImageSequence => {
            return Err(SequenceError::NotASequence {
                asset: id.clone(),
                kind: asset.kind,
            });
        }
        Some(asset) => asset.sequence.clone(),
        None => {
            let Some(stills) = change.stills.clone() else {
                return Err(SequenceError::NoStills { asset: id.clone() });
            };
            let made = Asset::image_sequence(id.clone(), ImageSequence::new(stills));
            proposed.assets.push(made);
            None
        }
    };
    let asset = proposed
        .assets
        .iter_mut()
        .find(|asset| &asset.id == id)
        .expect("the sequence was found or made above");
    let sequence = asset
        .sequence
        .get_or_insert_with(|| ImageSequence::new(Vec::new()));
    if let Some(stills) = change.stills {
        sequence.stills = stills;
    }
    if let Some(hold) = change.hold {
        sequence.hold = hold;
    }
    if let Some(looping) = change.looping {
        sequence.looping = looping;
    }
    let after = sequence.clone();
    proposed.validate().map_err(SequenceError::Invalid)?;
    *project = proposed;
    Ok(SequenceChanged { before, after })
}
