//! Generated-music checks: one block per asset, and a length the vendor
//! composes (#1264).

use crate::asset::{Asset, AssetKind, MAX_MUSIC_SECONDS, MIN_MUSIC_SECONDS};

use super::error::{AssetProblem, MusicProblem};
use super::field::AssetField;

pub(super) fn check(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let Some(music) = &asset.music else {
        return;
    };
    if asset.kind != AssetKind::GeneratedAudio {
        errors.push(AssetProblem::StrayField {
            asset: asset.id.clone(),
            field: AssetField::Music,
            kind: asset.kind,
        });
        return;
    }
    if asset.speech.is_some() {
        errors.push(
            MusicProblem::BothBlocks {
                asset: asset.id.clone(),
            }
            .into(),
        );
    }
    if let Some(seconds) = music.seconds
        && !(MIN_MUSIC_SECONDS..=MAX_MUSIC_SECONDS).contains(&seconds)
    {
        errors.push(
            MusicProblem::Length {
                asset: asset.id.clone(),
                found: seconds,
                min: MIN_MUSIC_SECONDS,
                max: MAX_MUSIC_SECONDS,
            }
            .into(),
        );
    }
}
