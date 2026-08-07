# `project.json` — schema v16

The contract between the CLI, the MCP server, the GUI, and every project
saved on someone's disk. It is meant to be hand-written: an agent should be
able to author a whole video in this file and render it without touching a
mouse.

Changing this format is `architecture` work — it needs a `schema_version`
bump and a migration note.

## The document

```json project
{
  "schema_version": 16,
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

`script` is the other optional top-level field, and it has a section of its
own below.

## The script, and notes

> **Neither the script nor any note ever renders.** Not a card, not a caption,
> not under any setting, in no version of this tool. That is an invariant of
> the format rather than a default someone could turn off, and it is stated
> this loudly because the format has precedent for text in a document reaching
> the screen: a `text` asset is drawn, and a sketch asset's `prompt` goes on a
> slug card. A note and a script are categorically unlike both. **Text that is
> meant to be seen is a `text` asset.** A test renders a project with a script
> and a note on every element, strips both, renders again, and fails on a
> single differing byte.

Everything else in this document says *what* the edit is. These two say
**why**, and without them every reason behind an edit lives outside the
project and dies with the conversation that produced it. The next person or
agent to open the file re-litigates all of it, and gets some of it wrong.

### `script` — one per project

```json fields
"script": "script.md"
```

The document the edit is being cut from: the brief, the outline, the list of
things the film must not claim. A **path**, relative to the project root like
every other, by convention `script.md` at the top of it.

A file rather than a string in the document, for the reason a `recipe` is one:
such a document is long. A real one measured 30 KB against a `project.json` of
34 KB — inlining it would very nearly double the document and bury the timeline
under prose, in the one file an agent opens to learn the edit. A file also gets
a readable diff and edits from any tool.

**It is never parsed.** No schema, no sections, no front matter, no convention
this tool enforces. The moment something started extracting meaning from it, it
would become a format with rules and stop being the place you write freely.
Markdown is convention only; nothing reads the extension.

A `script` naming a file that is **not there** is a warning, not a failure —
the same call a missing generated file gets, for the same reason: a project
that has lost its brief should still render. The path's *shape* is validated
like any other.

`scorsese new` does **not** leave a stub. An empty directory says "things go
here"; an empty file with the document pointing at it says the project carries
a script when it does not, and would make the missing-file warning fire for
every project that never had one. The MCP `script_write` tool creates the file
and sets the field in one call, which is what a stub was for.

### `note` — on anything with an `id`

```json asset
{ "id": "03-mosaic", "kind": "image", "path": "assets/03-mosaic.png",
  "note": "Stand-in footage: ocean container ships, not a barge convoy. Never describe these as the fleet's own cameras." }
```

Optional on an **asset**, a **track** and a **clip** — one uniform rule, and
keyframe tracks get none because they have no `id`, which falls out rather than
being decided. A free string; nothing reads it either.

Which of the three to write on follows from what the reason is *about*:

| on | what belongs there |
| --- | --- |
| asset | true of the file in every use of it — that this is a stand-in, that this is quoted copy |
| track | why the lane is here — why the music sits under the narration, why this is the one that gets ducked |
| clip | this shot's own choices — why it runs this long, why it was moved out of the full-bleed plate |

**A note attaches to an element and never to a span of time.** Most reasons are
not about time at all, and a timed note desynchronises silently the first time
the cut is retimed: "make it 20% faster" moves every clip and would leave every
timed note pointing at the wrong moment with nothing to notice. A note on a
clip moves with the clip for free.

**A note dies with its element, and that is the feature.** Deleting a clip
deletes the reasoning for a clip that no longer exists — nothing to garbage
collect, nothing left dangling. What has to survive a re-cut goes in the
script.

A note is not a `name`: a name is what a track is *called*, in a lane header.
It is not a `prompt` either — a prompt is handed to a provider and reaches the
screen on a card; a note is handed to nobody.

`scorsese describe` prints the script's path and every note, ahead of the cut
rather than after it.

## Assets

An asset is an entity. Clips point at assets **by id, never by path**, so
re-importing or regenerating a file is one edit in one place.

| Field | Required for | Meaning |
| --- | --- | --- |
| `id` | all | Unique within the project |
| `kind` | all | `video`, `image`, `audio`, `text`, `color`, `generated_video`, `generated_audio`, `synth_audio` |
| `path` | file-backed kinds | Relative to the project root |
| `sha256` | optional | 64 lowercase hex chars, of the file at `path` |
| `media` | optional | What ffprobe found: `duration_seconds`, `width`, `height`, `frame_rate` (a rational), `audio_channels`, `sample_rate` — see below |
| `prompt` | `generated_*` | What to generate, in words |
| `recipe` | `synth_audio` | Path to the document to synthesise from, by convention under `recipes/` |
| `state` | `generated_*`, `synth_audio` | `sketch`, `queued`, `generated`, `stale` |
| `text` | `text` | The string to render; text assets carry content inline and have no `path` |
| `style` | optional, `text` only | How that string looks: `font`, `weight`, `size`, `color`, `align`, `line_height`, `max_width` — see below |
| `color` | `color` | The colour to fill with, as `#rrggbb` or `#rrggbbaa`; colour assets have no `path` |
| `note` | optional | Why this asset is what it is. Never rendered — see above |
| `video` | optional, `generated_video` only | The rest of the brief: `model`, `resolution`, `seconds`, `aspect`, `first_image`, `last_image`, `reference_images` — see below |
| `speech` | optional, `generated_audio` only | The rest of the brief: `model`, `voice_id`, `language`, `seed` — see below |
| `created_at` | optional | When the asset joined the table, as UTC RFC 3339 (`2026-08-04T14:20:00Z`) |
| `queued_at` | optional, generated kinds | When a provider took the request. Not the same fact as `created_at` |
| `operation` | optional, `generated_video` | The provider's name for work in flight, while `queued` |
| `estimated_cost_cents` | optional, prompted kinds | What realising it was *calculated* to cost, in US cents — our arithmetic, never a bill. See [prices.md](prices.md) |

```json asset
{ "id": "shot-city", "kind": "generated_video", "state": "sketch",
  "prompt": "wide aerial of a city at dawn, slow push in" }
```

`media` is what something measured, never what anyone chose — so an asset you
write by hand leaves it out and has it filled in later. `scorsese probe` (and
`project_probe` over MCP) reads every asset that has a file and no `media`;
import does the same for what it brings in, and the window does it in the
background when it opens a project. Anything that needs a source's own length
reads `duration_seconds`, so an asset nobody has probed is one those features
skip — which is why `scorsese assets` counts it as needing attention.

A **still** is the one deliberate gap in that: its `media` carries a size and
never a `duration_seconds` or a `frame_rate`. ffprobe calls a still a one-frame
video and invents both, and how long a still is on screen is the clip's
business, not the file's.

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

### What a generated video asks for

A prompt says what the shot is *of*. `video` says what the shot **is** — and
every field in it changes the video that comes back, so all of them are hashed
into the brief along with the sentence, and editing any one of them makes the
asset `stale` exactly as rewording the prompt does.

```json asset
{ "id": "shot-hero", "kind": "generated_video", "state": "sketch",
  "prompt": "she turns to face the camera as the rain starts",
  "video": { "model": "fast", "resolution": "1080p", "seconds": 8,
             "aspect": "16:9", "first_image": "still-doorway",
             "reference_images": ["face-front", "face-side"] } }
```

| Field | Values | Default |
| --- | --- | --- |
| `model` | `fast`, `lite` | `fast` |
| `resolution` | `720p`, `1080p` | `1080p` |
| `seconds` | `4`, `6`, `8` | `8` |
| `aspect` | `16:9`, `9:16` | `16:9` |
| `first_image` | an `image` asset id | — |
| `last_image` | an `image` asset id | — |
| `reference_images` | up to 3 `image` asset ids | — |

Only `prompt` is required. Every field above has a default, so an absent
`video` and an empty one mean the same thing: a request made of a sentence and
nothing else.

The fields are the part that can be tabulated; the sentence is not. **What
certain words do to the shot that comes back is [`prompts.md`](prompts.md)** —
provider behaviour that cost a generation to find out, which is the half of
this brief no schema can describe.

**Stills are named by asset id, never by path** — the same rule a clip follows.
A path here would be a second way to point at media, and an absolute one would
break the promise that a project survives being copied to another machine. It
also means the brief hash can cover a still's `sha256`, so swapping the file
behind an id regenerates rather than quietly serving the old video.

The three image slots ask for different things. `first_image` is the frame the
shot opens on. `first_image` with `last_image` asks for the journey between two
stills — so a `last_image` alone is refused, because on its own it is not a
smaller version of that request. `reference_images` are pictures of a subject
that should go on looking like itself from shot to shot.

#### The combinations that are refused

These fields are not independent, and `scorsese check` reports every conflict
before anything is submitted — a request that could never succeed should cost a
message, not a round trip:

| Refused | Because |
| --- | --- |
| `seconds` other than `8` at `1080p` | that raster is only generated at eight seconds |
| `seconds` other than `8` with `reference_images` | likewise |
| `seconds` other than `8` with a first **and** last image | likewise |
| `reference_images` on `lite` | that tier does not take them |
| more than 3 `reference_images` | the provider accepts three |
| `last_image` without `first_image` | there is no journey from nowhere |
| a still that is not an `image` asset, or not in the table at all | every still handed over is a picture |

The first three are one rule wearing three hats, and the message always names
the *other* choice — the one that fixed the length — because that is the one
worth reconsidering. Dropping to `720p` costs less than giving up the stills a
shot is built from.

Switching a shot to `lite` to save money is the case to watch: it is the one
change that can invalidate a brief rather than merely cheapen it, which is why
it is refused rather than honoured with the images dropped.

### What a spoken line asks for

A prompt says the words. `speech` says how they are said — which voice, which
model, and whether the language is pinned or left to the model to infer.

```json asset
{ "id": "vo-open", "kind": "generated_audio", "state": "sketch",
  "prompt": "In nineteen seventy-six, nobody had seen a film like this.",
  "speech": { "model": "expressive", "voice_id": "EXAMPLEvoiceID012345",
              "language": "en", "seed": 42 } }
```

Like `video`, every field here is hashed into the brief along with the prompt,
so editing one makes the asset `stale` exactly as rewording the sentence does.

| Field | Default | Meaning |
| --- | --- | --- |
| `model` | `standard` | `expressive`, `standard` or `fast` — see below |
| `voice_id` | *none, and no default is possible* | The vendor's id for the voice |
| `language` | absent | ISO 639-1 (`en`, `pt`), pinning what the model would otherwise infer |
| `seed` | absent | For a reading that comes back the same way twice. Best-effort at the vendor, so worth recording and not worth relying on |

Three models, named for what the choice is *about* rather than for the
vendor's version strings — a person picks between expression and price, not
between `v3` and `v2_5`:

| `model` | On the wire | Price | For |
| --- | --- | --- | --- |
| `expressive` | `eleven_v3` | 10¢ / 1000 characters | the most expressive reading |
| `standard` | `eleven_multilingual_v2` | 10¢ / 1000 characters | the default, and the vendor's own |
| `fast` | `eleven_flash_v2_5` | 5¢ / 1000 characters | half the price, for narration where the reading matters less than the money |

The vendor also publishes Turbo variants, and its own documentation recommends
Flash over Turbo in every case — so offering both would be offering a choice
with a right answer.

**There is no default voice, and there cannot be one.** Every one of the
vendor's Default voices expires on 2026-12-31 and the set is being replaced
before then, so a fallback written into this format would be a guaranteed
outage with a date on it. A narration with no `voice_id` is still a legitimate
document; it is refused at the moment of spending, the way a shot with no
prompt is.

The id above is illustrative and is not a voice. Real ones are resolved from
the provider at runtime — which is the same reason none is written here as a
default.

#### The combinations that are refused

| Refused | Because |
| --- | --- |
| `language` on the `standard` model | that model **silently ignores** the field |
| a `prompt` over 40,000 characters | the vendor speaks at most that many in one request |
| an empty `voice_id` | it looks chosen and is not — leave it out instead |

The first is the one worth reading twice, because it is the only refusal in
this document that rejects a request the vendor would have **accepted**. The
API takes it, charges for it, and drops the field; the narration comes back
read in whatever language the model guessed. That failure has no symptom
except somebody listening to it, so the document is the only place it can be
caught at all.

Characters are counted as characters and not as bytes. Portuguese is one of
the two languages this is built for, and its accented letters are two bytes
each — counting bytes would refuse a legal script a quarter short of the
limit, and would have priced it wrong as well.

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
| `weight` | *none* | How heavy, 1–1000 — **required** for a variable font, refused for anything else |
| `size` | `0.1` | Em size as a fraction of the frame's **height** |
| `color` | `#ffffff` | `#rrggbb`, or `#rrggbbaa` for text you can see through |
| `align` | `center` | `left`, `center`, `right` — within the wrapped block |
| `line_height` | `1.25` | Baseline to baseline, as a multiple of `size` |
| `max_width` | `0.9` | Where lines wrap, as a fraction of the frame's **width** |

**Measurements are fractions of the raster, not pixels.** Resolution is a
render setting — the same project is previewed at 640×360 and delivered at 4K —
so a title written as `72` pixels would be a different title in each.
`size: 0.1` is a tenth of the picture's height whatever it is rendered at, and
the rule is the format's rather than this table's: `max_width` and
`transform.position.*` are fractions for the same reason.

**Two font names are reserved.** `sans` and `serif` are the faces scorsese
ships: Liberation Sans and Liberation Serif, metric-compatible with Arial and
Times New Roman, under the SIL Open Font License. Anything else in `font` is
read as a path to a font file the project carries, relative to the project root
like every other path — `assets/Inter-Regular.ttf`. The fonts are committed to
the repository rather than looked up on the system, because a system lookup
resolves differently on every platform and text has to render identically
everywhere.

#### Weight, and the variable font that would otherwise render hairline

```json asset
{ "id": "title", "kind": "text", "text": "Chapter One",
  "style": { "font": "assets/Manrope[wght].ttf", "weight": 700, "size": 0.12 } }
```

**Most modern open fonts ship only as variable files**, and that fact is the
reason this field exists. Google Fonts serves `Manrope[wght].ttf`,
`Outfit[wght].ttf` and `PlusJakartaSans[wght].ttf` and nothing else for those
families: one file holding a continuous range of weights, plus a *default
instance* the designer picked. That default is very often not Regular.

| family | its own default | what "no weight" would give you |
| --- | --- | --- |
| Manrope | 200 | ExtraLight |
| Outfit | 100 | Thin |
| Plus Jakarta Sans | 400 | Regular |

So **a variable font with no `weight` is refused**, by name, saying the file is
variable and what range its axis covers. There is no fallback to 400 and no
falling back to the file's own default, for the same reason a `color` asset has
no default colour: a title card set in hairline Thin at a tenth of the frame is
a shot rendered wrong, and a document that produced it would look entirely
correct. The rule this is holding is that **a project which renders wrong must
not render silently**.

The other three cases follow from the same rule:

- **A weight the file's axis does not reach is refused**, with the range it
  does reach. Manrope stops at 800, so `900` is an error rather than a quiet
  clamp — clamping is the same silent substitution one step along.
- **A weight on a static font is refused.** A file with no `wght` axis has one
  weight; silently ignoring a field you wrote is how you come to insist your
  bold is broken.
- **A weight on `sans` or `serif` is refused.** The shipped faces are one
  static weight each. This one is caught by validation rather than at the
  render, because it needs no file to know.

`weight` is the *only* axis read. Optical size, width and slant are real axes
and none of them is what "make this bold" means.

**One file, every weight**, which is the other half of what this buys. A
project wanting a bold title and a regular caption points both at the same
`.ttf` and names 700 and 400 — no second copy of the family, and no instancing
a static face with `fonttools` outside the project first. Weights the designer
never drew work too: `500` is a position on the axis, not the nearer of two
neighbours.

The text is laid out centred on the frame, wrapped to `max_width`, and
truncated with an ellipsis if it is taller than the picture. **Moving it is
`transform.position.x` and `transform.position.y`**, and fading it is
`opacity` — the same properties that move and fade a video clip, keyframes and
all. Text has no animatable properties of its own, which is why a title that
slides and fades needs nothing here. `fit` is meaningless on a text clip:
there is no source raster to reconcile, since the text is drawn at whatever
size the render is.

Italic, per-character animation, outlines and shadows are not here yet. Bold is
`weight` on a variable font, and nothing more than that: there is no `bold`
flag, because a flag would be a second, coarser way to say a number that
already exists.

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

Both a track and a clip take an optional `note` — see above. A track's `name`
is cosmetic and is what the lane is called; its `note` is why it is there.

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
meaningless rather than invalid. Anchors other than the centre and
stretch-to-fill are not here: the last one is what `transform.scale.*` already
does for anyone who truly wants it.

### Showing part of a source: `crop`

```json clip
{ "id": "c-panel", "asset": "screenshot", "start": 0, "duration": 120,
  "crop": { "x": 0.158, "y": 0.079, "width": 0.842, "height": 0.921 } }
```

A rectangle of the source, **in fractions of it**, aligned to the source's own
axes. Absent means the whole thing, the way an absent `fit` means the ordinary
case. All four fields are required when the rectangle is there: a partial one
is a rectangle whose other edges nobody stated.

**The asset is never touched**, and that is the point rather than a detail.
Cropping by cutting the file down is the one place this format's premise — a
document describing an edit over unmodified assets — has to be broken to do
ordinary work. Do it and the original pixels are gone, so "show more of the
map" means going back to the machine the screenshot came from; nothing in
`project.json` records that a sidebar was removed, or from where; and `sha256`
and `media` describe a file no camera and no capture ever produced. As a clip
property it is an edit you can change your mind about, like every other one.

**Fractions, not source pixels**, and the reasoning is worth more than the
choice. A fraction survives the asset being *replaced* by a higher-resolution
capture of the same thing: re-shoot the screenshot at 4K and the crop still
means the same region, where in pixels it would silently mean a different one —
and changing the crop later is exactly what the field is for. A fraction is
also checkable from the document alone, where a pixel rectangle would need the
source's dimensions, which are recorded only if something probed the asset.

**Crop happens before fit.** The order is `source → crop → fit into the raster
→ transform → composite`, and it is the only one that makes sense: cropping
after the fit would be cropping the *output*, which is a matte and a different
feature. So after a crop it is the **cropped rectangle** that `fit`, `fill` and
`native` reconcile against the raster — a crop that changes the aspect
therefore changes what `fit` does, and a cropped `native` layer is the cropped
pixels at their own size.

A rectangle that runs off an edge of the source, or encloses none of it, is a
validation error naming the clip and the edges as they are written.

This is a different question from `transform.position`, which is a fraction of
the **output** raster. A crop is against the **source** raster, and the two do
not have to answer the same way.

**The four numbers can be read off the footage rather than guessed at.**
`scorsese look <file> --grid` — or the `look` tool with `grid: true` — rules
every sampled frame with a line at each `0.1` of the source, which is precisely
the unit here. `scorsese still --grid` rules a composited frame the same way, in
the output raster's fractions, for `transform.position`.

**Not a mask, not a shape, not rotation-aware, not per-corner**, and not
animatable: the crop is applied as the source is decoded, ahead of the fit, so
it is one rectangle for the clip rather than a value the compositor resolves
per frame. Animating it would mean moving the fit into the compositor, which is
its own piece of work.

### Which edge a position is measured from: `anchor`

```json clip
{ "id": "c-title", "asset": "title", "start": 0, "duration": 112,
  "anchor": { "x": "left", "y": "center" },
  "keyframes": [ { "property": "transform.position.x",
                   "keyframes": [ { "t": 0, "value": 0.05 } ] } ] }
```

`x` is `left`, `center` or `right`; `y` is `top`, `center` or `bottom`. Absent
means centred on both, which is what every layer did before the field existed.

**Without it the format can say where a layer ended up and not what was meant.**
A title column beside a picture — the commonest arrangement there is — comes out
as an offset derived on paper from the block's width and the fact that text is
drawn centred. Nobody can read that back out; putting the text on the other side
is a recomputation rather than one word; and lengthening the title moves it,
because a centred block grows both ways. With an anchor the same layout is
`left` and a margin, and flipping it is `left` → `right` with the number
unchanged.

**A positive offset always moves the layer further in.** On `right` and
`bottom` the offset is measured inward from that edge, so the same number is the
same margin whichever edge it was measured from — which is what makes flipping a
layout one word rather than a sign change.

**What it anchors is the layer's laid-out rectangle**, and for text that is the
**wrapped block at `max_width`**, not the longest line. Left-anchored text could
plausibly mean either, and only the block keeps a column's left edge still when
the wording changes. `align` still places each line inside that block.

**A `fit` or `fill` layer is unaffected**, and that falls out rather than being
a special case: such a layer *is* the raster, so every edge already meets the
matching one. An anchor moves a `native` layer, a cropped one, and a text block.

**It is a field, not an animatable property**, and deliberately: an anchor says
how a coordinate is to be *read*, and animating that would slide a layer by
changing what its number means. `transform.position` stays the animated part.
Scale and rotation stay centre-anchored about the layer's own middle — the
anchor decides where the layer sits, and those decide what it does about its
own centre once it is there.

### Playing faster or slower: `speed`

```json clip
{ "id": "c-timelapse", "asset": "roof", "start": 0, "duration": 120, "speed": 2.0 }
```

How fast the source runs against the timeline. `1.0` when absent — one source
frame per timeline frame, which is what every clip did before there was a rate
to choose. Validated positive and finite.

**This is what breaks the one-to-one relationship `source_in` and `duration`
used to have.** A clip of 60 timeline frames at `2.0` consumes 120 frames of its
source; one at `0.5` consumes 30. `duration` is still a length of **timeline**,
so speeding a clip up without also shortening it shows more of the source in the
same slot.

That split is deliberate. What an editor's 2× button does — halve the clip's
length so it covers the same footage — is an **operation** a GUI or an assistant
performs, rescaling `duration` as it sets `speed`. The document stays
declarative, which is what lets a clip be sped up *and* trimmed without the two
fighting.

**Speed changes pitch.** Playing a clip at 2× resamples its audio and it rises,
which is what an editor expects from a speed control and what Filmora does.
Preserving pitch is a time-stretch algorithm and a new dependency; it is
deferred rather than assumed.

**A clip has one speed, for picture and sound alike.** Separating them is a
compositing-suite answer to a question nobody editing a video asks.

**Keyframes stay in timeline time.** A keyframe's `t` is where it sits on the
timeline, and a speed change does not move it — a fade written half a second
into a clip is half a second into that clip at any speed. Pinning keyframes to
source frames would shift every animation whenever a clip was retimed, which is
the opposite of what an author means by *the fade goes here*.

Speeding a clip up consumes more source in the same slot, so it can run off the
end of its footage exactly as an over-long trim can — same ceiling, same answer.

**Not here:** speed *ramps* (a keyframed speed, where the mapping from timeline
frame to source frame stops being multiplication and becomes an integral), and
negative speed, which needs the decoder to seek rather than stream.

A clip carries `start` and `duration` on the timeline, an optional
`source_in` offset into the media (default `0`), an optional `speed`
(default `1.0`), an optional `fit`
(default `fit`), an optional `crop` (default the whole source), an optional
`anchor` (default centred), and optional `keyframes`.
`source_in` counts in **timeline** frames too — "skip the first two seconds"
means the same thing whatever the source was shot at, and the conform rule
below turns it into a source frame.

**Times are whole frames on `timeline_fps`, not seconds.** At 30fps the clip
above runs frames 0–239 and covers the first eight seconds. A fractional or
negative time is not a time: it fails the load rather than being rounded into
place.

**A clip may not show source that is not there.** `source_in` plus how much of
the media the clip plays through has to land inside the asset's own length: an
edit goes on for as long as there is footage, and no more. Both edges are
bounded by the same fact — `source_in` cannot go below zero because a file has
nothing before its head, and the tail cannot pass `media.duration_seconds`
because it has nothing after its end.

The bound is only as good as what has been *measured*, and that is deliberate.
An asset carrying a `duration_seconds` bounds every clip that shows it. One
without bounds nothing, which covers a still, a title and a colour — each held
on screen for as long as you like, none with a length of its own — a sketch,
whose file does not exist yet, and a file nobody has probed. The last of those
is what `scorsese probe` is for: a ceiling that applied to half the pool would
be worse than none, because the half it skipped would be invisible to whoever
was trimming.

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
| `transform.position.x` | offset right, as a fraction of the raster's **width** | `0.0` unmoved |
| `transform.position.y` | offset down, as a fraction of the raster's **height** | `0.0` unmoved |
| `transform.scale.x` | width multiplier about the layer's centre | `1.0` natural size |
| `transform.scale.y` | height multiplier about the layer's centre | `1.0` natural size |
| `transform.rotation` | turn about the layer's centre, in **degrees clockwise** | `0.0` upright |
| `transform.flip.x` | turn about the layer's own **horizontal** axis, in degrees | `0.0` face on |
| `transform.flip.y` | turn about the layer's own **vertical** axis, in degrees | `0.0` face on |
| `volume` | how loud a clip plays, on either kind of track | `1.0` as recorded, `0.0` silent |

Scale, rotation and flip are all **centre-anchored**, so shrinking a clip does
not also slide it into a corner and turning one does not swing it around an
edge. Rotation is in degrees and **positive turns clockwise** — nobody should
have to render a frame to find that out. The layer is scaled first and then
turned, both about its own centre; position is applied after both.

**A flip turns the layer over.** `transform.flip.y` turns it about its
**vertical** axis — the page-turn, one side edge swinging toward you — and
`transform.flip.x` turns it about its **horizontal** one. Both in degrees, both
`0` by default: `0°` is face on, `90°` is edge on and therefore **invisible**
(the layer is skipped, not drawn as a line of smeared colour down the middle of
the frame), and `180°` is face on again and **mirrored**, which is what the back
of a card looks like.

`flip.x` and `flip.y` name the **axis turned about**, not the direction the
picture appears to move — so `transform.flip.y`, about the *vertical* axis, is
the left-right page-turn that some people would call "flipping horizontally".
That is the convention `transform.rotation` already uses, and it is written down
here rather than left to be discovered by rendering a frame.

Two things fall out of it, and neither is a feature of its own. A **mirrored
layer** is a static `transform.flip.y` of `180` — one keyframe, held. And a
**flip that swaps the picture halfway** is two ordinary clips: clip A animates
`0° → 90°`, clip B picks up at `90°` and carries on to `180°`, and the swap
happens on the frame where nothing is visible. Nothing owns both halves, so
every partial flip and every mid-flip substitution comes out of the same
property.

The flip is **flat, not perspective**: the layer squashes along the axis by
`cos θ`, and the near edge does not grow as it swings toward you. A true
projective warp is not an affine transform, and the flat version reads correctly
as a card turning.

**Position is a fraction of the raster**, for the same reason `size` is: a
title placed `110` pixels above centre is a tenth of the height at 1080 and a
twentieth at 4K, so a layout composed in a preview would not survive delivery.
`transform.position.x` of `0.25` moves the layer a quarter of the raster's
**width** to the right of where it naturally sits; `transform.position.y` of
`0.25` moves it a quarter of its **height** down. Each axis against its own
dimension, so `0.5` in x reaches the edge of the frame whatever its shape —
resolving both against the height would keep a diagonal's angle across aspect
ratios and cost the plain reading that placement actually wants.

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
document, not just `path`: a `style`'s font, a `synth_audio`'s `recipe` and the
document's own `script` obey it too.

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
font path, a `synth_audio`'s `recipe` and the document's `script`
obey the project-path rules, and that each generated kind carries exactly the
brief it takes: a `prompt` or a `recipe`, never both and never the other's —
clip references resolving, asset kind against track kind, non-zero durations,
clip overlap, no clip reaching past the end of the source it was measured to
have, and keyframe shape.

Note what is *not* on that list. A time that is negative, fractional, or
infinite cannot be represented as a frame count, so it fails the parse with
the line it is on — earlier and more precisely than validation could say it.
The same goes for an unusable `timeline_fps`: there is nothing useful to
validate about a timeline whose grid is undefined.

Validation is about the *document*, and stops at the edge of it: it checks that
`path` is legal and relative, never that anything is there. The clip-length
ceiling is not an exception to that, and it is worth saying why: it reads the
`media` block the document already carries, written there by whoever probed the
asset. Validation never opens a file to find out how long it is — that is why
an unprobed asset bounds nothing, and why the answer to a ceiling that feels
missing is `scorsese probe` rather than a slower load. Whether the media
actually exists is the pool's answer and `scorsese check`'s question — a
document can be flawless and still unrenderable because the footage was deleted
underneath it. `check` reports both together: an imported file a clip references and
cannot find is a problem, a file whose content no longer matches its recorded
`sha256` is a warning, a *generated* file that has gone is a warning too —
it renders as a slug card rather than stopping the render — and a `generated_*`
asset still awaiting generation is neither. A `style`'s font file is a path like any other, so the same split
applies: the shape is validated here, and whether the face is really on disk is
the render's to find out. So is the `script`, and its missing file is a warning:
a project that has lost its brief still renders.

`style.weight` splits the same way, and the split is worth reading once. What
the document alone can say is checked here: `sans` and `serif` are faces
scorsese ships and knows to be static, so a weight beside one is refused
without opening anything, and a number outside OpenType's own 1–1000 is not a
weight for any face. What only the *file* can answer — whether it is variable
at all, and how far its `wght` axis runs — is refused at the render, in the
same breath as "this is not a font I can read".

## Compatibility

`schema_version` is read before anything else, so a document from a different
build is refused on the number rather than on a field it does not recognise —
"upgrade scorsese" being the useful thing to say about a document from a newer
one.

No conversion notes are kept here. The format is pre-1.0 and still moving.

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
