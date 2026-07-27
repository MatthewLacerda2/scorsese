//! The recipe document: what a `synth_audio` asset is made from.
//!
//! One file, one asset, one of two shapes — an effect or a piece of music —
//! told apart by a `recipe` field rather than by guessing from the keys
//! present. Guessing would work today and break the first time the two
//! documents grow a field in common.
//!
//! ```jsonc
//! { "recipe": "patch", "note": "C4", "duration": 0.4, "patch": { … } }
//! { "recipe": "song",  "bpm": 120, "tracks": [ … ], "patterns": { … }, … }
//! ```

use serde::{Deserialize, Serialize};

use scorsese_soundgen::song::Pitch;
use scorsese_soundgen::{NoteOpts, Patch, Song};

/// What a recipe describes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "recipe", rename_all = "snake_case")]
pub enum Recipe {
    /// One note of one instrument: a gunshot, a footstep, a UI blip.
    Patch(OneShot),
    /// A piece of music.
    Song(Song),
}

/// One rendered note of a patch.
///
/// The note is part of the document rather than a render-time argument,
/// because *which pitch* is part of what the asset **is** — a footstep and a
/// gunshot can be the same patch family at different pitches, and the asset
/// has to pin one. It also keeps the whole bake a function of the file, which
/// is what lets the file's hash name the output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OneShot {
    /// The pitch to render, as a name (`"C#4"`) or a MIDI number.
    pub note: Pitch,
    /// Gate length in seconds. The patch's release rings out after it, so the
    /// finished file is longer than this.
    #[serde(default = "default_duration")]
    pub duration: f32,
    /// Striking force in `0..=1`.
    #[serde(default = "default_velocity")]
    pub velocity: f32,
    /// Seed for the stochastic sources. Changing it re-rolls the noise while
    /// keeping the bake reproducible.
    #[serde(default)]
    pub seed: u64,
    /// The instrument itself.
    pub patch: Patch,
}

impl OneShot {
    /// The per-note options this describes.
    pub fn opts(&self) -> NoteOpts {
        NoteOpts {
            duration: self.duration,
            velocity: self.velocity,
            seed: self.seed,
        }
    }
}

impl Recipe {
    /// Parses a recipe from its JSON form.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialises to pretty JSON — the on-disk form.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        Ok(json)
    }

    /// What this recipe is called in a report, in the words the format uses.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Patch(_) => "patch",
            Self::Song(_) => "song",
        }
    }
}

fn default_duration() -> f32 {
    0.5
}

fn default_velocity() -> f32 {
    1.0
}
