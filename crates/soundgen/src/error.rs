//! Why a recipe could not be rendered.
//!
//! Typed rather than a message, because the caller is usually an agent
//! repairing a recipe unattended: it has to be able to tell "this note name is
//! nonsense" from "this song names a pattern that does not exist" without
//! reading prose. Every variant carries the offending value, so the fix is in
//! the error rather than in a second look at the document.
//!
//! Only what would produce silence, a divide-by-zero, an unstable filter or an
//! unbounded allocation is rejected. Musical taste is the recipe's business:
//! an ugly patch renders.

/// A recipe the synthesiser cannot honour.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SynthError {
    /// An oscillator stack with nothing in it makes no sound at all.
    #[error("patch: `osc_stack` needs at least one oscillator")]
    EmptyOscStack,

    /// Past a handful of oscillators the stack costs CPU for no audible gain,
    /// so the cap is stated rather than discovered.
    #[error("patch: `osc_stack` takes at most {limit} oscillators, got {found}")]
    TooManyOscs {
        /// How many the stack asked for.
        found: usize,
        /// The most it may have.
        limit: usize,
    },

    /// Every oscillator weighted zero: a stack that renders silence, which is
    /// never what was meant.
    #[error("patch: every oscillator gain is zero — the stack is silent")]
    SilentOscStack,

    /// The FM modulator's frequency is a multiple of the played pitch, so a
    /// non-positive ratio has no sound to describe.
    #[error("patch: `fm2` needs a positive `ratio`, got {ratio}")]
    BadFmRatio {
        /// The ratio as written.
        ratio: f32,
    },

    /// A cutoff at or below zero Hz leaves the filter with nothing to pass.
    #[error("patch: filter `cutoff` must be positive Hz, got {cutoff}")]
    BadCutoff {
        /// The cutoff as written.
        cutoff: f32,
    },

    /// An LFO running backwards is not a shape the modulators can follow.
    #[error("patch: lfo `rate` must not be negative, got {rate}")]
    NegativeLfoRate {
        /// The rate as written.
        rate: f32,
    },

    /// A note with no length renders no samples.
    #[error("note: `duration` must be positive seconds, got {duration}")]
    BadDuration {
        /// The duration as written.
        duration: f32,
    },

    /// A note name with nothing in it.
    #[error("empty note name")]
    EmptyNoteName,

    /// Note names start with a letter A–G; anything else is a typo rather than
    /// an exotic tuning.
    #[error("note `{name}`: expected a letter A–G, got `{letter}`")]
    BadNoteLetter {
        /// The name as written.
        name: String,
        /// The character found where the letter should be.
        letter: char,
    },

    /// The octave is the number after the letter and any accidentals.
    #[error("note `{name}`: `{octave}` is not an octave number")]
    BadOctave {
        /// The name as written.
        name: String,
        /// The text found where the octave should be.
        octave: String,
    },

    /// MIDI numbers run 0–127, so a note outside that has no pitch to render.
    #[error("note `{name}`: MIDI {midi} is outside 0..=127")]
    NoteOutOfRange {
        /// The name as written.
        name: String,
        /// What it worked out to.
        midi: i32,
    },

    /// Tempo divides into every duration in the song.
    #[error("song: bpm must be positive, got {bpm}")]
    BadBpm {
        /// The tempo as written.
        bpm: f32,
    },

    /// A song with no tracks has no instruments to play anything on.
    #[error("song: no tracks")]
    NoTracks,

    /// An arrangement is the running order; an empty one renders nothing.
    #[error("song: arrangement is empty — nothing would be rendered")]
    EmptyArrangement,

    /// The arrangement names a pattern the song does not define — a typo that
    /// would otherwise be silence in the middle of a piece.
    #[error("song: arrangement names pattern `{pattern}`, which is not defined")]
    UnknownPattern {
        /// The name that matched no pattern.
        pattern: String,
    },

    /// A pattern's slot length decides where the next one starts.
    #[error("song: pattern `{pattern}`: beats must be positive, got {beats}")]
    BadPatternBeats {
        /// The pattern at fault.
        pattern: String,
        /// The length as written.
        beats: f32,
    },

    /// A note assigned to an instrument the song does not have.
    #[error("song: pattern `{pattern}` note {index}: no track named `{track}`")]
    UnknownTrack {
        /// The pattern holding the note.
        pattern: String,
        /// Which note in it, counting from zero.
        index: usize,
        /// The name that matched no track.
        track: String,
    },

    /// A note starting before the pattern does, or at no time at all.
    #[error("song: pattern `{pattern}` note {index}: start must be >= 0, got {start}")]
    BadNoteStart {
        /// The pattern holding the note.
        pattern: String,
        /// Which note in it, counting from zero.
        index: usize,
        /// The start as written, in beats.
        start: f32,
    },

    /// A note that is held for no time.
    #[error("song: pattern `{pattern}` note {index}: dur must be positive, got {dur}")]
    BadNoteDuration {
        /// The pattern holding the note.
        pattern: String,
        /// Which note in it, counting from zero.
        index: usize,
        /// The duration as written, in beats.
        dur: f32,
    },

    /// A target length that is not a length.
    #[error("song: `fit.seconds` must be positive, got {seconds}")]
    BadFitSeconds {
        /// The target as written.
        seconds: f32,
    },

    /// A fade that runs for a negative or nonsensical time.
    #[error("song: a fade must be zero or more seconds, got {seconds}")]
    BadFade {
        /// The offending length.
        seconds: f32,
    },

    /// `stretch` would have had to move the tempo further than a piece of
    /// music survives. Refused rather than delivered, because a bed at 40%
    /// speed is not a bed.
    ///
    /// Carries the tempo it would have needed, so the caller can decide
    /// between a different target, a different arrangement, and `loop`.
    #[error(
        "song: fitting this at `stretch` needs {needed:.1} bpm against {bpm:.1} written, \
         further than the {}% a piece survives — use `loop`, or change the arrangement",
        (limit * 100.0).round()
    )]
    StretchTooFar {
        /// The tempo the song is written at.
        bpm: f32,
        /// The tempo the target would have required.
        needed: f32,
        /// How far the tempo may move, as a fraction either way.
        limit: f32,
    },

    /// A track named its instrument by reference and the caller's resolver
    /// could not produce it. What "could not" means is the caller's to say —
    /// this crate never opens anything itself.
    #[error("song: track `{track}`: cannot resolve patch `{reference}`: {reason}")]
    UnresolvedPatch {
        /// The track whose instrument is missing.
        track: String,
        /// The reference as written in the song.
        reference: String,
        /// What the resolver said about it.
        reason: String,
    },
}
