# `project.json` — schema v7

The contract between the CLI, the MCP server, the GUI, and every project
saved on someone's disk. It is meant to be hand-written: an agent should be
able to author a whole video in this file and render it without touching a
mouse.

Changing this format is `architecture` work — it needs a `schema_version`
bump and a migration note.

## The document

```json project
{
  "schema_version": 7,
  "name": "Narrated teaser",
  "timeline_fps": { "num": 30, "den": 1 },
  "assets": [],
  "tracks": []
}
```

`assets` and `tracks` may be omitted; both default to empty. `timeline_fps`
may **not** — see below. Every other field shown below as optional may be
omitted, and omitted is written as *absent*, never as `null`. Unknown fields
are an error, not a warning — a typo like `"trackz"` fails the load rather
than being silently dropped.

## Assets

An asset is an entity. Clips point at assets **by id, never by path**, so
re-importing or regenerating a file is one edit in one place.

| Field | Required for | Meaning |
| --- | --- | --- |
| `id` | all | Unique within the project |
| `kind` | all | `video`, `image`, `audio`, `text`, `color`, `generated_video`, `generated_audio`, `synth_audio` |
| `path` | file-backed kinds | Relative to the project root |
| `sha256` | optional | 64 lowercase hex chars, of the file at `path` |
| `media` | optional | What ffprobe found: `duration_seconds`, `width`, `height`, `frame_rate` (a rational), `audio_channels`, `sample_rate` |
| `prompt` | `generated_*` | What to generate, in words |
| `recipe` | `synth_audio` | Path to the document to synthesise from, by convention under `recipes/` |
| `state` | `generated_*`, `synth_audio` | `sketch`, `queued`, `generated`, `stale` |
| `text` | `text` | The string to render; text assets carry content inline and have no `path` |
| `style` | optional, `text` only | How that string looks: `font`, `size`, `color`, `align`, `line_height`, `max_width` — see below |
| `color` | `color` | The colour to fill with, as `#rrggbb` or `#rrggbbaa`; colour assets have no `path` |

```json asset
{ "id": "shot-city", "kind": "generated_video", "state": "sketch",
  "prompt": "wide aerial of a city at dawn, slow push in" }
```

A generated asset with no media renders as a **slug card** — the brief on a
gray card, with what kind of brief it is and what state it is in written above
it. That is what makes previewing a full cut cost nothing. `GO` generates
exactly the sketch and stale assets; `generated` is a cache hit and is never
redone.

Three kinds are generated, and they differ in what the brief *is*:

| Kind | Brief | Realised by | Costs |
| --- | --- | --- | --- |
| `generated_video` | `prompt` — a sentence | Veo, over the network | money |
| `generated_audio` | `prompt` — a sentence | ElevenLabs, over the network | money |
| `synth_audio` | `recipe` — a document in the project | synthesis, locally | nothing |

An asset carries exactly the brief its kind takes and never the other:
a `recipe` on a Veo asset would never be read, so it is refused rather than
ignored.

Which states have media follows from what the states mean, and `generated` is
the only one that does:

| State | Renders as | Why |
| --- | --- | --- |
| `sketch` | a card | nothing has been generated |
| `queued` | a card | in flight; the file lands when it lands |
| `generated` | its media | the file is what the prompt asked for |
| `stale` | a card | the file exists and is **not** what the prompt now says |

`stale` is the one worth reading twice: the media is still on disk, and
showing it would be showing a shot the project no longer asks for.

A `generated` asset whose file is **not there** — deleted, or never copied
along with the project — renders as a card too, and the render says so in its
report rather than failing. The media can be generated again, and a preview
with one card in it is worth more than no preview at all.

### Narration prompts are visible

A `generated_audio` prompt lives on an audio track, and until it is generated
there is nothing to hear: it contributes **silence** to the mix. Its card
still appears, as a band across the foot of the picture for exactly the frames
its clip covers, so a cut built around a voice-over can be watched before a
word of it has been paid for. Once the audio exists the card is gone and the
picture is untouched — a slug card is a stand-in, never a caption.

### Synthesised audio

```json asset
{ "id": "theme", "kind": "synth_audio", "state": "sketch",
  "recipe": "recipes/theme.json" }
```

A `synth_audio` asset is sound computed from a document the project carries —
a synthesiser patch for an effect, or a song for a score. It sits on an audio
track like any other sound, and behaves like the prompt-backed kinds in every
way the lifecycle cares about: `sketch` until it is realised, silence in the
mix until then, a cache hit once it exists.

What differs is everything about the cost. Realising one needs no network, no
key and no money, and it is **deterministic** — the same recipe produces the
same bytes on any machine, on any run. So the generated file is named for the
SHA-256 of the recipe's bytes, and `path` pointing at a hash the recipe no
longer has *is* what `stale` means for this kind. Nothing else has to record
it.

The recipe is a separate file rather than inline JSON because a recipe is
long: a song is tracks, patterns and an arrangement, and inlining one would
bury the timeline under note lists in the document an agent reads to
understand the edit. It also makes the edit-and-rebake loop a single-file
diff. **What to write in one is [`recipes.md`](recipes.md).**

`synth_audio` does not replace `generated_audio`. That one is for voice —
a line of narration is a sentence, and no amount of arithmetic will read it
aloud.

### Text assets and how they look

```json asset
{ "id": "title", "kind": "text", "text": "Chapter One",
  "style": { "font": "serif", "size": 0.12, "color": "#ffcc00" } }
```

A `text` asset has no file behind it: its content is the `text` field and its
appearance is `style`. (`color` is the other such kind — see below.) **Every field of `style` is
optional, and an absent `style` means all of them** — white, centred, sans,
which is the title most people meant.

| Field | Default | Meaning |
| --- | --- | --- |
| `font` | `sans` | `sans`, `serif`, or a path to a font file inside the project |
| `size` | `0.1` | Em size as a fraction of the frame's **height** |
| `color` | `#ffffff` | `#rrggbb`, or `#rrggbbaa` for text you can see through |
| `align` | `center` | `left`, `center`, `right` — within the wrapped block |
| `line_height` | `1.25` | Baseline to baseline, as a multiple of `size` |
| `max_width` | `0.9` | Where lines wrap, as a fraction of the frame's **width** |

**Sizes are fractions of the raster, not pixels.** Resolution is a render
setting — the same project is previewed at 640×360 and delivered at 4K — so a
title written as `72` pixels would be a different title in each. `size: 0.1` is
a tenth of the picture's height whatever it is rendered at.

**Two font names are reserved.** `sans` and `serif` are the faces scorsese
ships: Liberation Sans and Liberation Serif, metric-compatible with Arial and
Times New Roman, under the SIL Open Font License. Anything else in `font` is
read as a path to a font file the project carries, relative to the project root
like every other path — `assets/Inter-Regular.ttf`. The fonts are committed to
the repository rather than looked up on the system, because a system lookup
resolves differently on every platform and text has to render identically
everywhere.

The text is laid out centred on the frame, wrapped to `max_width`, and
truncated with an ellipsis if it is taller than the picture. **Moving it is
`transform.position.x` and `transform.position.y`**, and fading it is
`opacity` — the same properties that move and fade a video clip, keyframes and
all. Text has no animatable properties of its own, which is why a title that
slides and fades needs nothing here. `fit` is meaningless on a text clip:
there is no source raster to reconcile, since the text is drawn at whatever
size the render is.

Bold, italic, per-character animation, outlines and shadows are not here yet.

### Colour assets

```json asset
{ "id": "black", "kind": "color", "color": "#000000" }
```

A `color` asset is the other kind with no file behind it, and the simpler of
the two: it has no content at all, only appearance. It is a background, a
colour card, a letterbox matte, or the wash under a title — everything that
would otherwise mean generating a PNG of identical pixels and importing a
megabyte of them to say one thing.

The `color` field is required and takes the same notation a text `style` does:
`#rrggbb`, or `#rrggbbaa` for one you can see through. There is no default. A
background is the largest thing on screen, and one that came out white because
nobody chose would be a shot rendered wrong that no error ever mentioned.

**It fills whatever raster the render is**, so it is resolution-independent by
construction — there is no size on it to be wrong at 4K, which is the whole
reason it exists rather than a PNG. For the same reason `fit` is meaningless
on a colour clip, exactly as it is on a text clip: there is no source raster to
reconcile. Neither is an error; both are simply not read.

It composites like any other layer. `opacity` and the transforms already apply,
so a colour that fades up is keyframes and nothing new — and a half-opacity
black over a shot is how you dim one.

Gradients are not here. A gradient has a direction, stops and an interpolation
between them, and that is a different feature wearing this one's clothes.

`media.duration_seconds` is wall-clock, and `media.frame_rate` is a rational
in the same shape as `timeline_fps` — a source's own grid, which is not
necessarily the timeline's.

## The timeline framerate

```json fields
"timeline_fps": { "num": 30000, "den": 1001 }
```

The grid this edit is authored against. Every clip and keyframe time in the
document is a whole frame count on it.

**Rational, not a float.** 29.97 is exactly 30000/1001; 23.976 is 24000/1001.
A float cannot hold either, and rounding them is where long-timeline drift
comes from. `{ "num": 30, "den": 1 }` is plain 30. The fraction is reduced on
load, so `60/2` and `30/1` are the same value.

**Required, with no default.** A missing framerate would leave every time in
the file meaning something other than what its author intended, so a document
without one does not load. Both parts must be non-zero.

**Chosen at project creation** — `scorsese new teaser.scor --fps 30000/1001`,
defaulting to 30. Changing it afterwards is a real operation — rescale the
edit, or reinterpret it at the new rate? — not a field edit. Nothing here
forecloses it; it is simply not something you do by hand.

`--fps` takes `30` or `30000/1001` and refuses `29.97`, for the same reason
the field is a fraction.

### Timeline fps is not output fps

Render settings — resolution, fps, bitrate — are still chosen per render.
(Aspect ratio is not a setting of its own: it is whatever the resolution says,
and how a source of another shape meets it is the clip's `fit`, below.) The
two are different questions:

- the **timeline** framerate answers *what is on screen when*, and
- the **output** framerate is what the file you deliver is encoded at.

A render at a rate other than the timeline's **conforms** from the grid; the
grid stays authoritative.

### Conforming: source fps ≠ timeline fps

A source shot at another rate — a 24fps clip on a 30fps timeline — is
conformed by taking, for each timeline frame, the **nearest source frame in
wall-clock time**. No interpolation, no invented in-between frames: 24→30
repeats source frames in the familiar 2:3 pattern. Optical-flow retiming is a
feature someone can ask for later, not a silent default.

The same rule, in the same direction, covers rendering at an output rate
other than the timeline's.

## Tracks and clips

```json track
{
  "id": "v1", "kind": "video", "name": "Main",
  "clips": [
    { "id": "c-shot", "asset": "shot-city", "start": 0, "duration": 240 }
  ]
}
```

A track is `video` or `audio`. Video tracks composite in array order, first
at the bottom; audio tracks all mix together. Visual assets go on video
tracks, audible ones on audio tracks.

**Order means nothing on audio tracks.** Sounds playing at once are summed,
and addition does not care which came first — there is no "on top" for a
music bed. A clip is heard because it is somewhere, not because of where its
track sits in the list.

**A video clip's own sound is mixed too.** Every camera clip has sound on it,
and a clip on a *video* track whose file carries an audio stream is mixed
alongside the audio tracks, at the same keyframed `volume` as anything else —
so muting a talking head under a voiceover is `volume: 0.0` on that clip. No
new field, no second concept, and no demuxing a file by hand to line its own
sound back up against its picture.

Whether a file has an audio stream is read from the asset's
`media.audio_channels`. An asset nobody has probed is **not** assumed to be
silent: a render probes what the project never recorded before it plans
anything, and if that probe fails, the clip is mixed without its own sound and
the report says which clip and why. Silence is never something a render
decided on its own without mentioning it.

A **hole in a track contributes nothing**, so the tracks below it show through.
Only a stretch with nothing on *any* video track renders black. That is the
difference between an empty patch of an overlay track and an empty timeline.

### How a source is fitted into the raster

A clip chooses this with `fit`, which is `fit` when absent:

| `fit` | what happens | for |
| --- | --- | --- |
| `fit` | scaled to sit **inside** the raster, keeping proportions; the leftover is **transparent** | the default — the whole shot, bars allowed |
| `fill` | scaled to **cover** the raster, keeping proportions; the overflow is cropped off the edges | a background plate that must not have bars |
| `native` | not scaled at all; the source arrives at its **own pixel size**, resting centred | a logo or badge at the size it was authored |

```json clip
{ "id": "c-logo", "asset": "logo", "start": 0, "duration": 60, "fit": "native" }
```

The leftover under `fit` is transparent rather than black. On the bottom track
the distinction is invisible, since the canvas beneath is black anyway. On an
upper track it is the whole point: a 4:3 clip over a 16:9 one shows the wider
clip at the sides rather than blacking it out. The same goes for the canvas
around a `native` layer — the tracks below show through it.

**Why `native` exists.** Under `fit`, a 64×64 logo in a 1920×1080 render
arrives 1080×1080 and has to be shrunk back with `transform.scale.x: 0.06`.
That number means nothing to a reader, it stops being right the moment the
render's resolution changes, and working it out means arithmetic against a
raster the project is not supposed to know about. `native` says "the logo, at
its size, moved here", which is what the author meant.

`native` rests the source **centred**, and `transform.position.*` offsets from
there. Centred, because the alternative — a corner — is an arbitrary edge of
that same raster. A source an odd number of pixels smaller than the raster
cannot sit exactly in the middle of it and is rounded to a whole pixel, since
half a pixel out would soften every edge in the layer. A source **larger** than
the raster is clipped by it rather than being shrunk: native means native.

`fit` is picture only. An audio clip has no raster, and a `fit` on one is
meaningless rather than invalid. Anchors other than the centre, per-clip crop
rectangles, and stretch-to-fill are not here: the last one is what
`transform.scale.*` already does for anyone who truly wants it.

A clip carries `start` and `duration` on the timeline, an optional
`source_in` offset into the media (default `0`), an optional `fit`
(default `fit`), and optional `keyframes`.
`source_in` counts in **timeline** frames too — "skip the first two seconds"
means the same thing whatever the source was shot at, and the conform rule
below turns it into a source frame.

**Times are whole frames on `timeline_fps`, not seconds.** At 30fps the clip
above runs frames 0–239 and covers the first eight seconds. A fractional or
negative time is not a time: it fails the load rather than being rounded into
place.

Clips on one track may *touch* but never overlap, and with integer frames that
is a fact rather than a tolerance. A clip ending at frame 240 and one starting
at 240 do not overlap: frame 240 belongs to the second, and nothing has to
arbitrate a cut at `1.0333333`.

A gap is allowed, and renders **black** for its length — or, on an audio
track, **silence**. Leaving a hole is a way of saying "two seconds of nothing
here", not a way of shortening the timeline. A timeline ends where its last
clip ends.

### How long a render is

**Picture decides.** A render's length is where the last video clip ends;
audio carrying on past it is cut there and reported. The thing being produced
is a video, and an edit ends when the last thing you can see ends — a music
bed left long is a bed left long, not a request for a longer film.

The other way round is simply silence: audio shorter than the picture leaves
the rest of the soundtrack empty, and the file still carries a sound stream.
A project with no audio clips at all is different again — that file has **no
audio stream**, which is not the same as a stream of silence.

Sample rate and audio bitrate are chosen per render, like resolution and
framerate, and default to 48 kHz. Sources of any rate are resampled on the way
in, so the mix only ever works in one.

## Keyframes

```json clip
{
  "id": "c-title", "asset": "title", "start": 0, "duration": 90,
  "keyframes": [
    { "property": "opacity", "keyframes": [
        { "t": 0, "value": 0.0, "easing": "ease_in" },
        { "t": 15, "value": 1.0 }
    ]}
  ]
}
```

A keyframe track is `(property_path, [(t, value, easing)])` over any numeric
property. `t` is in frames relative to the **start of the clip**, so moving a
clip never rewrites its keyframes. Times must ascend strictly.

Frames are enough resolution even for audio. Keyframes are *control points*
and the value travels continuously between them, so putting the points on the
frame grid does not make a ramp steppy — it only quantises where the ramp's
corners sit, to 1/30s, which is well below audible for a fade. `easing` is
`linear` (default), `ease_in`, `ease_out`, `ease_in_out`, or `hold`.

`property` is a dotted string — `opacity`, `transform.position.x`, `volume`
— and core does **not** check that it names a property that exists. That is
the generality rule: core defines property types, never property values. The
compositor resolves paths; adding a new animatable property costs nothing
here.

### Tracks a tool wrote

A keyframe track may carry an optional `by` naming the tool that generated it:

```json clip
{
  "id": "c-bed", "asset": "bed", "start": 0, "duration": 300,
  "keyframes": [
    { "property": "volume", "by": "duck", "keyframes": [
        { "t": 0, "value": 1.0 },
        { "t": 9, "value": 0.25, "easing": "ease_out" }
    ]}
  ]
}
```

**Absent means a person or an agent wrote it by hand**, which is what every
keyframe meant before the field existed.

It buys exactly one thing. A tool that generates keyframes may replace tracks
it signed, and must never touch a track that is unsigned or signed by somebody
else. So re-running auto-ducking redoes the ducking and leaves your fades
alone — without it, a generator can only clobber everything or refuse to run
twice, and both make the edit-and-listen loop unusable.

Nothing else reads it. It reaches no renderer, and changes no pixel and no
sample. A `by` that is present but blank is an error: it claims a tool wrote
the track and names none, so nothing could ever recognise or replace it.

**`duck` is the first tool that signs anything.** `scorsese duck --music <track>`
lowers a music clip's `volume` while narration plays over it, by writing exactly
the keyframes above. It triggers on the narration **clip's extents**, not on the
sound, which means it works on narration nobody has generated yet — a cut built
around a voice-over can be ducked, watched and judged before a word of it has
been paid for. A pause mid-sentence stays ducked; that is the cost of the
choice, and it is the right way round.

Where the four points land: full a moment before the narration starts, down by
the frame it starts, held until it ends, back to full a little after. Two lines
closer together than one recovery-plus-fall become a single dip, because coming
all the way up and immediately going back down is pumping — which draws more
attention than the ducking was avoiding.

### What the compositor animates today

| path | means | `1.0` / `0.0` |
| --- | --- | --- |
| `opacity` | how solid the layer is | `1.0` solid, `0.0` invisible |
| `transform.position.x` | offset right, in **output pixels** | `0.0` unmoved |
| `transform.position.y` | offset down, in output pixels | `0.0` unmoved |
| `transform.scale.x` | width multiplier about the layer's centre | `1.0` natural size |
| `transform.scale.y` | height multiplier about the layer's centre | `1.0` natural size |
| `transform.rotation` | turn about the layer's centre, in **degrees clockwise** | `0.0` upright |
| `volume` | how loud a clip plays, on either kind of track | `1.0` as recorded, `0.0` silent |

Scale and rotation are both **centre-anchored**, so shrinking a clip does not
also slide it into a corner and turning one does not swing it around an edge.
Rotation is in degrees and **positive turns clockwise** — nobody should have to
render a frame to find that out. The layer is scaled first and then turned,
both about its own centre; position is applied after both and measured in
output pixels, so it means the same thing whatever the source was shot at.

`volume` applies to any clip that makes a sound, which includes a clip on a
video track whose file has audio on it. It is a multiplier, so above `1.0` is
gain and below zero is nothing —
a negative multiplier is a phase inversion, which is not what dragging a
volume line past the floor means, so it is clamped away. **Muting a clip is
`volume` `0.0`**, not a flag: one keyframe holds for the whole clip, and the
thing that makes a clip silent is the same thing that fades it out.

Volume is evaluated **per sample**, travelling continuously between keyframes
rather than stepping once a frame — thirty steps a second is inaudible as
pitch but audible as a zipper. That is why frames are enough resolution for an
audio fade: they place the corners of the ramp, not the ramp itself.

A path nothing animates is **ignored** — not an error. A project authored
against a newer scorsese has to still render on an older one, so an unknown
property can never fail a render.

It is **warned** about, though, because the cost of ignoring it silently is
that a typo like `opactiy` does nothing at all: the keyframe track is valid,
the render succeeds, and the fade simply never happens. `scorsese check` and
every render's report name the clip and the property, and suggest the property
it was probably meant to be when there is an obvious candidate:

```
warning: clip `c1`: nothing animates `opactiy` — did you mean `opacity`?
```

A warning is all it is. It never fails a render, a check, or a merge — it
audits quality rather than proving correctness, and a hard error would make
every newly animatable property a breaking change for projects that already
use it. The list of what *is* animatable lives in the crate that implements
each property — the compositor for the visual ones, the mixer for `volume` —
so it cannot drift from the code, and core still knows nothing about which
properties exist.

Because `t` counts from the clip's start, a fade written once keeps working
after the clip is dragged elsewhere on the timeline. `scorsese-compositor`'s
`fade_in` and `fade_out` are sugar that write exactly these opacity keyframes
— there is no separate fade mechanism, which is why a fade composes with a
move or a zoom for free.

## Paths

Every path is relative to the project root and uses forward slashes on every
platform. Absolute paths (`/media/x.mp4`, `C:/media/x.mp4`, `\\host\share`),
backslashes, and `..` components are all rejected. This is what lets a
project survive `scp -r` between machines. The rule covers every path in the
document, not just `path`: a `style`'s font and a `synth_audio`'s `recipe`
obey it too.

A project directory holds four of its own:

| Directory | What is in it | Survives a delete? |
| --- | --- | --- |
| `assets/` | imported media, copied in on import | no — the originals are elsewhere |
| `generated/` | provider and synthesis output, named for the hash of its brief | yes — it can be made again |
| `recipes/` | authored synthesis documents | **no** — deleting one loses work |
| `cache/` | rebuildable scratch, gitignored | yes |

## Validation

**A save is atomic.** The document is written to a scratch file beside it and
renamed into place, so anything reading `project.json` at that moment gets the
whole of the old document or the whole of the new one and never a piece of
either. That matters because more than one program is looking: the MCP server's
`project_read` is stateless and can be called at any instant, the GUI writes
when a hand comes off a clip, and there is no journal to recover from — this
one file *is* the edit. Everything else written into a project goes the same
way, recipes and baked media included.

`Project::load` validates; `Project::save` does not, so an editor may save
work that is mid-edit and temporarily incoherent. Validation reports **every**
problem in one pass rather than stopping at the first, so an agent repairing
a project unattended sees the whole list at once.

What it checks: schema version, duplicate ids, path rules, hash shape, the
fields each asset kind requires — including that only a `text` asset carries
`text` or `style` and only a `color` asset carries `color`, that a `style`'s
font path and a `synth_audio`'s `recipe`
obey the project-path rules, and that each generated kind carries exactly the
brief it takes: a `prompt` or a `recipe`, never both and never the other's —
clip references resolving, asset kind against track kind, non-zero durations,
clip overlap, and keyframe shape.

Note what is *not* on that list. A time that is negative, fractional, or
infinite cannot be represented as a frame count, so it fails the parse with
the line it is on — earlier and more precisely than validation could say it.
The same goes for an unusable `timeline_fps`: there is nothing useful to
validate about a timeline whose grid is undefined.

Validation is about the *document*, and stops at the edge of it: it checks that
`path` is legal and relative, never that anything is there. Whether the media
actually exists is the pool's answer and `scorsese check`'s question — a
document can be flawless and still unrenderable because the footage was deleted
underneath it. `check` reports both together: an imported file a clip references and
cannot find is a problem, a file whose content no longer matches its recorded
`sha256` is a warning, a *generated* file that has gone is a warning too —
it renders as a slug card rather than stopping the render — and a `generated_*`
asset still awaiting generation is neither. A `style`'s font file is a path like any other, so the same split
applies: the shape is validated here, and whether the face is really on disk is
the render's to find out.

## Migrating from v6

v7 adds the `color` asset kind and the `color` field that goes with it. Both
are new: no v6 document can contain either, so **no v6 document means anything
different under v7**. Converting one is changing `"schema_version": 6` to
`"schema_version": 7` and nothing else.

## Migrating from v5

v6 adds one optional field: `by` on a keyframe track. No v5 document can
contain it, and **absent means hand-written**, which is what every keyframe
track in every v5 project already is. So no v5 document means anything
different under v6 — converting one is changing `"schema_version": 5` to
`"schema_version": 7` and nothing else.

## Migrating from v4

v5 adds the `synth_audio` asset kind and the `recipe` field that goes with it.
Both are new: no v4 document can contain either, so **no v4 document means
anything different under v5**. Converting one is changing `"schema_version": 4`
to `"schema_version": 5` and nothing else.

A v5 project directory also has a `recipes/` directory, which `scorsese new`
creates. A converted v4 project does not have one until something writes a
recipe into it, and nothing needs it before then — an absent `recipes/` is not
a validation error, the same way an absent `generated/` never was.

## Migrating from v3

v4 adds one optional field: `style` on a `text` asset. **Absent means every
default** — white, centred, sans, a tenth of the frame high — which is what
every text asset did before the field existed, so no v3 document means anything
different under v4. Converting one is changing `"schema_version": 3` to
`"schema_version": 4`, and then on to `7` as above.

Before v4 a text asset could not be rendered at all: the renderer refused a
clip showing one. So the only v3 documents affected are ones that were never
renderable, and there is nothing for a migration to preserve.

## Migrating from v2

v3 added one optional field: `fit` on a clip. **Absent means `fit`**, which is
what every clip did before the field existed, so no v2 document means anything
different under v3 — converting one is changing `"schema_version": 2` to
`"schema_version": 3` and then on to `7` as above.

The version still has to be changed by hand, because this build reads exactly
one schema version and refuses the rest. That refusal is the point: a document
that says `2` was written against a build that could not have meant anything by
`fit`, and inferring which of the two a file in front of us is would be
guessing rather than reading.

## Migrating from v1

v1 measured the timeline in float seconds and had no `timeline_fps`. It was
never shipped and no v1 project is known to exist, so **v2 ships no migration
code**: a v1 document is refused with "this build reads schema_version 2".

Converting one by hand, if one ever turns up, is two steps: pick the
framerate the edit was authored against and add it as `timeline_fps`, then
multiply every `start`, `duration`, `source_in`, and keyframe `t` by that rate
and round to the nearest whole frame. Rounding is the reason this is a manual
decision rather than an automatic one — it can move a cut by a frame, and only
the person who made the cut can say whether that matters.

A complete worked example lives in
`crates/core/tests/fixtures/narrated_teaser.json`.

## What CI checks about this page

This is the document an agent reads before writing a `project.json`, so it is
held to the code it describes rather than to anyone's memory:

- **Every animatable property is listed.** The compositor and the mixer each
  publish what they animate; a test asserts every published path appears in
  the table above, and that the table names nothing nobody animates. Adding a
  property without documenting it fails the build, and so does documenting one
  that does not exist.
- **Every example parses.** Each fenced `json` block carries a marker after
  the language, saying what it is a piece of; a test completes it into a whole
  document and parses that. An unmarked `json` block fails rather than quietly
  escaping the check, so a new example has to say what it is.

  | marker | the block is | checked by |
  | --- | --- | --- |
  | `project` | a whole `project.json` | parsing **and** validating it |
  | `fields` | top-level fields of the document | splicing them into a minimal document |
  | `asset` | one entry of `assets` | putting it in an otherwise empty project |
  | `track` | one entry of `tracks` | putting it in an otherwise empty project |
  | `clip` | one entry of a track's `clips` | putting it on an otherwise empty track |

  Fragments are parsed but not validated. A fragment may legitimately name an
  asset it does not carry, and failing it for that would be failing it for
  being a fragment.
- **Every command and flag in `scorsese --help` says something**, so the only
  interface an agent has today cannot grow a silent flag.

**What none of this proves.** A green CI run says every property is mentioned,
every example still parses, and every flag has help. It says nothing about
whether the sentence next to any of them is still *true* — that is the limit
of any documentation gate, and it is written here so a green check is never
mistaken for an accurate page. Prose that describes behaviour is still checked
by reading it.
