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
//! [`render_note`](crate::render_note) renders it.

pub(crate) mod stages;

use serde::{Deserialize, Serialize};

use crate::error::SynthError;

pub use stages::{
    Adsr, EqBand, EqKind, Filter, FilterKind, Fx, Lfo, LfoTarget, MAX_EQ_BANDS, MAX_OSCS, Osc,
    PitchEnv, Source, Wave,
};

/// Rejects an fx chain the renderer cannot honour.
///
/// It lives beside the patch rather than inside it because a chain lives in
/// **three** places — a patch, a track and the song — and one check for all
/// three is what stops the cap being a property of where the chain happens to
/// be written. The song's own validation calls this for the other two.
pub(crate) fn check_chain(chain: &[Fx]) -> Result<(), SynthError> {
    for fx in chain {
        if let Fx::Eq { bands } = fx
            && bands.len() > MAX_EQ_BANDS
        {
            return Err(SynthError::TooManyEqBands {
                found: bands.len(),
                limit: MAX_EQ_BANDS,
            });
        }
    }
    Ok(())
}

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
    /// An optional one-shot sweep of the note's own pitch. Absent means the
    /// note holds the pitch it was played at, which is what every patch
    /// written before this stage existed already meant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pitch_env: Option<PitchEnv>,
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
        check_chain(&self.fx)
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
