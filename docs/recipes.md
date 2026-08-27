# Recipes — sound from a document

A **recipe** is a JSON file under `recipes/` that a `synth_audio` asset is made
from. Rendering one costs nothing, needs no key and no network, and produces
the same bytes every time a given build of scorsese renders it — so a project
can carry its whole sound design as text and rebuild it anywhere. What that
last clause is doing is [below](#the-synthesiser-is-the-other-half-of-a-bake).

Two shapes, told apart by the `recipe` field: **`patch`** is one instrument
playing one note (an effect), **`song`** is a piece of music.

**Both are sound nobody speaks.** Recipes make effects and score — a gunshot, a
footstep, a UI blip, the music under all of them — and there is no voice in
them. That is a boundary rather than a gap: nothing in a patch or a song has
anywhere to put a phoneme, and a synthesised approximation of a person talking
is not what anyone wants when real narration is one asset away. Spoken audio is
a `generated_audio` asset — a prompt handed to a provider, costing money and a
network, reproducible from nothing but its cache. Keeping the two kinds apart
is what lets everything on this page promise free and deterministic.

```
scorsese synth new zap                 # a patch recipe, and its asset
scorsese synth new theme --kind song   # a song recipe, and its asset
scorsese synth bake                    # render everything not already baked
scorsese synth check recipes/theme.json
```

Both starters make a sound as written. Bake first, listen, then edit.

## Effects: `"recipe": "patch"`

```json recipe
{
  "recipe": "patch",
  "note": "C4",
  "duration": 0.4,
  "velocity": 1.0,
  "seed": 0,
  "patch": {
    "source": { "kind": "noise" },
    "amp": { "a": 0.001, "d": 0.12, "s": 0.0, "r": 0.05 },
    "filter": {
      "kind": "lowpass", "cutoff": 400, "resonance": 0.3,
      "env_amount": 5000,
      "adsr": { "a": 0.0, "d": 0.08, "s": 0.0, "r": 0.05 }
    },
    "fx": [{ "fx": "reverb", "size": 0.3, "damp": 0.6, "mix": 0.15 }]
  }
}
```

That one is a gunshot: noise through a lowpass whose cutoff slams open on the
attack and shuts in 80 ms, with a little room around it.

`note` is a name (`"C#4"`, `"Bb3"`) or a MIDI number, and fractional numbers
are legal microtonal pitches. It lives in the document because *which pitch* is
part of what the asset is — a footstep and a gunshot can be the same instrument
at different pitches. `duration` is how long the key is held; the release rings
out after it, so the file is longer. `velocity` is `0..=1`. `seed` re-rolls the
stochastic sources — same seed, same bytes, forever.

Only `note` and `patch` are required; the rest default.

### The signal path

Fixed, always, in this order. A recipe chooses what fills each stage, never how
they connect:

```
source ─► filter ─► amp envelope ─► fx
   ▲         ▲            ▲
   ├──── envelopes ───────┘        amp always; pitch and cutoff optional
   └─────── lfo ──────────┘        one target: pitch | cutoff | amp
```

**`source`** — one of four, tagged by `kind`:

| `kind` | Fields | Good for |
| --- | --- | --- |
| `osc_stack` | `oscs`: up to 4 of `{ wave, detune_cents, gain, octave }` | leads, basses, pads |
| `karplus` | `damping` 0..1, `brightness` 0..1 | plucked strings, marimbas |
| `noise` | — | gunshots, impacts, footsteps, wind |
| `fm2` | `ratio`, `index`, `vel_index`, `mod_decay` | bells, electric pianos, metal |

`wave` is `sine`, `triangle`, `saw` or `square`. Integer `ratio` on `fm2` stays
tonal; a fractional one goes metallic.

**`amp`** — an ADSR: `{ "a": 0.005, "d": 0.15, "s": 0.4, "r": 0.2 }`. Attack,
decay and release are seconds; sustain is a level in `0..=1`. **Required.**

Every ADSR — the amp one, the filter's, the pitch one — also takes an optional
`curve`, and it is worth reaching for. `0.0` is a straight line and the
default; positive bends the three timed segments into the exponential a
physical thing makes, falling fast and then trailing. Nothing real decays in a
straight line, and the ear knows: a linear decay holds up too long and then
arrives at silence too abruptly, which is one of the few cues that reliably
says *synthesised* about an otherwise well-made sound. Try `3.0` to `5.0` on
anything plucked, struck or hit; `8.0` is the steepest accepted. Negative
mirrors it into a slow start and a sudden arrival — a swell, not a decay.

**`filter`** *(optional)* — `kind` is `lowpass` or `highpass`, plus `cutoff` in
Hz, `resonance` 0..1, `env_amount` in Hz, `vel_cutoff` in Hz, and its own
`adsr`. `env_amount` is what makes a patch expressive rather than static: it
opens the cutoff on the attack and closes it as the note decays. A pluck is a
lowpass with a big positive `env_amount` and a fast filter decay.

**`pitch_env`** *(optional)* — `{ "semitones": 10, "adsr": { … } }`. A sweep of
the note's own pitch that happens once and settles — see
[Sweeping the pitch](#sweeping-the-pitch-not-just-wobbling-it) below.

**`lfo`** *(optional)* — `{ "rate": 5.0, "depth": 0.5, "target": "pitch" }`.
`pitch` is vibrato in semitones, `cutoff` is wobble in octaves, `amp` is tremolo
where 1.0 dips to silence.

**`fx`** *(optional)* — a list, applied in order. Each entry is tagged by `fx`:

| `fx` | Fields | What it does |
| --- | --- | --- |
| `delay` | `time` in seconds, `feedback` 0..1, `mix` 0..=1 | feedback echo — a slapback, a corridor |
| `reverb` | `size` 0..=1, `damp` 0..=1, `mix` 0..=1 | a room the sound is in |
| `saturate` | `drive`, `mix` 0..=1 | soft clip: warmth, weight, glue |
| `eq` | `bands`: up to 8 of `{ kind, freq, gain_db, q }` | takes a region away, or adds one |

A limiter always runs after them and is not listed — a bake must not clip, and
that is not the recipe's decision.

`saturate` is the only stage anywhere in synthesis that adds frequencies the
source did not have, which is what "warm" and "analog" are made of; everything
else is a straight line. Its `drive` is gain-compensated, so it changes the
shape of the wave rather than its level, and `0` is clean, `1`–`2` is warmth
and `4` is audible drive. Past `4` the invented harmonics start folding back as
inharmonic ringing on a bright source — fine on a bass or a drum, not on a lead
— and `8` is the ceiling.

`eq` is the other odd one, and it is the only effect here that is a **mixing
move** rather than a sound. A band's `kind` is one of:

| `kind` | reads `gain_db`? | what it does |
| --- | --- | --- |
| `high_pass` | no | everything below `freq` goes |
| `low_shelf` | yes | everything below `freq` moves by `gain_db`, together |
| `peak` | yes | a bump or a dip centred on `freq`, `q` wide |
| `high_shelf` | yes | everything above `freq` moves by `gain_db`, together |
| `low_pass` | no | everything above `freq` goes |

`q` is how narrow the band is — `0.7` is the gentle default, `2` is a
noticeable notch, `8` is surgical — and it means the same thing for all five.
Two things are worth knowing before writing one:

- **`freq` may be left out, and its default is the number the bake report
  splits at.** The three kinds that work on the bottom default to **250 Hz**
  and the two that work on the top default to **4 kHz**, so a report reading
  `low 61%` is answered by `{ "kind": "low_shelf", "gain_db": -3 }` with no
  arithmetic in between. Spell `freq` out whenever the ear says somewhere else.
- **`gain_db: 0.0` is a bypass**, exactly — a band at zero gain is not applied
  at all, so it is sample-identical to leaving it out. List the bands you are
  thinking about and sweep one; the parked ones colour nothing. (The two pass
  filters have no gain to ask for, so this does not switch them off.)

This chain is the *instrument's own*: it is applied to each note separately. A
song has two more places to put one, and reverb almost always wants one of them
— see [Where an effect goes](#where-an-effect-goes).

### Playing harder, not just louder

By default a note's `velocity` is a fader: it scales the level and nothing
else. That is not how an instrument behaves. Hit a piano key harder and it does
not merely get louder, it gets **brighter** — more of the energy goes into the
upper harmonics — and the ear reads that change as effort. Its absence is a
large part of why a carefully written synthesised part still sounds like a
machine.

Two optional fields aim velocity at the stages that decide brightness:

| field | on | does |
| --- | --- | --- |
| `vel_cutoff` | `filter` | adds this many Hz to the cutoff at full velocity |
| `vel_index` | `fm2` | adds this much modulation depth at full velocity |

Both default to `0.0`, and a recipe that does not mention them bakes exactly
the file it always did.

The example is a song rather than a one-shot, because the point only shows up
when one instrument is played at two strengths — songs are the section below.

```json recipe
{
  "recipe": "song",
  "bpm": 100,
  "tracks": [
    {
      "name": "keys",
      "patch": {
        "source": {
          "kind": "fm2", "ratio": 3.0, "index": 1.0,
          "vel_index": 7.0, "mod_decay": 0.25
        },
        "amp": { "a": 0.002, "d": 0.6, "s": 0.0, "r": 0.2 },
        "filter": {
          "kind": "lowpass", "cutoff": 900, "vel_cutoff": 3500,
          "adsr": { "a": 0.001, "d": 0.3, "s": 0.0, "r": 0.1 }
        }
      }
    }
  ],
  "patterns": {
    "a": { "beats": 4, "notes": [
      { "track": "keys", "note": "E3", "start": 0, "dur": 1.5, "vel": 1.0 },
      { "track": "keys", "note": "E3", "start": 2, "dur": 1.5, "vel": 0.35 }
    ] }
  },
  "arrangement": ["a"]
}
```

An electric piano playing the same note twice. Without those two fields the
second note is a photocopy of the first at a third of the level; with them it
is a different, softer sound — a key touched rather than struck.

Three things worth knowing:

- **They add, they do not scale.** The cutoff is
  `cutoff + env_amount × envelope + vel_cutoff × velocity`, so each source of
  movement stays independent and a zero stays harmless. Start `vel_cutoff`
  somewhere near `env_amount` and adjust by ear; they are the same quantity
  from different places.
- **Negative is legal**, and means velocity *darkens* — a real instrument, if
  an unusual one. A negative `vel_index` bottoms out at a bare carrier rather
  than turning around and brightening again.
- **A song's `vel_scale` gets this for free.** A quiet reprise written as
  `"vel_scale": 0.6` (below) sounds softer rather than merely quieter,
  which is most of the difference between a section that reads as a dynamic and
  one that reads as a volume knob.

### Sweeping the pitch, not just wobbling it

An `lfo` aimed at `pitch` is a vibrato: it wobbles around the played note for
as long as the note lasts. A great many sounds do the opposite — they start
*off* their pitch and arrive on it exactly once. A kick drum starts near 90 Hz
and is at 50 in about 40 ms; a tom, an 808, a timpani and a laser zap are the
same move at other speeds and depths. That gesture is `pitch_env`.

```json recipe
{
  "recipe": "patch",
  "note": "A1",
  "duration": 0.12,
  "patch": {
    "source": { "kind": "osc_stack", "oscs": [{ "wave": "sine" }] },
    "amp": { "a": 0.001, "d": 0.22, "s": 0.0, "r": 0.02, "curve": 4.0 },
    "pitch_env": {
      "semitones": 10,
      "adsr": { "a": 0.0, "d": 0.04, "s": 0.0, "r": 0.0, "curve": 5.0 }
    }
  }
}
```

That is a kick drum, and it is a sine wave. Everything that makes it a kick is
in the two envelopes: the pitch falls ten semitones onto A1 in 40 ms, and the
level sheds most of itself immediately and trails. Take either `curve` out and
it turns back into a synthesised approximation of a drum.

Three things worth knowing:

- **The note is where the sweep ends, not where it starts.** With the usual
  shape — full straight away, decaying to nothing — the sound begins
  `semitones` away and lands on the pitch it was played at. So positive falls
  onto the note from above (the drum case) and negative rises onto it from
  below (a reverse zap). Naming the destination is what lets one drum patch be
  played at several pitches and stay the same instrument.
- **Semitones, not Hz.** Pitch is logarithmic: the same number of Hz is an
  octave low down and a rounding error high up, so a sweep written in Hz would
  stop meaning the same gesture the moment the patch was played elsewhere.
- **It adds to a vibrato rather than replacing one.** The two offsets are
  summed in semitones and applied once, the same way `env_amount` and
  `vel_cutoff` add at the cutoff — so a note can fall onto its pitch and then
  wobble around it. `karplus` is the exception and always was: its delay line
  is sized once when the string is plucked, so neither an LFO nor a pitch
  envelope reaches it.

## Music: `"recipe": "song"`

```json recipe
{
  "recipe": "song",
  "bpm": 96,
  "seed": 1,
  "tracks": [
    {
      "name": "lead",
      "gain": 0.9,
      "patch": {
        "source": { "kind": "karplus", "damping": 0.995, "brightness": 0.4 },
        "amp": { "a": 0.001, "d": 0.3, "s": 0.0, "r": 0.15 }
      }
    },
    { "name": "bass", "patch": "recipes/bass.json", "gain": 0.8 }
  ],
  "patterns": {
    "a": { "beats": 4, "notes": [
      { "track": "lead", "note": "C3", "start": 0, "dur": 0.9, "vel": 1.0 },
      { "track": "lead", "note": "E3", "start": 1, "dur": 0.9, "vel": 0.9 },
      { "track": "bass", "note": "C2", "start": 0, "dur": 2.0, "vel": 1.0 }
    ] }
  },
  "arrangement": [
    "a",
    { "pattern": "a", "transpose": 12, "vel_scale": 0.6, "tracks": ["lead"] },
    "a"
  ]
}
```

**Everything is beats, never seconds.** `bpm` converts once at render time, so
changing the tempo of a finished piece is one number.

- **`tracks`** — the instruments. `patch` is either the document inline or a
  project-relative path to a bare patch file, so several songs can share one
  instrument. That path obeys the same rules every path in a project does: no
  absolute paths, no `..`, forward slashes.
- **`patterns`** — named blocks. `beats` is the *slot* the block occupies; notes
  may ring out past it and the next pattern still starts on time.
- **`arrangement`** — which patterns play, in order.

Write repetition as repetition. A melody with any structure is a few short
patterns and a list naming them; the same music as one flat note list is
hundreds of lines, and every edit becomes a merge conflict with itself.

### Writing a chord as a chord

An entry in `notes` is **one note or one chord**, told apart by which field it
carries — `note` or `chord`. Here are two bars of `Dm7 – Gm7 – A7` written both
ways. First, one pitch at a time:

```json fields
"tracks": [{ "name": "keys", "patch": {
  "source": { "kind": "karplus", "damping": 0.99, "brightness": 0.4 },
  "amp": { "a": 0.002, "d": 0.6, "s": 0.0, "r": 0.3 } } }],
"patterns": { "a": { "beats": 8, "notes": [
  { "track": "keys", "note": "D3",  "start": 0, "dur": 2, "vel": 0.8 },
  { "track": "keys", "note": "F3",  "start": 0, "dur": 2, "vel": 0.8 },
  { "track": "keys", "note": "A3",  "start": 0, "dur": 2, "vel": 0.8 },
  { "track": "keys", "note": "C4",  "start": 0, "dur": 2, "vel": 0.8 },
  { "track": "keys", "note": "G3",  "start": 2, "dur": 2, "vel": 0.8 },
  { "track": "keys", "note": "Bb3", "start": 2, "dur": 2, "vel": 0.8 },
  { "track": "keys", "note": "D4",  "start": 2, "dur": 2, "vel": 0.8 },
  { "track": "keys", "note": "F4",  "start": 2, "dur": 2, "vel": 0.8 },
  { "track": "keys", "note": "A3",  "start": 4, "dur": 4, "vel": 0.7 },
  { "track": "keys", "note": "C#4", "start": 4, "dur": 4, "vel": 0.7 },
  { "track": "keys", "note": "E4",  "start": 4, "dur": 4, "vel": 0.7 },
  { "track": "keys", "note": "G4",  "start": 4, "dur": 4, "vel": 0.7 }
] } },
"arrangement": ["a"]
```

And the same two bars as three chords, which render to exactly those twelve
notes:

```json fields
"tracks": [{ "name": "keys", "patch": {
  "source": { "kind": "karplus", "damping": 0.99, "brightness": 0.4 },
  "amp": { "a": 0.002, "d": 0.6, "s": 0.0, "r": 0.3 } } }],
"patterns": { "a": { "beats": 8, "notes": [
  { "track": "keys", "chord": "Dm7", "oct": 3, "start": 0, "dur": 2, "vel": 0.8 },
  { "track": "keys", "chord": "Gm7", "oct": 3, "start": 2, "dur": 2, "vel": 0.8 },
  { "track": "keys", "chord": "A7",  "oct": 3, "start": 4, "dur": 4, "vel": 0.7 }
] } },
"arrangement": ["a"]
```

Twelve objects to three is the small half of the argument. The larger half is
that the first version says nothing about those four notes being one thing:
transposing the chord is four coordinated edits, changing its quality is four
more, and any one of them can disagree with the others — leaving a chord that
is not wrong enough to be refused and not right enough to be what was meant.
Nothing catches that; only ears find it.

**How a chord is voiced.** `oct` places the **root** in scientific pitch
notation, and the rest of the chord is stacked upward from it in close
position, nothing dropped and nothing spread: `Dm7` at octave 3 is `D3 F3 A3
C4`. Absent, `oct` is 3, which puts a seventh chord across middle C. The
boring voicing is the deliberate one — it is what the name literally says, a
reader can work it out in their head, and it re-qualifies and transposes
without moving notes nobody asked about. Anything cleverer is taste, and taste
belongs to the document.

**The names, and only these names.** A name is looked up in this table or the
song is refused; nothing is guessed at. The root is a letter `A`–`G`, **upper
case**, optionally followed by `#` or `b`. A `/bass` suffix puts that pitch
class in the octave below the root, which is how an inversion is written.

| family | qualities |
| --- | --- |
| triads | *(none)*, `maj`, `m`, `min`, `dim`, `aug`, `sus2`, `sus4`, `5` |
| sixths | `6`, `m6` |
| sevenths | `7`, `maj7`, `m7`, `min7`, `mmaj7`, `m7b5`, `dim7`, `aug7`, `7sus4` |
| altered sevenths | `7b5`, `7b9`, `7#9` |
| ninths | `9`, `maj9`, `m9`, `add9`, `madd9` |

So `Dm7`, `F#maj9`, `Bb7`, `Absus4`, `C/G` and `Gm7/D` are all chords, and
`CM7`, `dm7`, `C11` and `Cmaj13` are all refusals. `M` is absent because it is
one shift key from `m` and means the opposite. Elevenths and thirteenths are
absent because they are named for a tone they add and played by dropping
others, and which ones to drop is a judgement — which is exactly what a closed
table must not make on your behalf.

**When no name fits**, write the pitches instead. `chord` takes either a name
or a list, and a list keeps what the name form was for: one `start`, one `dur`
and one `vel` across voices that stay one idea in the document.

```json fields
"patterns": { "a": { "beats": 4, "notes": [
  { "track": "t", "chord": ["D2", "A3", "D4", "F4", "C5"], "start": 0, "dur": 4 }
] } },
"arrangement": ["a"]
```

A spelled chord already carries an octave per voice, so `oct` beside one is
refused rather than ignored.

**Each voice is a note from there on.** Expansion happens before the
performance, so a chord picks up [swing and humanise](#playing-it-rather-than-clocking-it)
one voice at a time and does not land as a perfectly simultaneous block — which
is a real part of sounding played, and free.

### Writing a rhythm as a rhythm

A percussion part is one entry too, told apart by carrying `steps`. Here is one
bar of sixteenth-note hi-hats, accented on the quarters. First, one hit at a
time:

```json fields
"tracks": [{ "name": "hat", "gain": 0.5, "patch": {
  "source": { "kind": "noise" },
  "amp": { "a": 0.001, "d": 0.04, "s": 0, "r": 0.03 },
  "filter": { "kind": "highpass", "cutoff": 6000 } } }],
"patterns": { "a": { "beats": 4, "notes": [
  { "track": "hat", "note": "C4", "start": 0.00, "dur": 0.25, "vel": 1.0 },
  { "track": "hat", "note": "C4", "start": 0.25, "dur": 0.25, "vel": 0.45 },
  { "track": "hat", "note": "C4", "start": 0.50, "dur": 0.25, "vel": 0.45 },
  { "track": "hat", "note": "C4", "start": 0.75, "dur": 0.25, "vel": 0.45 },
  { "track": "hat", "note": "C4", "start": 1.00, "dur": 0.25, "vel": 1.0 },
  { "track": "hat", "note": "C4", "start": 1.25, "dur": 0.25, "vel": 0.45 },
  { "track": "hat", "note": "C4", "start": 1.50, "dur": 0.25, "vel": 0.45 },
  { "track": "hat", "note": "C4", "start": 1.75, "dur": 0.25, "vel": 0.45 },
  { "track": "hat", "note": "C4", "start": 2.00, "dur": 0.25, "vel": 1.0 },
  { "track": "hat", "note": "C4", "start": 2.25, "dur": 0.25, "vel": 0.45 },
  { "track": "hat", "note": "C4", "start": 2.50, "dur": 0.25, "vel": 0.45 },
  { "track": "hat", "note": "C4", "start": 2.75, "dur": 0.25, "vel": 0.45 },
  { "track": "hat", "note": "C4", "start": 3.00, "dur": 0.25, "vel": 1.0 },
  { "track": "hat", "note": "C4", "start": 3.25, "dur": 0.25, "vel": 0.45 },
  { "track": "hat", "note": "C4", "start": 3.50, "dur": 0.25, "vel": 0.45 },
  { "track": "hat", "note": "C4", "start": 3.75, "dur": 0.25, "vel": 0.45 }
] } },
"arrangement": ["a"]
```

And the same bar as one string, which renders to exactly those sixteen notes:

```json fields
"tracks": [{ "name": "hat", "gain": 0.5, "patch": {
  "source": { "kind": "noise" },
  "amp": { "a": 0.001, "d": 0.04, "s": 0, "r": 0.03 },
  "filter": { "kind": "highpass", "cutoff": 6000 } } }],
"patterns": { "a": { "beats": 4, "notes": [
  { "track": "hat", "steps": "XxxxXxxxXxxxXxxx", "div": 0.25, "vel": 0.45 }
] } },
"arrangement": ["a"]
```

Sixteen objects to one is the smaller half of the argument. The larger half is
that the second version **can be read as a rhythm**: the hits and the gaps are
where they sound, so a whole bar arrives at a glance and a revision is one
character. A list of objects with float `start` values has to be counted, and
most of the work on a piece of music is revision.

**Three characters, and no fourth.** `x` is a hit, `X` is an accent, `-` is a
rest, and anything else is refused — including the `|` bar lines and the spaces
a tracker screen would have drawn for you. Every character is one step, the
count is what proves the string covers its bar, and a character that looked
like a step but was not would take that with it.

| field | means |
| --- | --- |
| `steps` | the string itself, one character per step |
| `div` | how long one step is, in beats — `0.5` an eighth, `0.25` a sixteenth |
| `start` | where the **first step** falls, in beats. Absent means 0 |
| `dur` | gate length for every hit. Absent means one step |
| `note` | what pitch every hit plays. Absent means middle C |
| `vel` | velocity of a plain `x`. An `X` always plays at 1 |

**Velocity is the shift key.** Two levels is all the notation carries, because
`4-4-4-4-4-4-4-49` makes you compare digits to find the accent where
`x-x-x-x-x-x-x-xX` shows it as a silhouette. What the two levels *mean* is
still yours: an accent is the hardest an instrument is struck, which is
velocity 1, so `vel` is the plain hit and the distance between them is the one
number you write. `vel` at 0.4 is a quiet hat with a hard accent; at 0.85 it is
a nearly even one; at 1 the two cases are the same hit, and a string using both
is **refused** rather than played flat.

**The string covers its pattern exactly**, from `start` to the end of the slot,
rests included — so a figure in the last bar is written with the rests in front
of it rather than positioned by a number:

```json fields
"tracks": [
  { "name": "kick", "patch": {
    "source": { "kind": "osc_stack", "oscs": [{ "wave": "sine" }] },
    "amp": { "a": 0.001, "d": 0.22, "s": 0, "r": 0.02, "curve": 4.0 },
    "pitch_env": { "semitones": 10,
      "adsr": { "a": 0, "d": 0.04, "s": 0, "r": 0, "curve": 5.0 } } } },
  { "name": "snare", "gain": 0.6, "patch": { "source": { "kind": "noise" },
    "amp": { "a": 0.001, "d": 0.14, "s": 0, "r": 0.06 },
    "filter": { "kind": "highpass", "cutoff": 1200 } } }
],
"patterns": { "a": { "beats": 8, "notes": [
  { "track": "kick",  "steps": "X--x--X---x-X---", "div": 0.5, "note": "C1", "vel": 0.7 },
  { "track": "snare", "steps": "----X-------X-xX", "div": 0.5, "vel": 0.6 }
] } },
"arrangement": ["a"]
```

`div` is therefore redundant — it could be derived from the length and the
slot — and **the redundancy is the check**. A string one character short reads
as a bar until the ear finds it, and comparing the count you typed against the
count the grid needs is the only thing that catches it. So fifteen sixteenths
in a four-beat pattern is refused, and told how many it needed.

**Each hit is a note from there on.** Expansion happens before the performance,
so a step string picks up [swing and humanise](#playing-it-rather-than-clocking-it)
one hit at a time and is played rather than clocked — the same way a chord's
voices are.

**One pitch per entry, and no melody.** A percussion track plays one sound
throughout, so `note` sits on the entry. A pitch per step would need a
character per pitch, and at that point the string is no longer legible as a
shape, which is what it was for. Melody is written as notes.

A **Euclidean generator** (`{ "hits": 5, "steps": 16 }`) is deliberately
absent, for the reason inversion and retrograde are: it is elegant, and it is
one idea past what anybody reaches for by hand. It would also cost the thing
this notation buys — a generated rhythm cannot be seen without running the
algorithm in your head — and it saves eleven characters over writing out what
it would have produced.

### Repeating a pattern differently

An arrangement entry is a pattern's **name**, or that name with **transforms**.
The middle entry above is the same four bars an octave up, played quieter, by
the lead alone — three fields instead of a second pattern written out note by
note.

| field | does |
| --- | --- |
| `transpose` | adds semitones to every note — a key change, an octave double, a lifted final chorus |
| `vel_scale` | multiplies every velocity — a quiet reprise, a half-time breakdown |
| `tracks` | plays only these tracks; everything else is silent for that entry |

Reach for these. Without them the format prices variation out: a repeat costs
one word and a variation costs a whole pattern, so a song written under that
pressure comes out four thin patterns played twice each — sparse and
symmetrical, for the same reason water runs downhill. A transform costs less
than either, and it puts the relationship between two sections **in the
document**, where an edit to one keeps the other in step and a reader can tell
a deliberate variation from an inconsistency.

Three things worth knowing before you write one:

- **Transforms do not stack.** Every entry transforms the pattern *as written*,
  never what the entry before it produced, so the tenth repeat is not nine
  octaves up.
- **`tracks` is a filter, not a remap.** It says which of the song's tracks
  sound; it cannot move a note to a different instrument. If it could, a
  pattern would mean something different depending on where it was played, and
  then it is no longer a block of music.
- **A transpose off the end of the keyboard is clamped, not refused.** Whether
  a transpose is legal should not depend on the register of a pattern written
  months ago.

Inversion, retrograde, augmentation and fragmentation are deliberately absent.
They are real compositional operations and also the ones nobody reaches for by
hand; if a real song wants one, that is a request with the song attached.

### Playing it, rather than clocking it

Every note lands on its exact beat at exactly the velocity written, forever.
That is the loudest thing a generated piece says about itself — louder than the
notes — and it survives every improvement made to the composition. Two optional
fields say otherwise, and both are absent by default.

```json fields
  "swing": 0.15,
  "humanize": { "timing": 0.012, "velocity": 0.08, "timbre": 0.12 }
```

**`swing`** sits the off-beat eighths late: `0` is straight, `0.33` is roughly
the triplet feel, `0.5` is dotted. The beat is stretched *around* the off-beat
rather than the eighth being picked out and moved, so finer subdivisions ride
along with it and two notes never swap places. Onsets only — a note's `dur` is
what the document says it is. It is a property of the performance, so it is
song-level: a rhythm section that swings while the lead does not is a specific
effect, not a default.

**`humanize`** scatters each note by a bounded amount, on three axes:

| field | scatters | a natural band | a loose one |
| --- | --- | --- | --- |
| `timing` | when the note lands, **in seconds** either way — not beats, because a player is not three times sloppier at 40 bpm | `0.012` | `0.04` |
| `velocity` | how hard it is struck, as a fraction of the velocity written — level *and* the brightness that comes with it | `0.08` | `0.2` |
| `timbre` | how it is struck: the brightness alone, as a fraction of the velocity played, with the level left where it is | `0.12` | `0.3` |

A note written at full velocity can only come out quieter, so a piece that
wants dynamics in both directions writes its notes below `1.0`. `timbre` is not
a milder `velocity`: leaning on a note makes it louder *and* brighter, which is
what `velocity` does, while changing the touch makes it brighter at the same
level. It reaches an instrument through the two routings that already read
velocity as effort — a filter's `vel_cutoff` and `fm2`'s `vel_index` — so a
patch that names neither hears nothing from it, which is the right silence:
what "brighter" means belongs to the instrument.

All three draw from the same seed chain everything stochastic here draws from,
keyed per note in *arrangement* order. So the piece stays byte-identical across
runs, `seed` re-rolls the whole performance at once, and a pattern played twice
is played differently the second time — which is the point. A repeat nudged
identically both times is still a photocopy, just a crooked one.

The oscillators themselves start somewhere in their cycle rather than at zero,
drawn from that same chain, so two hits of one patch are not the same waveform
and a detuned pair is already drifting at the attack. Nothing asks for it and
nothing can turn it off: a repeat that was a photocopy is the thing this whole
section is about, and it was one before any of these fields were written.

### Where an effect goes

A chain can live in three places. Which one you pick is most of the difference
between *some synth parts* and *a piece of music*, so it is worth a moment.

```json fields
  "tracks": [
    { "name": "t", "gain": 0.8,
      "patch": {
        "source": { "kind": "noise" },
        "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 }
      },
      "fx": [{ "fx": "delay", "time": 0.28, "feedback": 0.35, "mix": 0.2 }] }
  ],
  "fx": [
    { "fx": "reverb", "size": 0.7, "damp": 0.4, "mix": 0.18 },
    { "fx": "saturate", "drive": 1.5, "mix": 0.5 }
  ]
```

| where | runs on | reach for it when the effect is |
| --- | --- | --- |
| `patch.fx` | every note of that instrument, one at a time | part of the **sound** — the corridor a gunshot is fired in, a slapback that *is* the instrument |
| a track's `fx` | that instrument's whole part, before its `gain` | part of the **performance** — a delay whose repeats answer the phrase rather than each note |
| the song's `fx` | the sum of every track, before the limiter | the **room** — one space the whole piece is playing in |

**Reverb belongs on the song.** Put it on a patch and each instrument is in its
own room, drifting apart the moment one of them is tuned; put it on the song and
it is one setting, decaying once across everything and ringing past the last
note as a single tail. It is also less work for the machine: one room, convolved
once, instead of the same room convolved for every note in the piece.

**Drive belongs in all three, for three different reasons.** It is the one
effect with a real use in each place, because it is not describing a space — it
is changing the thing itself, and there are three different things. On a patch
it is part of the instrument's voice: a bass that growls the harder it is
played. On a track it thickens that one part and leaves the others alone. On
the song it is **glue** — every track pushed gently into the same curve, which
is much of what stops a mix sounding like separate parts added together, and
the reason a low `drive` at a partial `mix` over the whole piece is worth trying
before anything else. The three stack, so keep each modest.

**EQ belongs on a track, nearly always.** It is the one effect here that is a
mixing decision rather than a sound, and a mix is made of instruments sitting
out of each other's way — which is a statement about one track at a time. On
the song it is a last, gentle move over everything (a shelf of a decibel or
two); on a patch it is rare and usually means the instrument itself was written
wrong, which is cheaper to fix at the source than to correct downstream.

Both fields default to empty, and an empty one is not written down — a song that
does not use them is exactly the song it was before they existed.

A song chain runs **before** the limiter, always. Nothing may add gain after the
thing that guarantees a bake does not clip. Fades still come last, after
limiting, as they always did.

### Fitting a song to the cut

A song's natural length is whatever its notes add up to, plus the ring-out.
When the picture decides the length instead, say so — all three fields are
optional, and absent means the song is as long as it is.

```json fields
  "fit": { "seconds": 43.0, "mode": "loop" },
  "fade": { "in_seconds": 1.5, "out_seconds": 3.0 },
  "tail": "ring"
```

**`fit`** makes the file exactly `seconds` long, to the sample. `mode` is:

| mode | What it does | Use it for |
| --- | --- | --- |
| `loop` *(default)* | repeats the arrangement, cutting mid-arrangement where the time runs out | a bed under dialogue — the seam is inaudible under speech |
| `stretch` | moves the tempo so a whole number of passes lands on the target | music that has to stay intact |
| `once` | plays through once and pads with silence | a sting |

`stretch` **refuses** if it would have to move the tempo by more than 25%,
and says what tempo it would have needed — a bed at 40% speed is not a bed.
Reach for `loop` when that happens.

A cut is always faded over about 20 ms, because a buffer truncated at an
arbitrary sample ends mid-waveform, and that is a click.

**`fade`** moves the level at each end of the finished piece. This belongs to
the *music* — a piece that ends by resolving and receding stays that way when
it is moved elsewhere in the timeline or reused in another project. Ducking
*this* use of it under *this* voice-over is a keyframe on the clip instead.

**`tail`** is `ring` (the default: the file grows to hold the last note's
release and any fx tail) or `exact` (the file ends on the arrangement's final
beat, with the tail faded into it). Use `exact` when the music has to butt
against something.

`seed` re-rolls every stochastic source in the piece at once. A pattern played
twice does not repeat its noise — a repeated snare is not a photocopy — while
the whole piece stays byte-identical across runs.

## What a bake is, and when it happens

`synth bake` renders every recipe whose output is not already on disk, and
writes it to `generated/<digest>.wav` — the digest being a hash of the recipe's
bytes together with the version of the synthesiser that rendered them.

That naming is the whole cache. The path an asset holds *is* the record of what
produced it, so editing a recipe makes the asset stale by arithmetic — nobody
has to mark it, and re-running `bake` is free when nothing changed.

### The synthesiser is the other half of a bake

A recipe is one of **two** inputs to a render. The other is the synthesiser,
and it moves too: a change to a source, a filter, an envelope or an effect
changes what every recipe in every project renders to. "The same bytes every
time" is a promise about a document handed twice to *one* synthesiser, and it
was never a promise across versions of scorsese.

So the address carries a version number, and the effect is the one you would
want:

- **Nothing to do, ever.** When the synthesiser changes, the recipe you have
  not touched hashes to an address the old file is not at. The next
  `synth bake` misses, re-renders it and points the asset at the new file — the
  same path an edited recipe takes. There is no migration, no flag and no list
  of affected projects to work through.
- **The superseded file is left where it is**, exactly as an edited recipe's
  previous bake is. `generated/` is rebuildable output, and a bake nothing
  points at is a few hundred kilobytes rather than a problem.
- **The re-bake is free.** Seconds of CPU and no network, which is the reason
  re-rendering is the right answer here rather than reporting a mismatch and
  asking somebody to decide.

The number is `SYNTH_VERSION` in `crates/zimmer/src/lib.rs`. It is declared
rather than computed: the honest way to derive it would be to hash rendered
output, and this crate's voices ride on the platform's `sin`, `exp` and `powf`,
which are not bit-identical between Linux and macOS. A digest has no tolerance
to spend, so a derived version would claim the synthesiser had changed every
time a project moved machine. **Changing what a recipe renders to means bumping
it in the same commit**, the way a `project.json` format change means bumping
`schema_version`.

Output is **mono, 16-bit PCM, 44.1 kHz**. A render resamples and upmixes it
into the mix exactly as it would any imported file, so a recipe never has to
know what the finished video will be delivered at.

Mono is a settled decision rather than a stage on the way to stereo, so there
is no panning and no width: a recipe places a sound in *time*, never in space.

## How a bake came out

A bake says how it turned out, and so does a render — mean and peak for the
whole thing, then the same again for each stretch of it, plus the share of its
energy that is low, mid and high:

```
trilha — baked, 3702 KB
  generated/9f3c….wav
  43.0 s  mean -14.4 dBFS, peak -2.9 dBFS, low  47%  mid  45%  high   8%
     0:00-0:08 intro   mean  -19.1   peak   -8.2   crest  10.9   low  42%  mid  51%  high   7%
     0:08-0:24 verse   mean  -13.8   peak   -3.1   crest  10.7   low  38%  mid  49%  high  13%
     0:24-0:40 chorus  mean  -13.9   peak   -2.9   crest  11.0   low  61%  mid  33%  high   6%
     sub     mean  -18.9   peak   -6.1   crest  12.8   low  96%  mid   4%  high   0%
     pad     mean  -21.4   peak  -11.0   crest  10.4   low  71%  mid  28%  high   1%
     arp     mean  -25.2   peak  -12.7   crest  12.5   low   6%  mid  88%  high   6%
     hat     mean  -33.7   peak  -14.2   crest  19.5   low   3%  mid  22%  high  75%
```

The rows with a clock on them are the **arrangement's own sections** when there
is an arrangement, which is what lets a finding be "the second chorus is the
quiet one" rather than "seconds 24 to 32 are quiet". A one-shot, an imported
file and a rendered mixdown have no arrangement, so those are cut on a fixed
interval instead. A piece with only one stretch gets no rows at all — one row
under a one-line summary is the same sentence twice.

*crest* is peak minus mean: the cheapest proxy there is for "does this have
dynamics, or is it a wall". The three band shares are a coarse split at 250 Hz
and 4 kHz, enough to say **muddy**, **thin** or **balanced** — not a spectrum
analyser, and deliberately not one.

### Which layer is taking up the room

The rows with a **name** on them are the song's tracks: the same figures again,
split by instrument instead of by time. That is the half a summary cannot
answer. "87% of the energy is below 250 Hz" is a correct diagnosis of a
five-instrument mix and an address nobody can act on — the only available
response is to change four instruments at once and re-bake, and if it works,
nobody learns which change did it. A row per track turns that guess into a
measurement: above, the sub is the low end and the pad is most of what is left
of it, so those are the two faders worth touching.

- **Post-gain.** A row is what that track contributes *at its fader*, which is
  what it takes up in the mix — not what it would sound like soloed. The song's
  own effects, the master limiter and the fades are not in these numbers;
  they belong to the sum, and a row that included them would answer a question
  about the piece under the name of a track.
- **One line per track for the whole piece**, never per section. Five
  instruments over four sections is twenty rows in a report usually read as
  "fine, carry on", and the per-section detail is already on the sum above.
- **Measured over the length of the piece**, so a hat that plays in one section
  does not read as louder than the pad it sits over.
- **A track that never played says `silent`**, which is a finding rather than a
  gap: an instrument written into the tracks and left out of every pattern is a
  mistake worth seeing.
- **Only for a song of more than one track.** A one-shot is one gesture played
  by one voice, and a single track's row would repeat the summary above it —
  the same rule the section rows follow.

There is nothing to switch on: the rows are always printed, in
`scorsese synth bake` and over MCP in `synth_bake`. `scorsese level` and
`audio_level` do not have them, and cannot — they measure a finished file,
which has no tracks in it any more, and re-rendering the piece to find them
would cost what measuring it while it was made costs nothing.

`scorsese level <file>` says the same about any finished file, and
`--against <other>` compares the two field by field:

```
trilha.wav  vs  trilha.prev.wav
  mean     -14.4 dB     ( -4.6 dB quieter )
  peak      -2.9 dB     ( -0.2 dB quieter )
  crest     11.5 dB     ( +1.2 dB more dynamic )
  bands   low  47% ( +9 pts )   mid  45% ( -3 pts )   high   8% ( -6 pts )
```

That is the form with the most teeth. An absolute number is hard to judge — is
−14 dBFS good? it depends entirely — and a difference is not. Over MCP the same
two answers come back from `audio_level`.

**All of it is a signal and none of it is a gate.** There is no correct
loudness: a sting is meant to be hot, a bed under narration is meant to be far
down, and a threshold that refused a bake would be a taste enforced as a build
failure. Nor is it a critic. Measurement finds defects — too quiet, clipping,
muddy, a section flat where the arrangement said climax. It does not find
taste, and a metric treated as an ear produces music that optimises the number
and gets worse.

### Worked: from a report to a fix

Take the report at the top of this section at its word. Three readings come out
of it, and only one of them is a problem:

- **`0:24-0:40 chorus … low 61%`** against 42% and 38% for the two sections
  before it. Something joins in at the chorus and it is all bottom end. The
  piece will read as *muddy* there and nowhere else.
- **`sub … low 96%`** is not the fault. A sub is supposed to be entirely low;
  a row saying so is an instrument doing its job.
- **`pad … low 71%`** is. A pad's job is the middle, and this one is putting
  most of its energy underneath the sub — two instruments stacked in one
  octave, which is the textbook cause of exactly the reading above.

The old answer was the pad's `gain`, and it does not work: turning the pad down
takes the chords out of the chorus along with the mud. The answer is to leave
the pad exactly as loud as it is and take the bottom out of *it*.

Here are the two tracks the finding is about, on their own — a sub holding the
root and a pad written a little too far down on top of it, which is how a mix
gets into this state in the first place:

```json recipe
{
  "recipe": "song",
  "bpm": 96,
  "tracks": [
    { "name": "sub", "gain": 0.9,
      "patch": {
        "source": { "kind": "osc_stack", "oscs": [ { "wave": "sine" } ] },
        "amp": { "a": 0.02, "d": 0.3, "s": 0.8, "r": 0.3 }
      } },
    { "name": "pad", "gain": 0.7,
      "patch": {
        "source": { "kind": "osc_stack", "oscs": [
          { "wave": "saw", "detune_cents": -6 },
          { "wave": "saw", "detune_cents": 6 }
        ] },
        "amp": { "a": 0.4, "d": 0.6, "s": 0.6, "r": 0.8, "curve": 3.0 }
      },
      "fx": [ { "fx": "eq", "bands": [
        { "kind": "high_pass" },
        { "kind": "peak", "freq": 400, "gain_db": -4, "q": 1.2 }
      ] } ] }
  ],
  "patterns": {
    "chorus": { "beats": 4, "notes": [
      { "track": "sub", "note": "E1", "start": 0, "dur": 4 },
      { "track": "pad", "note": "E2", "start": 0, "dur": 4 },
      { "track": "pad", "note": "B2", "start": 0, "dur": 4 }
    ] }
  },
  "arrangement": ["chorus"],
  "fx": [ { "fx": "eq", "bands": [ { "kind": "high_shelf", "gain_db": 2 } ] } ]
}
```

Four lines, and each one is a line of the report answered:

- `{ "kind": "high_pass" }` on the pad, at its default 250 Hz — **the same
  boundary the `low` column is measured at**. Everything the report counted as
  the pad's low share is gone, and the bottom of the piece is the sub's alone.
- `{ "kind": "peak", "freq": 400, "gain_db": -4, "q": 1.2 }`, just above it. A
  high-pass leaves the 250–500 Hz box untouched and that is where a mix goes
  *boxy* rather than *boomy*; a moderate dip there is the second half of the
  same fix.
- `{ "kind": "high_shelf", "gain_db": 2 }` on the song, at its default 4 kHz —
  the boundary the `high` column is measured at. The summary said `high 8%`,
  and with the low end no longer crowded there is now room to hear the top.
- Nothing moved a `gain`. The balance the piece was written with is the balance
  it still has.

Then bake it again and read the same rows, because the report is how the fix is
checked as well as how it was found. On the sketch above the pad's row goes from
`low 76% mid 24%` to `low 27% mid 71%`, and the sub's row does not move at all —
that second half is the real check, because a fix that moved both would have
been a fix aimed at the mix rather than at the layer the mix said was wrong.

## What the whole set is made of

Everything above measures **one** bake. `scorsese synth survey`, and
`synth_survey` over MCP, read every song recipe in the project instead and say
what the set is made of:

```text
01-km      96 bpm  E2-A5  C D E F G A B (diatonic on C)
  hat   noise      gain 0.60  duty  20%  sustain 0.00  4.0/s  F#5  cutoff 9000 Hz
  pad   osc_stack  gain 0.40  duty  95%  sustain 0.62  0.3/s   B3  no filter
02-calado  132 bpm  E2-G5  C D E F G A B (diatonic on C)
  arp   karplus    gain 0.76  duty  35%  sustain 0.00  8.4/s   E5  cutoff 4400 Hz
  bell  fm2        gain 0.50  duty  70%  sustain 0.00  2.0/s   A5  no filter
2 songs
  osc_stack  in 1, loudest in 1
  fm2        in 1, loudest in 1
  karplus    in 1, loudest in 0
  noise      in 1, loudest in 0
  tempo      96-132 bpm
  register   E2-A5
```

A **song line** carries the tempo it plays at (the written one, unless `fit`
stretched it), the register its notes reach, and the pitch classes they fall
in. The collection is named only when the set of classes *is* exactly a
diatonic one — that is arithmetic, and naming a mode would mean picking a tonic
the document never states.

A **track row** has two halves, and the split is the useful part.

What the instrument **is**: its source kind in the word the recipe spells it
with, its gain as written, and where its filter starts.

What it **does** in the piece: the **duty** — the share of the arrangement it is
sounding over at all — its `amp.s` **sustain**, how many notes per second it
plays over one pass, and the **median pitch** of those notes as a note name. A
track that plays nothing at all reads `silent` rather than `0.0/s`, and has no
middle note to name.

That second half is there because the first half does not predict what anyone
hears. Three cues can read `karplus`, `fm2` and `osc_stack` — three different
instruments on paper — and be one plucked guitar to a listener, because what
they actually share is a sustain of zero, a couple of notes a second and a
register up around F♯5. Changing the source kind does not move that complaint,
and the row now shows why.

Every column is written, not measured — this costs no bake, so `gain 0.60` is
the number in the score rather than a level anything came out at, and `duty` is
counted off the note starts and lengths rather than off any sound. A song of one
track gets no rows, the same rule the section and layer tables follow.

**`sustain` is the envelope's, not the source's**, and it is worth knowing
before trusting the column. A `karplus` string damps on its own and an `fm2`
operator's modulator decays on its own, so a patch can read `sustain 0.40` and
still be a sound that dies away. The column reports what the document says, in
the same "written, not measured" spirit as `gain` — read it as *what the
envelope was told to do*, not as *how long you will hear it*.

The **rollup** counts the same facts across the project: how many songs use
each source kind, how many carry it in their *loudest* track, and the spread of
tempos and registers.

**"Loudest" means `gain × duty`**, and that is why the duty is on the row: it is
the second half of a number you would otherwise have to take on faith. Written
gain alone is a per-note *peak* — it says how loud a note is and nothing about
how much of the piece has any note in it — and percussion is written loud
exactly because it is short. The example above is the shape of the mistake: the
hat is written at `0.60` and the pad at `0.40`, and the pad is what the piece
sounds like, because the hat is only there for a fifth of it.

That matters more than a wording nit, because the rollup line is the one
sentence in this project that can catch *six cues, one instrument*. Ranked by
gain alone it once crowned a hi-hat in two songs out of six and reported a
**more varied** set than the one on disk — the exact wrong answer, since a
client with no ears has to take the line literally and would stop looking.

**It is a better proxy, not an ear.** A plucked harp sitting 27 dB down can
still be the instrument you hear, because transients carry prominence that no
written number and no energy average knows about. Nothing here names a lead with
authority, and a row that surprises you is worth a `synth_bake` and a listen.

None of it costs anything: no bake, no samples, no ffmpeg, no network. It is
parsing files that were going to be parsed anyway, which makes it the cheapest
signal in the project and the one with the least excuse for being missing. A
client with no ears cannot hear that six cues are one guitar six times, but it
can absolutely count it.

**It counts and stops.** No score, no grade, no recommendation, and above all
no diversity number. Six variations on one instrument is a legitimate thing to
write on purpose, and by the rule stated just above — a metric treated as an
ear produces music that optimises the number — a measure of variety is exactly
the metric that would get optimised. Nothing here can fail anything.

A project of fewer than two songs gets **no report at all**: a survey is across
a set, and one row under a summary of that row is the same sentence twice.
One-shot patches are left out for the same kind of reason — an effect has no
tempo, no arrangement and no mix to be the loudest thing in.

## What is refused

Only what would produce silence, a divide-by-zero, an unstable filter or an
unbounded allocation: an empty oscillator stack, a stack weighted entirely to
zero, a non-positive `fm2` ratio, a cutoff at or below 0 Hz, a negative LFO
rate, a note of no length, an arrangement naming a pattern that does not exist,
a note on a track that does not exist, a `tracks` filter naming one that does
not either, a `vel_scale` below zero, a `swing` outside `0..1` (at 1 the
off-beat lands on the next beat, which reorders the music), and a negative
`humanize` amount.

Chords are the one place something is refused for a second reason: a name off
the [quality table](#writing-a-chord-as-a-chord), a chord voiced past either
end of the keyboard, an empty list of pitches, and `oct` written beside a
chord that spelled its pitches out. None of those would be silence — they
would be *a different chord*, which nothing downstream can notice.

[Step strings](#writing-a-rhythm-as-a-rhythm) are refused for the same second
reason, and it is why the checks are there at all: a character that is not
`x`, `X` or `-`, a string whose length is not the length its grid needs, a
`div` no whole number of steps fits the slot with, and both cases used beside a
`vel` of 1, where the accents the page shows would not be in the audio. Every
one of those would otherwise be *a different rhythm* — usually a shorter one,
which is the failure nobody hears until much later.

Musical taste is not checked. An ugly patch renders.

## What CI checks about this page

Every fenced `json` block here carries a marker saying what it is a piece of,
and a test completes it into a whole recipe:

| marker | the block is | checked by |
| --- | --- | --- |
| `recipe` | a complete recipe | parsing **and rendering** it |
| `fields` | top-level fields of a song | splicing them into a minimal song, then both |

An unmarked `json` block fails rather than quietly escaping the check. Blocks
marked `jsonc` are illustrative fragments and make no claim to be complete.

Rendering, not just parsing: a recipe that parses can still be silent, and an
example nobody can play is worse than no example.

What this never proves is that the prose next to a block is *true*. That is the
limit of any documentation gate, and it is written here so a green check is
never mistaken for an accurate page.

## Where this fits

`synth_audio` is one of the three generated asset kinds — see
[`project-format.md`](project-format.md#assets). It does not replace
`generated_audio`: that one is ElevenLabs, for **voice**. No amount of
arithmetic will read a line of narration aloud.
