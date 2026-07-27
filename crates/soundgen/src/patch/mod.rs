//! The patch document: one instrument, as serde data.
//!
//! A [`Patch`] is a "Minimoog-lite" description of a single sound. It is
//! **structured, not a free graph**: the signal path is fixed —
//! `source → filter → amp envelope → fx`, with an LFO tapping one target —
//! because a patch has to honour a contract, *playable as a note*. The recipe
//! chooses what goes in each stage, never how the stages connect.
//!
//! That is not a simplification for its own sake. A free graph would let a
//! recipe describe something that is not a note — a feedback loop with no
//! output, a stage that never terminates — and the whole point of a patch is
//! that `(pitch, velocity, duration) in → buffer out` always holds.
//!
//! Patch-as-truth: no buffers and no handles, so it round-trips losslessly.
//! [`crate::core`] renders it.

pub mod stages;

use serde::{Deserialize, Serialize};

use crate::error::SynthError;

pub use stages::{Adsr, Filter, FilterKind, Fx, Lfo, LfoTarget, MAX_OSCS, Osc, Source, Wave};

/// One instrument.
///
/// `source` and `amp` are mandatory — a sound needs a tone and a shape.
/// `filter`, `lfo` and `fx` are optional stages.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    /// What makes the raw tone.
    pub source: Source,
    /// The amplitude envelope: what turns a continuous tone into a note.
    pub amp: Adsr,
    /// The optional filter stage, with its own envelope.
    #[serde(default)]
    pub filter: Option<Filter>,
    /// The optional low-frequency oscillator, bending one stage.
    #[serde(default)]
    pub lfo: Option<Lfo>,
    /// The post-chain, applied in list order.
    #[serde(default)]
    pub fx: Vec<Fx>,
}

impl Patch {
    /// Serialises to pretty JSON — the on-disk recipe form.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parses a patch from its JSON form.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Rejects a patch the renderer cannot honour, *before* any samples are
    /// produced.
    ///
    /// Only the bounds that would produce silence, a divide-by-zero or an
    /// unstable filter are checked. Musical taste is the recipe's business: an
    /// ugly patch renders.
    pub fn validate(&self) -> Result<(), SynthError> {
        self.check_source()?;
        if let Some(filter) = &self.filter
            && filter.cutoff <= 0.0
        {
            return Err(SynthError::BadCutoff {
                cutoff: filter.cutoff,
            });
        }
        if let Some(lfo) = &self.lfo
            && lfo.rate < 0.0
        {
            return Err(SynthError::NegativeLfoRate { rate: lfo.rate });
        }
        Ok(())
    }

    /// The head of the signal path, which is where every unrenderable patch so
    /// far has gone wrong.
    fn check_source(&self) -> Result<(), SynthError> {
        match &self.source {
            Source::OscStack { oscs } if oscs.is_empty() => Err(SynthError::EmptyOscStack),
            Source::OscStack { oscs } if oscs.len() > MAX_OSCS => Err(SynthError::TooManyOscs {
                found: oscs.len(),
                limit: MAX_OSCS,
            }),
            Source::OscStack { oscs } if oscs.iter().all(|osc| osc.gain <= 0.0) => {
                Err(SynthError::SilentOscStack)
            }
            Source::Fm2 { ratio, .. } if *ratio <= 0.0 => {
                Err(SynthError::BadFmRatio { ratio: *ratio })
            }
            _ => Ok(()),
        }
    }
}
