//! Why a recipe could not be rendered.
//!
//! Typed rather than a message, because the caller is usually an agent
//! repairing a recipe unattended: it has to be able to tell "this note name is
//! nonsense" from "this song names a pattern that does not exist" without
//! reading prose. Every variant carries the offending value, so the fix is in
//! the error rather than in a second look at the document.
//!
//! ## Where a new variant goes
//!
//! The variants sit in four labelled groups below — **an instrument and its
//! chain**, **a note as it is written**, **a piece**, and **the caller's
//! resolver**. A new refusal joins the group its subject belongs to; the words
//! it is printed with go in the same group of [`wording`], and that match is
//! exhaustive, so the compiler will not let a variant be added without them.
//!
//! The words are in a second file because this one does not fit under the size
//! gate otherwise, and they are the half that can leave: nothing matches on a
//! message. The variants themselves cannot be split, because [`SynthError`] is
//! one flat public enum — callers and tests match its variants by name, so
//! nesting a group behind a second enum would be an API change rather than a
//! file-layout one, and Rust has no way to write one enum across two files.
//!
//! Only what would produce silence, a divide-by-zero, an unstable filter or an
//! unbounded allocation is rejected. Musical taste is the recipe's business:
//! an ugly patch renders.

mod wording;

/// A recipe the synthesiser cannot honour.
///
/// Flat and public deliberately: a caller matches a variant by name. What each
/// one *says* is in the `wording` module beside this one.
#[derive(Debug, Clone, PartialEq)]
pub enum SynthError {
    // ────────────────────────────────────────────────────────────────────
    // An instrument and its chain: what a source, a filter, an envelope, an
    // LFO or an effect is refused for. An fx chain is a patch's own stage, so
    // the three refusals a `sidechain` earns sit here too.
    // ────────────────────────────────────────────────────────────────────
    /// An oscillator stack with nothing in it makes no sound at all.
    EmptyOscStack,

    /// Past a handful of oscillators the stack costs CPU for no audible gain,
    /// so the cap is stated rather than discovered.
    TooManyOscs {
        /// How many the stack asked for.
        found: usize,
        /// The most it may have.
        limit: usize,
    },

    /// Every oscillator weighted zero: a stack that renders silence, which is
    /// never what was meant.
    SilentOscStack,

    /// A unison count outside what one oscillator may sound at once. Both ends
    /// refuse: zero voices is an oscillator asked to make no sound without
    /// saying so, and past the limit is
    /// [`SynthError::TooManyOscs`]'s argument one level down — the copies stop
    /// separating and the arithmetic keeps growing.
    BadVoiceCount {
        /// How many the oscillator asked for.
        found: usize,
        /// The most it may have.
        limit: usize,
    },

    /// The FM modulator's frequency is a multiple of the played pitch, so a
    /// non-positive ratio has no sound to describe.
    BadFmRatio {
        /// The ratio as written.
        ratio: f32,
    },

    /// An additive series with nothing in it states no spectrum, so there is
    /// no tone for it to make.
    EmptyPartials,

    /// Past a handful of partials the series costs one sine oscillator per
    /// note per sample for no audible gain — the same argument
    /// [`SynthError::TooManyOscs`] makes about a stack, at a higher count
    /// because a spectrum genuinely needs more entries than a stack does.
    TooManyPartials {
        /// How many the series asked for.
        found: usize,
        /// The most it may have.
        limit: usize,
    },

    /// Every partial weighted zero: a series that renders silence, which is
    /// never what was meant.
    SilentPartials,

    /// A partial's frequency is a multiple of the played pitch, so a
    /// non-positive multiple names no frequency. Zero in particular is a DC
    /// offset — inaudible, and it eats the headroom the rest of the series
    /// needs.
    BadPartialRatio {
        /// Which partial of the series it is, counting from zero.
        index: usize,
        /// The ratio as written.
        ratio: f32,
    },

    /// One operator of an `fm4` source with no pitch to describe.
    /// [`SynthError::BadPartialRatio`] for a routing rather than a series, and
    /// its own variant for the same reason that one is: the fix has to name
    /// *which* entry is wrong.
    ///
    /// Counted from **one**, unlike a partial, because an operator's number is
    /// part of the algorithm's own vocabulary rather than a place in a list —
    /// operator 3 is a row of the diagram, and the field name says as much.
    BadOperatorRatio {
        /// Which operator, numbered from one as the recipe and the algorithm
        /// diagrams number them.
        operator: usize,
        /// The ratio as written.
        ratio: f32,
    },

    /// Every operator an `fm4` algorithm is heard through is at level zero, so
    /// the source renders silence. [`SynthError::SilentPartials`] one
    /// indirection further away: which operators are audible is a property of
    /// the algorithm rather than of the list, so the message names it.
    SilentCarriers {
        /// The algorithm, as the document spells it.
        algorithm: &'static str,
    },

    /// A cutoff at or below zero Hz leaves the filter with nothing to pass.
    BadCutoff {
        /// The cutoff as written.
        cutoff: f32,
    },

    /// Past a handful of bands an EQ is a filter bank being assembled one band
    /// at a time, which is a different tool — the same argument
    /// [`SynthError::TooManyOscs`] makes about a stack, applied to arithmetic
    /// that runs over every sample.
    ///
    /// Named `fx` rather than `patch`, because a chain lives in three places
    /// and the message has to be true in all of them.
    TooManyEqBands {
        /// How many the band list asked for.
        found: usize,
        /// The most it may have.
        limit: usize,
    },

    /// A compressor keyed from a track this song does not have — the typo that
    /// would otherwise be a duck the recipe wrote and never heard.
    UnknownSidechain {
        /// The track carrying the compressor.
        track: String,
        /// The name it asked to listen to.
        key: String,
    },

    /// A track keyed from itself is an ordinary compressor written the long
    /// way round, and far more likely a name that was meant to be another's.
    SelfSidechain {
        /// The track that named itself.
        track: String,
    },

    /// A `sidechain` outside a track's own chain. A patch's chain runs per note
    /// and the song's runs on the sum; in neither is there a track to listen
    /// to, so it is refused rather than quietly dropped.
    MisplacedSidechain {
        /// Which chain it was written on — `patch` or `song`.
        place: &'static str,
        /// The name it asked to listen to.
        key: String,
    },

    /// An LFO running backwards is not a shape the modulators can follow.
    NegativeLfoRate {
        /// The rate as written.
        rate: f32,
    },

    // ────────────────────────────────────────────────────────────────────
    // A note as it is written: how long it lasts, and what it is called.
    // ────────────────────────────────────────────────────────────────────
    /// A note with no length renders no samples.
    BadDuration {
        /// The duration as written.
        duration: f32,
    },

    /// A note name with nothing in it.
    EmptyNoteName,

    /// Note names start with a letter A–G; anything else is a typo rather than
    /// an exotic tuning.
    BadNoteLetter {
        /// The name as written.
        name: String,
        /// The character found where the letter should be.
        letter: char,
    },

    /// The octave is the number after the letter and any accidentals.
    BadOctave {
        /// The name as written.
        name: String,
        /// The text found where the octave should be.
        octave: String,
    },

    /// MIDI numbers run 0–127, so a note outside that has no pitch to render.
    NoteOutOfRange {
        /// The name as written.
        name: String,
        /// What it worked out to.
        midi: i32,
    },

    // ────────────────────────────────────────────────────────────────────
    // A piece: its tempo, its tracks, its patterns and arrangement, how it is
    // played, the curves that move a value across it, and the length it has to
    // come out at.
    // ────────────────────────────────────────────────────────────────────
    /// Tempo divides into every duration in the song.
    BadBpm {
        /// The tempo as written.
        bpm: f32,
    },

    /// A song with no tracks has no instruments to play anything on.
    NoTracks,

    /// An arrangement is the running order; an empty one renders nothing.
    EmptyArrangement,

    /// The arrangement names a pattern the song does not define — a typo that
    /// would otherwise be silence in the middle of a piece.
    UnknownPattern {
        /// The name that matched no pattern.
        pattern: String,
    },

    /// An arrangement entry's `tracks` filter names an instrument the song does
    /// not have — the same typo as an unknown pattern, with the same
    /// consequence: something that silently never plays.
    UnknownTrackFilter {
        /// The pattern the entry plays.
        pattern: String,
        /// The name that matched no track.
        track: String,
    },

    /// An arrangement entry asks to transpose by something that is not a
    /// number of semitones.
    BadTranspose {
        /// The pattern the entry plays.
        pattern: String,
        /// The transposition as written.
        transpose: f32,
    },

    /// An arrangement entry scales velocity by something that is not a
    /// non-negative number. Negative would be a phase inversion by another
    /// name, which is not what "quieter" means.
    BadVelocityScale {
        /// The pattern the entry plays.
        pattern: String,
        /// The scale as written.
        scale: f32,
    },

    /// A pattern's slot length decides where the next one starts.
    BadPatternBeats {
        /// The pattern at fault.
        pattern: String,
        /// The length as written.
        beats: f32,
    },

    /// A note assigned to an instrument the song does not have.
    UnknownTrack {
        /// The pattern holding the note.
        pattern: String,
        /// Which note in it, counting from zero.
        index: usize,
        /// The name that matched no track.
        track: String,
    },

    /// A note starting before the pattern does, or at no time at all.
    BadNoteStart {
        /// The pattern holding the note.
        pattern: String,
        /// Which note in it, counting from zero.
        index: usize,
        /// The start as written, in beats.
        start: f32,
    },

    /// A note that is held for no time.
    BadNoteDuration {
        /// The pattern holding the note.
        pattern: String,
        /// Which note in it, counting from zero.
        index: usize,
        /// The duration as written, in beats.
        dur: f32,
    },

    /// A chord name the grammar does not carry. Refused rather than guessed:
    /// a chord that quietly means something other than what was written is
    /// worse than the notes it replaced, because the notes were visible.
    UnknownChord {
        /// The name as written.
        chord: String,
    },

    /// A chord voiced off the end of the keyboard. Refused rather than clamped,
    /// unlike an arrangement's transpose: the octave is right there in the same
    /// entry, so this is a document that can be fixed rather than a pattern
    /// meeting a transform written elsewhere.
    ChordOutOfRange {
        /// The name as written.
        chord: String,
        /// The octave its root was placed in.
        oct: i32,
        /// The voice that fell outside the range.
        midi: i32,
    },

    /// `oct` places the root of a *named* chord. A chord written as pitches
    /// already carries an octave per voice, so the two together say two
    /// different things about the same chord.
    SpelledChordOctave {
        /// The track the chord is on.
        track: String,
        /// Where in the pattern it sits, in beats.
        start: f32,
        /// The octave as written.
        oct: i32,
    },

    /// A chord spelled as an empty list of pitches: an entry that sounds
    /// nothing, which is never what a chord was written for.
    EmptyChord {
        /// The track the chord is on.
        track: String,
        /// Where in the pattern it sits, in beats.
        start: f32,
    },

    /// A character in a step string that is not a step. Refused rather than
    /// skipped: every character is one step and the count is what proves the
    /// string covers its pattern, so a character that is quietly not a step
    /// takes the grid with it.
    BadStep {
        /// The track the step string is on.
        track: String,
        /// The character as written.
        character: char,
        /// Which step of the string it is, counting from zero.
        step: usize,
    },

    /// A step string whose length is not the length its grid needs. This is
    /// the error the notation exists to make loud: fifteen sixteenths read as
    /// a bar on the page, and silent truncation would leave the ear to find it.
    StepsDoNotFit {
        /// The track the step string is on.
        track: String,
        /// Where in the pattern the string starts, in beats.
        start: f32,
        /// The step length as written.
        div: f32,
        /// The slot the pattern occupies, in beats.
        beats: f32,
        /// How many steps the string has.
        written: usize,
        /// How many it would need to reach the end of the slot.
        needed: usize,
    },

    /// A step length that no whole number of steps fits into the pattern with
    /// — including one that is zero, negative or not a number. Its own error
    /// rather than a length mismatch because the fix is different: `div` is
    /// what has to change, not the string.
    BadStepDiv {
        /// The track the step string is on.
        track: String,
        /// Where in the pattern the string starts, in beats.
        start: f32,
        /// The step length as written.
        div: f32,
        /// The slot the pattern occupies, in beats.
        beats: f32,
    },

    /// A step string drawing a distinction its velocity leaves no room for.
    /// An `X` plays at full velocity, so beside a `vel` of 1 it is the same
    /// hit as an `x` — and the page would show accents the audio does not
    /// have, which is the one thing worse than no accents at all.
    AccentWithoutHeadroom {
        /// The track the step string is on.
        track: String,
        /// The velocity as written.
        vel: f32,
    },

    /// A `key` the grammar does not read. Refused rather than ignored: a song
    /// that declares a key nobody can parse is one whose every degree would
    /// resolve somewhere else, and silently.
    BadKey {
        /// The key as written.
        key: String,
    },

    /// A degree that is not a degree: zero or below, or text that is not
    /// accidentals in front of a number. Degrees count from **one**, so a zero
    /// is exactly what a writer who assumed otherwise would have written, and
    /// reading it as the tonic would put a whole part a step flat.
    BadDegree {
        /// The degree as written.
        degree: String,
    },

    /// A note written as a degree in a song that declares no `key`. There is
    /// no scale for it to be a degree *of*, and inferring one from the other
    /// notes is analysis this crate does not do.
    DegreeWithoutKey {
        /// The track the note is on.
        track: String,
        /// Where in the pattern it sits, in beats.
        start: f32,
    },

    /// A degree that lands off the end of the keyboard. Refused rather than
    /// clamped, for the reason [`SynthError::ChordOutOfRange`] is: the octave
    /// is in the same entry and can simply be corrected.
    DegreeOutOfRange {
        /// The degree as written.
        degree: String,
        /// The octave its tonic was placed in.
        oct: i32,
        /// What it worked out to.
        midi: i32,
    },

    /// An arrangement entry asks for a diatonic lift in a song with no `key`.
    /// There is no scale to step along, and guessing one would put a whole
    /// section somewhere nobody chose.
    DiatonicWithoutKey {
        /// The pattern the entry plays.
        pattern: String,
    },

    /// One entry asking for both lifts. Which applies first changes the
    /// answer and no convention decides it, so the document has to say which
    /// one it means — see
    /// [`transpose_degrees`](crate::song::Play::transpose_degrees).
    TwoTransposes {
        /// The pattern the entry plays.
        pattern: String,
    },

    /// A swing outside the range where it still means "the off-beat sits
    /// late". At 1 the off-beat eighth lands on the following beat — the two
    /// have swapped places rather than been felt — and below 0 the off-beats
    /// run early, which is not swing under any name.
    BadSwing {
        /// The swing as written.
        swing: f32,
    },

    /// A humanise amount that is not an amount. Both fields are magnitudes —
    /// how far a player may stray, either way — so a negative one is not the
    /// other direction, it is nonsense.
    BadHumanize {
        /// Which of the two fields is at fault, as it is spelled in the
        /// document.
        field: &'static str,
        /// The amount as written.
        amount: f32,
    },

    /// A curve moving a parameter of a track this song does not have. The same
    /// typo as an unknown track anywhere else, with the same consequence: a
    /// build the recipe says is there and nothing can hear.
    UnknownAutomationTrack {
        /// The name that matched no track.
        track: String,
        /// The parameter the curve moves, as the document spells it.
        param: &'static str,
    },

    /// Two curves on one track and parameter. Which one applies is not a
    /// question with an answer, so it is refused rather than resolved by
    /// declaration order.
    DuplicateAutomation {
        /// The track carrying both.
        track: String,
        /// The parameter both claim.
        param: &'static str,
    },

    /// A list of points that is not a curve: none at all, which moves nothing,
    /// or points that do not ascend in time. A curve is read as a path from
    /// each point to the next, so an out-of-order list is a path nobody wrote
    /// — refused rather than sorted, because silently reordering an author's
    /// document is worse than declining it.
    BadAutomationCurve {
        /// The track the curve rides.
        track: String,
        /// The parameter it claims to move.
        param: &'static str,
        /// What is wrong with the list, in the words the message carries.
        why: &'static str,
    },

    /// A control point at a time that is not a time, or holding a value that
    /// is not a number. A NaN would spread through the mix as a whole song of
    /// silence, a long way from the field that caused it.
    BadAutomationPoint {
        /// The track the curve rides.
        track: String,
        /// The parameter it moves.
        param: &'static str,
        /// Which of a point's two numbers is at fault — `beat`, which must be
        /// finite and at or after zero, or `value`, which must be finite.
        field: &'static str,
        /// The number as written.
        value: f32,
    },

    /// A cutoff curve passing through zero Hz or below — refused for the
    /// reason [`SynthError::BadCutoff`] refuses a written one, at every point
    /// rather than once.
    BadAutomationCutoff {
        /// The track the curve rides.
        track: String,
        /// The offending point's value.
        cutoff: f32,
    },

    /// A `cutoff` curve on an instrument with no filter: there is nothing for
    /// it to move. Caught only once the track's patch is resolved, since a
    /// track may name its instrument rather than carry it.
    AutomationWithoutFilter {
        /// The track whose instrument has no filter stage.
        track: String,
    },

    /// A target length that is not a length.
    BadFitSeconds {
        /// The target as written.
        seconds: f32,
    },

    /// A fade that runs for a negative or nonsensical time.
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
    StretchTooFar {
        /// The tempo the song is written at.
        bpm: f32,
        /// The tempo the target would have required.
        needed: f32,
        /// How far the tempo may move, as a fraction either way.
        limit: f32,
    },

    // ────────────────────────────────────────────────────────────────────
    // The caller's resolver — the one refusal this crate does not make itself.
    // ────────────────────────────────────────────────────────────────────
    /// A track named its instrument by reference and the caller's resolver
    /// could not produce it. What "could not" means is the caller's to say —
    /// this crate never opens anything itself.
    UnresolvedPatch {
        /// The track whose instrument is missing.
        track: String,
        /// The reference as written in the song.
        reference: String,
        /// What the resolver said about it.
        reason: String,
    },
}

impl std::error::Error for SynthError {}

impl std::fmt::Display for SynthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.say(f)
    }
}
