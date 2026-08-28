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

pub(crate) mod fm;
pub(crate) mod stages;

use serde::{Deserialize, Serialize};

use crate::error::SynthError;

pub use fm::{Algorithm, FM_OPERATORS, Operator};
pub use stages::{
    Adsr, EqBand, EqKind, Filter, FilterKind, Fx, Lfo, LfoTarget, MAX_EQ_BANDS, MAX_OSCS,
    MAX_PARTIALS, MAX_VOICES, NoiseColor, Osc, Partial, PitchEnv, Source, Wave,
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

/// Every track a chain is keyed from, in list order.
///
/// It lives beside the patch for the reason [`check_chain`] does: a chain is a
/// chain wherever it is written, and both the mixer that honours a sidechain
/// and the validation that refuses one read it through this.
pub(crate) fn sidechains(chain: &[Fx]) -> impl Iterator<Item = &str> {
    chain.iter().filter_map(|fx| match fx {
        Fx::Compress { sidechain, .. } => sidechain.as_deref(),
        _ => None,
    })
}

/// Rejects a chain that names a track from somewhere no track can be named.
///
/// A patch's chain runs per note and the song's runs on the sum; in neither
/// does a track exist as a thing to listen to. Refusing is the crate's usual
/// answer to what it cannot honour — a silently ignored `sidechain` would be a
/// duck the recipe wrote and never got, which is exactly the failure the song
/// validator exists to prevent.
pub(crate) fn check_no_sidechain(chain: &[Fx], place: &'static str) -> Result<(), SynthError> {
    match sidechains(chain).next() {
        Some(key) => Err(SynthError::MisplacedSidechain {
            place,
            key: key.to_owned(),
        }),
        None => Ok(()),
    }
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
        check_chain(&self.fx)?;
        check_no_sidechain(&self.fx, "patch")
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
            // Both ends of the unison range, in one pass: no voices at all is
            // an oscillator the recipe asked to be silent without saying so,
            // and too many is [`MAX_VOICES`]'s argument.
            Source::OscStack { oscs } => match oscs
                .iter()
                .find(|osc| !(1..=MAX_VOICES).contains(&osc.voices))
            {
                Some(osc) => Err(SynthError::BadVoiceCount {
                    found: osc.voices,
                    limit: MAX_VOICES,
                }),
                None => Ok(()),
            },
            Source::Fm2 { ratio, .. } if *ratio <= 0.0 => {
                Err(SynthError::BadFmRatio { ratio: *ratio })
            }
            Source::Fm4 {
                algorithm,
                operators,
                ..
            } => check_operators(*algorithm, operators),
            Source::Additive { partials } => check_partials(partials),
            _ => Ok(()),
        }
    }
}

/// Rejects an additive series the renderer cannot honour.
///
/// The same four questions `check_source` asks of an oscillator stack — is
/// there anything in it, is there too much of it, does each entry name a
/// frequency, and does any of it sound — because a series is the same kind of
/// table with the same ways of being wrong.
///
/// **Not asked: whether a partial is above Nyquist.** That depends on the
/// pitch played, and a patch is validated once while it is played at many
/// pitches, so refusing here would refuse a legal organ for the top note of a
/// part. The renderer drops those per note instead.
fn check_partials(partials: &[Partial]) -> Result<(), SynthError> {
    if partials.is_empty() {
        return Err(SynthError::EmptyPartials);
    }
    if partials.len() > MAX_PARTIALS {
        return Err(SynthError::TooManyPartials {
            found: partials.len(),
            limit: MAX_PARTIALS,
        });
    }
    if let Some((index, partial)) = partials
        .iter()
        .enumerate()
        .find(|(_, partial)| partial.ratio <= 0.0 || !partial.ratio.is_finite())
    {
        return Err(SynthError::BadPartialRatio {
            index,
            ratio: partial.ratio,
        });
    }
    if partials.iter().all(|partial| partial.gain <= 0.0) {
        return Err(SynthError::SilentPartials);
    }
    Ok(())
}

/// Rejects a four-operator source the renderer cannot honour.
///
/// Two of the four questions [`check_partials`] asks, because the shape of the
/// document has already answered the other two: an `fm4` source carries
/// exactly [`FM_OPERATORS`] operators as a fixed-size array, so it can be
/// neither empty nor oversized by the time serde has finished with it.
///
/// What is left is the same pair. Does each entry name a frequency — the
/// [`SynthError::BadPartialRatio`] question, and refusing a non-finite one for
/// the same reason. And does any of it sound: the
/// [`SynthError::SilentOscStack`] question, one indirection further away,
/// because which operators are audible is a property of the *algorithm* rather
/// than of the list. A recipe that picks [`Algorithm::Chain`] and then writes
/// its levels as if they were faders silences the whole source by zeroing the
/// one operator it happens to be heard through, with nothing on the page to
/// say so.
///
/// **Not asked: whether an operator is above Nyquist** — the same answer
/// [`check_partials`] gives, for the same reason. That depends on the pitch
/// played, and a patch is validated once while it is played at many pitches.
/// The renderer drops those per note instead.
fn check_operators(
    algorithm: Algorithm,
    operators: &[Operator; FM_OPERATORS],
) -> Result<(), SynthError> {
    if let Some((index, operator)) = operators
        .iter()
        .enumerate()
        .find(|(_, operator)| operator.ratio <= 0.0 || !operator.ratio.is_finite())
    {
        return Err(SynthError::BadOperatorRatio {
            // Numbered from **one**, unlike a partial's index, and the field
            // name says which: a partial's is its place in a list whose order
            // carries no meaning, while an operator's number is part of the
            // algorithm's own vocabulary. An error reading "operator 0" would
            // send a reader looking for a row that is not on the page.
            operator: index + 1,
            ratio: operator.ratio,
        });
    }
    if (0..FM_OPERATORS)
        .filter(|index| algorithm.is_carrier(*index))
        .all(|index| operators[index].level <= 0.0)
    {
        return Err(SynthError::SilentCarriers {
            algorithm: algorithm.name(),
        });
    }
    Ok(())
}
