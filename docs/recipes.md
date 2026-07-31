# Recipes — sound from a document

A **recipe** is a JSON file under `recipes/` that a `synth_audio` asset is made
from. Rendering one costs nothing, needs no key and no network, and produces
the same bytes every time — so a project can carry its whole sound design as
text and rebuild it anywhere.

Two shapes, told apart by the `recipe` field: **`patch`** is one instrument
playing one note (an effect), **`song`** is a piece of music.

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
   └─────── lfo ──────────┘        one target: pitch | cutoff | amp
```

**`source`** — one of four, tagged by `kind`:

| `kind` | Fields | Good for |
| --- | --- | --- |
| `osc_stack` | `oscs`: up to 4 of `{ wave, detune_cents, gain, octave }` | leads, basses, pads |
| `karplus` | `damping` 0..1, `brightness` 0..1 | plucked strings, marimbas |
| `noise` | — | gunshots, impacts, footsteps, wind |
| `fm2` | `ratio`, `index`, `mod_decay` | bells, electric pianos, metal |

`wave` is `sine`, `triangle`, `saw` or `square`. Integer `ratio` on `fm2` stays
tonal; a fractional one goes metallic.

**`amp`** — an ADSR: `{ "a": 0.005, "d": 0.15, "s": 0.4, "r": 0.2 }`. Attack,
decay and release are seconds; sustain is a level in `0..=1`. **Required.**

**`filter`** *(optional)* — `kind` is `lowpass` or `highpass`, plus `cutoff` in
Hz, `resonance` 0..1, `env_amount` in Hz, and its own `adsr`. `env_amount` is
what makes a patch expressive rather than static: it opens the cutoff on the
attack and closes it as the note decays. A pluck is a lowpass with a big
positive `env_amount` and a fast filter decay.

**`lfo`** *(optional)* — `{ "rate": 5.0, "depth": 0.5, "target": "pitch" }`.
`pitch` is vibrato in semitones, `cutoff` is wobble in octaves, `amp` is tremolo
where 1.0 dips to silence.

**`fx`** *(optional)* — a list, applied in order: `{ "fx": "delay", "time",
"feedback", "mix" }` and `{ "fx": "reverb", "size", "damp", "mix" }`. A limiter
always runs after them and is not listed — a bake must not clip, and that is
not the recipe's decision.

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
  "arrangement": ["a", "a"]
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
- **`arrangement`** — which patterns play, in order. Repeats are just repeats.

Write repetition as repetition. A melody with any structure is a few short
patterns and a list naming them; the same music as one flat note list is
hundreds of lines, and every edit becomes a merge conflict with itself.

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
writes it to `generated/<sha256 of the recipe's bytes>.wav`.

That naming is the whole cache. The path an asset holds *is* the record of
which recipe produced it, so editing a recipe makes the asset stale by
arithmetic — nobody has to mark it, and re-running `bake` is free when nothing
changed.

Output is **mono, 16-bit PCM, 44.1 kHz**. A render resamples and upmixes it
into the mix exactly as it would any imported file, so a recipe never has to
know what the finished video will be delivered at.

Mono is a settled decision rather than a stage on the way to stereo, so there
is no panning and no width: a recipe places a sound in *time*, never in space.

## What is refused

Only what would produce silence, a divide-by-zero, an unstable filter or an
unbounded allocation: an empty oscillator stack, a stack weighted entirely to
zero, a non-positive `fm2` ratio, a cutoff at or below 0 Hz, a negative LFO
rate, a note of no length, an arrangement naming a pattern that does not exist,
a note on a track that does not exist.

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
