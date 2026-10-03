//! The instrument library: named patches to start a song from.
//!
//! Every song made with scorsese used to begin by writing a kick from nothing —
//! five to ten lines of envelopes and curves that have to be right before the
//! sound reads as a kick at all, retyped or copied out of the last song's JSON.
//! This is the set those songs kept rebuilding: a drum machine's kick, snare,
//! closed hat and crash, and a synth bass, a clav, a brass section, a pad and an
//! electric piano. A recipe asks for one as `"patch": "kit:kick"`.
//!
//! **Copied on use, never referenced.** Writing a recipe through
//! [`expand`] replaces each `kit:` name with the patch itself, inline, so the
//! song carries its own copy from then on. That is the whole design, and the
//! three properties it buys are why:
//!
//! - **A song never changes sound because scorsese was upgraded.** Improving the
//!   kick here changes the *next* song that asks for it, never one already
//!   written; a referenced preset would rewrite every song on disk silently.
//! - **Editing a preset is not a `SYNTH_VERSION` bump.** The synthesiser's
//!   output for any given document is unchanged — only which document a name
//!   produces moves — and every existing document already holds its copy.
//! - **A project still survives `scp -r`.** Nothing in it points outside it.
//!
//! So at bake time a `kit:` name is a mistake rather than an instrument: the
//! resolver refuses it with the way to copy it in (see `instruments` in the
//! parent module), instead of reaching for a library a project must not depend
//! on.
//!
//! **Compiled in, beside `core`.** The patches are JSON files in `presets/`,
//! included at build time: one source of truth that reads exactly like what
//! gets copied, and that every caller — the CLI, the MCP server, the web
//! server — has without an install path to find. `core` never sees them
//! (it defines what a patch *can* be, never what one *is*), and neither does
//! `zimmer`: a library of starting points is a fact about making scorsese
//! projects, not about rendering sound, and putting it there would make it
//! part of the API rusty pins.

use serde_json::Value;

use super::recipe::Recipe;
use scorsese_zimmer::Patch;

/// The prefix that marks a track's patch as a library name rather than a path.
pub const PREFIX: &str = "kit:";

/// One instrument in the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instrument {
    /// What a recipe calls it, after [`PREFIX`].
    pub name: &'static str,
    /// What it sounds like and what it is for, in a line.
    pub describes: &'static str,
    /// The note a one-shot of it is written at. Drums sound as themselves on
    /// any note, and on middle C — what a run of steps plays when it names no
    /// note — above all.
    pub home: &'static str,
    json: &'static str,
}

impl Instrument {
    /// The patch, as the document a recipe receives.
    pub fn patch(&self) -> Patch {
        Patch::from_json(self.json).expect("every preset parses — `kit` tests hold it")
    }

    /// The patch as JSON — what `kit:<name>` becomes.
    pub fn json(&self) -> &'static str {
        self.json
    }
}

/// Every instrument, drums first.
pub const KIT: &[Instrument] = &[
    Instrument {
        name: "kick",
        describes: "drum-machine kick: a sine falling onto its pitch, with a little drive",
        home: "C4",
        json: include_str!("presets/kick.json"),
    },
    Instrument {
        name: "snare",
        describes: "drum-machine snare: a crack of pink noise over a short body, in a small room",
        home: "C4",
        json: include_str!("presets/snare.json"),
    },
    Instrument {
        name: "hat",
        describes: "closed hi-hat: a short tick of bright noise",
        home: "C4",
        json: include_str!("presets/hat.json"),
    },
    Instrument {
        name: "crash",
        describes: "crash cymbal: a long wash of bright noise that darkens as it fades",
        home: "C4",
        json: include_str!("presets/crash.json"),
    },
    Instrument {
        name: "bass",
        describes: "synth bass: saw and square sub through a plucky lowpass, warm and round",
        home: "E2",
        json: include_str!("presets/bass.json"),
    },
    Instrument {
        name: "clav",
        describes: "clavinet: a bright, funky, percussive keyboard that barks when played hard",
        home: "C4",
        json: include_str!("presets/clav.json"),
    },
    Instrument {
        name: "brass",
        describes: "brass section: FM horns that bloom into the note, brighter when played hard",
        home: "F3",
        json: include_str!("presets/brass.json"),
    },
    Instrument {
        name: "pad",
        describes: "pad: a slow, wide, chorused saw bed that breathes",
        home: "C3",
        json: include_str!("presets/pad.json"),
    },
    Instrument {
        name: "epiano",
        describes: "electric piano: an FM tine that rings and fades, bell-like when struck hard",
        home: "C4",
        json: include_str!("presets/epiano.json"),
    },
];

/// The instrument a name asks for, written with or without [`PREFIX`].
pub fn lookup(name: &str) -> Option<&'static Instrument> {
    let bare = name.strip_prefix(PREFIX).unwrap_or(name);
    KIT.iter().find(|instrument| instrument.name == bare)
}

/// A recipe with its library names copied in, and which ones were.
#[derive(Debug, Clone, PartialEq)]
pub struct Expanded {
    /// The recipe's JSON. Byte-for-byte what was given when nothing was
    /// copied, so a document without a `kit:` name is never reformatted.
    pub json: String,
    /// The recipe it parses to.
    pub recipe: Recipe,
    /// The instruments copied, in the order they were met — one entry per
    /// place, so a kit named by two tracks is listed twice.
    pub copied: Vec<&'static str>,
}

/// Why a recipe could not be expanded.
#[derive(Debug, thiserror::Error)]
pub enum KitError {
    /// Not a recipe, whatever it names.
    #[error("{0}")]
    Malformed(#[from] serde_json::Error),
    /// A `kit:` name the library does not have.
    #[error("there is no `{PREFIX}{name}` — the kit has {}", names())]
    Unknown {
        /// The name after the prefix, as written.
        name: String,
    },
}

/// Replaces every `kit:` name in a recipe with a copy of that instrument.
///
/// It looks where a patch goes and nowhere else: each song track's `patch`,
/// and a one-shot's `patch`. A one-shot is included because "one hit of the
/// kick" is the most ordinary effect there is, and its `patch` field could not
/// hold a name before this.
///
/// Done on the JSON rather than on a parsed recipe for that reason: a
/// one-shot's patch is not a reference in the format, so a name there would
/// not parse until it has been replaced.
pub fn expand(json: &str) -> Result<Expanded, KitError> {
    let mut document: Value = serde_json::from_str(json)?;
    let mut copied = Vec::new();
    if let Some(tracks) = document.get_mut("tracks").and_then(Value::as_array_mut) {
        for track in tracks {
            if let Some(slot) = track.get_mut("patch") {
                copy_into(slot, &mut copied)?;
            }
        }
    } else if let Some(slot) = document.get_mut("patch") {
        copy_into(slot, &mut copied)?;
    }

    if copied.is_empty() {
        let recipe = Recipe::from_json(json)?;
        return Ok(Expanded {
            json: json.to_owned(),
            recipe,
            copied,
        });
    }
    let recipe: Recipe = serde_json::from_value(document)?;
    // Written back from the parsed recipe rather than from the edited value,
    // so the fields come out in the format's own order rather than sorted.
    let json = recipe.to_json()?;
    Ok(Expanded {
        json,
        recipe,
        copied,
    })
}

/// Replaces one patch slot, if it holds a library name.
fn copy_into(slot: &mut Value, copied: &mut Vec<&'static str>) -> Result<(), KitError> {
    let Some(name) = slot.as_str().and_then(|it| it.strip_prefix(PREFIX)) else {
        return Ok(());
    };
    let instrument = lookup(name).ok_or_else(|| KitError::Unknown {
        name: name.to_owned(),
    })?;
    *slot = serde_json::from_str(instrument.json)?;
    copied.push(instrument.name);
    Ok(())
}

/// Every name in the kit, for a message that lists them.
pub(crate) fn names() -> String {
    KIT.iter()
        .map(|instrument| instrument.name)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests;
