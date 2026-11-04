//! The three blocks a `generated_audio` asset can carry, with every field
//! set, for [`super::document`]'s asset.

use scorsese_core::{MusicRequest, SoundRequest, SpeechModel, SpeechRequest};

/// A spoken line's block.
pub(crate) fn speech() -> SpeechRequest {
    SpeechRequest {
        model: SpeechModel::Expressive,
        voice_id: Some("voice".to_owned()),
        language: Some("pt".to_owned()),
        seed: Some(7),
    }
}

/// A generated track's block.
pub(crate) fn music() -> MusicRequest {
    MusicRequest {
        seconds: Some(40),
        instrumental: false,
    }
}

/// A generated sound effect's block.
pub(crate) fn sound() -> SoundRequest {
    SoundRequest {
        seconds: Some(2.5),
        looping: true,
    }
}
