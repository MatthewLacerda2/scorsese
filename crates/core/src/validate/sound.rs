//! Generated-sound checks: one block per asset, and a length the vendor
//! makes (#1265).

use crate::asset::{Asset, AssetKind, MAX_SOUND_SECONDS, MIN_SOUND_SECONDS, SoundRequest};

use super::error::{AssetProblem, SoundProblem};
use super::field::AssetField;

pub(super) fn check(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let Some(sound) = &asset.sound else {
        return;
    };
    if asset.kind != AssetKind::GeneratedAudio {
        errors.push(AssetProblem::StrayField {
            asset: asset.id.clone(),
            field: AssetField::Sound,
            kind: asset.kind,
        });
        return;
    }
    let other = if asset.speech.is_some() {
        Some(AssetField::Speech)
    } else if asset.music.is_some() {
        Some(AssetField::Music)
    } else {
        None
    };
    if let Some(other) = other {
        errors.push(
            SoundProblem::TwoBlocks {
                asset: asset.id.clone(),
                other,
            }
            .into(),
        );
    }
    if let Some(seconds) = sound.seconds
        && !SoundRequest::takes(seconds)
    {
        errors.push(
            SoundProblem::Length {
                asset: asset.id.clone(),
                found: seconds,
                min: MIN_SOUND_SECONDS,
                max: MAX_SOUND_SECONDS,
            }
            .into(),
        );
    }
}
