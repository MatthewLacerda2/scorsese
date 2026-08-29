//! What each refusal says out loud.
//!
//! Here rather than beside its variant because [`SynthError`] does not fit in
//! one file under the size gate, and a message is the half that can leave:
//! nothing matches on one, so moving it costs no caller anything. The groups
//! and the order are [the enum's](super), one arm per variant.
//!
//! **The match is exhaustive**, which is what makes keeping the two apart
//! safe: a variant added next door and left without words here does not
//! compile, and the error names it.

use std::fmt;

use super::SynthError;

impl SynthError {
    /// The words this refusal is printed with — the sentence its reader acts
    /// on, which is usually an agent repairing a recipe unattended.
    pub(super) fn say(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // ────────────────────────────────────────────────────────────────────
            // An instrument and its chain: what a source, a filter, an envelope, an
            // LFO or an effect is refused for. An fx chain is a patch's own stage, so
            // the three refusals a `sidechain` earns sit here too.
            // ────────────────────────────────────────────────────────────────────
            Self::EmptyOscStack => write!(f, "patch: `osc_stack` needs at least one oscillator"),
            Self::TooManyOscs { found, limit, .. } => write!(
                f,
                "patch: `osc_stack` takes at most {limit} oscillators, got {found}"
            ),
            Self::SilentOscStack => write!(
                f,
                "patch: every oscillator gain is zero — the stack is silent"
            ),
            Self::BadVoiceCount { found, limit, .. } => write!(
                f,
                "patch: an oscillator sounds 1 to {limit} voices, got {found}"
            ),
            Self::BadFmRatio { ratio, .. } => {
                write!(f, "patch: `fm2` needs a positive `ratio`, got {ratio}")
            }
            Self::EmptyPartials => write!(f, "patch: `additive` needs at least one partial"),
            Self::TooManyPartials { found, limit, .. } => write!(
                f,
                "patch: `additive` takes at most {limit} partials, got {found}"
            ),
            Self::SilentPartials => write!(
                f,
                "patch: every partial gain is zero — the series is silent"
            ),
            Self::BadPartialRatio { index, ratio, .. } => write!(
                f,
                "patch: `additive` partial {index} needs a positive `ratio`, got {ratio}"
            ),
            Self::BadOperatorRatio {
                operator, ratio, ..
            } => write!(
                f,
                "patch: `fm4` operator {operator} needs a positive `ratio`, got {ratio}"
            ),
            Self::SilentCarriers { algorithm, .. } => write!(
                f,
                "patch: `fm4` algorithm `{algorithm}` is heard through its carriers, and every one of them is at level zero"
            ),
            Self::BadCutoff { cutoff, .. } => write!(
                f,
                "patch: filter `cutoff` must be positive Hz, got {cutoff}"
            ),
            Self::TooManyEqBands { found, limit, .. } => {
                write!(f, "fx: `eq` takes at most {limit} bands, got {found}")
            }
            Self::UnknownSidechain { track, key, .. } => write!(
                f,
                "song: track `{track}` is keyed from `{key}`, which is not a track in this song"
            ),
            Self::SelfSidechain { track, .. } => write!(
                f,
                "song: track `{track}` is keyed from itself — leave `sidechain` out for a compressor that listens to its own part"
            ),
            Self::MisplacedSidechain { place, key, .. } => write!(
                f,
                "{place}: `compress` is keyed from `{key}`, and only a track's own chain sits where one track can listen to another"
            ),
            Self::NegativeLfoRate { rate, .. } => {
                write!(f, "patch: lfo `rate` must not be negative, got {rate}")
            }

            // ────────────────────────────────────────────────────────────────────
            // A note as it is written: how long it lasts, and what it is called.
            // ────────────────────────────────────────────────────────────────────
            Self::BadDuration { duration, .. } => write!(
                f,
                "note: `duration` must be positive seconds, got {duration}"
            ),
            Self::EmptyNoteName => write!(f, "empty note name"),
            Self::BadNoteLetter { name, letter, .. } => {
                write!(f, "note `{name}`: expected a letter A–G, got `{letter}`")
            }
            Self::BadOctave { name, octave, .. } => {
                write!(f, "note `{name}`: `{octave}` is not an octave number")
            }
            Self::NoteOutOfRange { name, midi, .. } => {
                write!(f, "note `{name}`: MIDI {midi} is outside 0..=127")
            }

            // ────────────────────────────────────────────────────────────────────
            // A piece: its tempo, its tracks, its patterns and arrangement, how it is
            // played, the curves that move a value across it, and the length it has to
            // come out at.
            // ────────────────────────────────────────────────────────────────────
            Self::BadBpm { bpm, .. } => write!(f, "song: bpm must be positive, got {bpm}"),
            Self::NoTracks => write!(f, "song: no tracks"),
            Self::EmptyArrangement => {
                write!(f, "song: arrangement is empty — nothing would be rendered")
            }
            Self::UnknownPattern { pattern, .. } => write!(
                f,
                "song: arrangement names pattern `{pattern}`, which is not defined"
            ),
            Self::UnknownTrackFilter { pattern, track, .. } => write!(
                f,
                "song: arrangement entry for `{pattern}`: no track named `{track}` to play"
            ),
            Self::BadTranspose {
                pattern, transpose, ..
            } => write!(
                f,
                "song: arrangement entry for `{pattern}`: transpose must be finite, got {transpose}"
            ),
            Self::BadVelocityScale { pattern, scale, .. } => write!(
                f,
                "song: arrangement entry for `{pattern}`: vel_scale must be >= 0, got {scale}"
            ),
            Self::BadPatternBeats { pattern, beats, .. } => write!(
                f,
                "song: pattern `{pattern}`: beats must be positive, got {beats}"
            ),
            Self::UnknownSoloTrack { track } => {
                write!(f, "song: no track named `{track}` to render on its own")
            }
            Self::UnknownTrack {
                pattern,
                index,
                track,
                ..
            } => write!(
                f,
                "song: pattern `{pattern}` note {index}: no track named `{track}`"
            ),
            Self::BadNoteStart {
                pattern,
                index,
                start,
                ..
            } => write!(
                f,
                "song: pattern `{pattern}` note {index}: start must be >= 0, got {start}"
            ),
            Self::BadNoteDuration {
                pattern,
                index,
                dur,
                ..
            } => write!(
                f,
                "song: pattern `{pattern}` note {index}: dur must be positive, got {dur}"
            ),
            Self::UnknownChord { chord, .. } => write!(
                f,
                "song: `{chord}` is not a chord name — see the table in docs/recipes.md, or write the pitches out as `[\"D3\", \"F3\", \"A3\"]`"
            ),
            Self::ChordOutOfRange {
                chord, oct, midi, ..
            } => write!(
                f,
                "song: chord `{chord}` at octave {oct} reaches MIDI {midi}, outside 0..=127"
            ),
            Self::SpelledChordOctave { track, start, .. } => write!(
                f,
                "song: track `{track}` at beat {start}: `oct` means nothing beside spelled pitches"
            ),
            Self::EmptyChord { track, start, .. } => write!(
                f,
                "song: track `{track}` at beat {start}: a chord needs at least one pitch"
            ),
            Self::BadStep {
                track,
                character,
                step,
                ..
            } => write!(
                f,
                "song: track `{track}`: `{character}` at step {step} is not a step — use `x` (a hit), `X` (an accent) or `-` (a rest), and nothing else"
            ),
            Self::StepsDoNotFit {
                track,
                start,
                div,
                beats,
                written,
                needed,
                ..
            } => write!(
                f,
                "song: track `{track}`: {written} steps of {div} beats from beat {start} do not fill the pattern's {beats} — {needed} would"
            ),
            Self::BadStepDiv {
                track,
                start,
                div,
                beats,
                ..
            } => write!(
                f,
                "song: track `{track}`: no whole number of {div}-beat steps fills the {beats} beats from beat {start}"
            ),
            Self::AccentWithoutHeadroom { track, vel, .. } => write!(
                f,
                "song: track `{track}`: `X` and `x` are the same hit at `vel` {vel} — write a `vel` below 1 for the plain hits to be softer than the accents"
            ),
            Self::TwiceAccented { track, .. } => write!(
                f,
                "song: track `{track}`: a step string cannot be played `accent` and also mark accents with `X` — accent the whole run, or mark the hits"
            ),
            Self::BadKey { key, .. } => write!(
                f,
                "song: `{key}` is not a key — write a tonic and a mode, like `D minor`, `F# lydian` or `Bb major`"
            ),
            Self::BadDegree { degree, .. } => write!(
                f,
                "song: `{degree}` is not a scale degree — they count from 1, with accidentals in front (`b3`, `#4`)"
            ),
            Self::DegreeWithoutKey { track, start, .. } => write!(
                f,
                "song: track `{track}` at beat {start}: a `degree` needs the song to declare a `key`"
            ),
            Self::DegreeOutOfRange {
                degree, oct, midi, ..
            } => write!(
                f,
                "song: degree `{degree}` with the tonic at octave {oct} reaches MIDI {midi}, outside 0..=127"
            ),
            Self::DiatonicWithoutKey { pattern, .. } => write!(
                f,
                "song: arrangement entry for `{pattern}`: `transpose_degrees` needs the song to declare a `key` — or use `transpose` for a chromatic shift"
            ),
            Self::TwoTransposes { pattern, .. } => write!(
                f,
                "song: arrangement entry for `{pattern}`: `transpose` is chromatic and `transpose_degrees` moves within the key — write one or the other"
            ),
            Self::BadSwing { swing, .. } => write!(
                f,
                "song: `swing` must be at least 0 and below 1 (0 is straight, 0.33 swings), got {swing}"
            ),
            Self::BadHumanize { field, amount, .. } => write!(
                f,
                "song: `humanize.{field}` must be zero or more, got {amount}"
            ),
            Self::UnknownAutomationTrack { track, param, .. } => write!(
                f,
                "song: automation of `{param}` names `{track}`, which is not a track here"
            ),
            Self::DuplicateAutomation { track, param, .. } => write!(
                f,
                "song: track `{track}` has two curves for `{param}` — one parameter moves one way"
            ),
            Self::BadAutomationCurve {
                track, param, why, ..
            } => write!(f, "song: automation of `{param}` on track `{track}`: {why}"),
            Self::BadAutomationPoint {
                track,
                param,
                field,
                value,
                ..
            } => write!(
                f,
                "song: automation of `{param}` on `{track}`: bad `{field}`, got {value}"
            ),
            Self::BadAutomationCutoff { track, cutoff, .. } => write!(
                f,
                "song: automation of `cutoff` on `{track}`: must be positive Hz, got {cutoff}"
            ),
            Self::AutomationWithoutFilter { track, .. } => write!(
                f,
                "song: automation of `cutoff` on track `{track}`, whose patch has no filter"
            ),
            Self::BadFitSeconds { seconds, .. } => {
                write!(f, "song: `fit.seconds` must be positive, got {seconds}")
            }
            Self::BadFade { seconds, .. } => write!(
                f,
                "song: a fade must be zero or more seconds, got {seconds}"
            ),
            Self::StretchTooFar {
                bpm, needed, limit, ..
            } => write!(
                f,
                "song: fitting this at `stretch` needs {needed:.1} bpm against {bpm:.1} written, further than the {}% a piece survives — use `loop`, or change the arrangement",
                (limit * 100.0).round()
            ),

            // ────────────────────────────────────────────────────────────────────
            // The caller's resolver — the one refusal this crate does not make itself.
            // ────────────────────────────────────────────────────────────────────
            Self::UnresolvedPatch {
                track,
                reference,
                reason,
                ..
            } => write!(
                f,
                "song: track `{track}`: cannot resolve patch `{reference}`: {reason}"
            ),
        }
    }
}
