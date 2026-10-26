//! Prompt-backed assets, in each state of the sketch lifecycle.
//!
//! The lifecycle is what these fixtures are for, so the state is always
//! spelled out at the call site and the file that goes with it follows from
//! the state: `generated` and `stale` have media on disk, `sketch` and
//! `queued` do not. A `stale` asset **with a real file beside it** is the one
//! that matters most — showing that file instead of a card would be showing a
//! shot the prompt no longer asks for.

use scorsese_core::{Asset, AssetId, AssetKind, GenerationState, ProjectPath};

/// The prompt every video fixture carries, short enough to read on a card at a
/// fixture's raster.
pub(crate) const SHOT_PROMPT: &str = "a skyline at dusk";

/// The prompt every narration fixture carries.
pub(crate) const NARRATION_PROMPT: &str = "and then the city woke up";

/// A Veo prompt nobody has spent money on yet.
pub(crate) fn sketch_asset(id: &str) -> Asset {
    Asset::sketch(AssetId::new(id), AssetKind::GeneratedVideo, SHOT_PROMPT)
}

/// An ElevenLabs narration prompt, likewise still a sketch. Belongs on an
/// audio track, and until it is generated it is silence there — and a card on
/// the picture.
pub(crate) fn narration_asset(id: &str) -> Asset {
    Asset::sketch(
        AssetId::new(id),
        AssetKind::GeneratedAudio,
        NARRATION_PROMPT,
    )
}

/// A prompt in `state`, carrying the generated file that state implies.
///
/// The path is under `generated/`, where provider output lives, and is named
/// after the asset so a fixture that conjures media for it agrees without
/// being told.
pub(crate) fn in_state(id: &str, kind: AssetKind, state: GenerationState) -> Asset {
    let extension = if kind == AssetKind::GeneratedAudio {
        "wav"
    } else {
        "mp4"
    };
    Asset {
        state: Some(state),
        path: state
            .has_media()
            .then(|| ProjectPath::new(format!("generated/{id}.{extension}"))),
        ..if kind == AssetKind::GeneratedAudio {
            narration_asset(id)
        } else {
            sketch_asset(id)
        }
    }
}

/// A prompt whose media exists and no longer answers it: the prompt was edited
/// after it was generated. The file is still on disk, which is exactly why
/// this state has to be told apart from `generated`.
pub(crate) fn stale_asset(id: &str) -> Asset {
    Asset {
        path: Some(ProjectPath::new(format!("generated/{id}.mp4"))),
        ..in_state(id, AssetKind::GeneratedVideo, GenerationState::Stale)
    }
}

/// A prompt that has been generated: the state a render decodes rather than
/// draws.
pub(crate) fn generated_asset(id: &str, kind: AssetKind) -> Asset {
    in_state(id, kind, GenerationState::Generated)
}
