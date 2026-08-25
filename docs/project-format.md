# `project.json` — schema v29

The contract between the CLI, the MCP server and the GUI — the contract *now*,
not across time. It is meant to be hand-written: an agent should be able to
author a whole video in this file and render it without touching a mouse.

Changing this format is `architecture` work — it needs a `schema_version`
bump. It does not need a migration note, and there are none here: **nothing is
kept working for the sake of a project saved by an older build.** The bump is
what makes a break honest rather than what softens it — a document whose
version is not this build's is refused on sight instead of being read as
something it no longer means.

A complete worked example lives in
`crates/core/tests/fixtures/narrated_teaser.json`.

## The document

```json project
{
  "schema_version": 29,
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
| `kind` | all | `video`, `image`, `audio`, `text`, `color`, `shape`, `icon`, `generated_video`, `generated_audio`, `synth_audio` |
| `path` | file-backed kinds | Relative to the project root |
| `sha256` | optional | 64 lowercase hex chars, of the file at `path` |
| `media` | optional | What ffprobe found: `duration_seconds`, `width`, `height`, `frame_rate` (a rational), `has_alpha`, `audio_channels`, `sample_rate` — see below |
| `prompt` | `generated_*` | What to generate, in words |
| `recipe` | `synth_audio` | Path to the document to synthesise from, by convention under `recipes/` |
| `state` | `generated_*`, `synth_audio` | `sketch`, `queued`, `generated`, `stale` |
| `text` | `text` | The string to render; text assets carry content inline and have no `path` |
| `style` | optional, `text` only | How that string looks: `font`, `weight`, `size`, `color`, `align`, `line_height`, `max_width`, `stroke` — see below |
| `color` | `color` | The colour to fill with, as `#rrggbb` or `#rrggbbaa`; colour assets have no `path` |
| `shape` | `shape` | The outline to draw and how it is coloured — see below |
| `icon` | `icon` | Which symbol to draw, how big and in what colour — see below |
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

`has_alpha` is the one `media` field a render acts on for *picture* rather than
sound, and it says whether the file's pixel format carries transparency. A
source with alpha has to be premultiplied before it is scaled and
unpremultiplied afterwards, or the black that exporters leave behind fully
transparent pixels gets averaged into the opaque ones beside them and every
scaled edge comes back with a dark rim — the thing a transparent logo is
brought into an edit to avoid. Absent means nobody has looked, and a render
that has not been told treats the source as opaque: it scales exactly as it did
before the field existed. A palette image is recorded as having alpha, because
its transparency lives in the palette where the pixel format cannot show it.

A **still** is the one deliberate gap in that: its `media` carries a size and
never a `duration_seconds` or a `frame_rate`. ffprobe calls a still a one-frame
video and invents both, and how long a still is on screen is the clip's
business, not the file's.

An `image` may carry an **animation** — a gif or an avif with more than one
frame in it. It is still an image and still held for as long as its clip says:
the picture it holds moves, and when the clip outlasts the animation it starts
again from the top. That follows from the gap above rather than working around
it — the file has no say in how long it is on screen, so filling the clip means
looping. An animated **webp** is the exception, and it is ffmpeg's rather than
ours: there is no decoder for the animation, so the file measures `0x0` and is
refused at import instead of reaching a render nothing can draw.

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
| `model` | `fast` | `expressive`, `standard` or `fast` — see below |
| `voice_id` | *none, and no default is possible* | The vendor's id for the voice |
| `language` | absent | ISO 639-1 (`en`, `pt`), pinning what the model would otherwise infer |
| `seed` | absent | For a reading that comes back the same way twice. Best-effort at the vendor, so worth recording and not worth relying on |

Three models, named for what the choice is *about* rather than for the
vendor's version strings — a person picks between expression and price, not
between `v3` and `v2_5`:

| `model` | On the wire | Price | For |
| --- | --- | --- | --- |
| `expressive` | `eleven_v3` | 10¢ / 1000 characters | the most expressive reading |
| `standard` | `eleven_multilingual_v2` | 10¢ / 1000 characters | the vendor's own default, and the one model that ignores `language` |
| `fast` | `eleven_flash_v2_5` | 5¢ / 1000 characters | **the default** — half the price, and the vendor's pick over its Turbo variants |

**`fast` is the default, and the reason is the price.** `standard` and
`expressive` cost the same as each other, so the only choice the rate card
actually poses is *fast or not* — and a reading nobody configured should not
quietly be the dearer one. Name `expressive` when the reading matters; it costs
exactly what `standard` does, so there is never a money argument for `standard`
over it.

The vendor also publishes Turbo variants, and its own documentation recommends
Flash over Turbo in every case — so offering both would be offering a choice
with a right answer. That recommendation is Flash over *Turbo* and not Flash
over everything: ElevenLabs positions `eleven_v3` as the expressive one. The
argument for the default above is scorsese's, not theirs.

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
| `font` | `sans` | one of the eight shipped names, or a path to a font file inside the project |
| `weight` | *none* | How heavy, 1–1000 — **required** for a variable font, refused for anything else |
| `size` | `0.1` | Em size as a fraction of the frame's **height** |
| `color` | `#ffffff` | `#rrggbb`, or `#rrggbbaa` for text you can see through |
| `align` | `center` | `left`, `center`, `right` — within the wrapped block |
| `line_height` | `1.25` | Baseline to baseline, as a multiple of `size` |
| `max_width` | `0.9` | Where lines wrap, as a fraction of the frame's **width** |
| `stroke` | *none* | `#rrggbb` or `#rrggbbaa` — a rim round the glyphs; absent means no edge |
| `stroke_width` | `0.002` | How far that rim reaches outward, as a fraction of the frame's **height** |

**A newline in `text` is honoured and ordinary whitespace is not.** An author
who broke a title in two meant it, so `\n` starts a new line; runs of spaces
and tabs inside a line collapse to one, because that is what wrapped prose
wants and a stray double space is almost never deliberate.

**When it *is* deliberate, write a non-breaking space** — `U+00A0`, the
character that exists to opt out of exactly that. A run of them survives whole,
one at the start of a line indents it, and no line ever breaks at one. That is
what makes a column or an indent expressible: `size` and `max_width` say how
big the text is and where it wraps, and NBSP is the only thing that says where
the spacing inside a line goes. It pairs with `jetbrains-mono` below, which is
the face those columns are usually set in.

**Measurements are fractions of the raster, not pixels.** Resolution is a
render setting — the same project is previewed at 640×360 and delivered at 4K —
so a title written as `72` pixels would be a different title in each.
`size: 0.1` is a tenth of the picture's height whatever it is rendered at, and
the rule is the format's rather than this table's: `max_width` and
`transform.position.*` are fractions for the same reason.

**A bare word is a font scorsese ships; anything with a slash or a dot in it is
a font file the project carries** — `assets/Manrope[wght].ttf`, relative to the
project root like every other path. Eight families ship, all under the SIL Open
Font License:

| name | family | weights | for |
| --- | --- | --- | --- |
| `inter` | Inter | 100 – 900 | the default sans; a modern interface face |
| `source-serif` | Source Serif 4 | 200 – 900 | the default serif; readable at caption size |
| `liberation-sans` | Liberation Sans | 400, 700 | **the Arial look** |
| `liberation-serif` | Liberation Serif | 400, 700 | **the Times New Roman look** |
| `montserrat` | Montserrat | 100 – 900 | geometric, for titles |
| `lora` | Lora | 400 – 700 | a warm text serif |
| `playfair-display` | Playfair Display | 400 – 900 | high contrast, for a title card |
| `jetbrains-mono` | JetBrains Mono | 100 – 800 | monospace |

**`sans` and `serif` are aliases**, for `inter` and `source-serif`. They are what
every project written before this list existed says, and they go on meaning the
default sans and the default serif — which is the point of an alias: the thing
they point at can change without a document changing.

**Arial and Times New Roman themselves can never ship.** They are Monotype's and
cannot be committed to a public repository. `liberation-sans` and
`liberation-serif` are the open substitutes: metric-compatible, the same advance
widths, and to anyone who is not a typographer the same look.

The fonts are committed to the repository rather than looked up on the system,
because a system lookup resolves differently on every platform and text has to
render identically everywhere. Which files, from which release, with which
hashes, is `crates/compositor/fonts/README.md`.

**A name nothing ships is refused with the list**, at the render and by
`scorsese check`, rather than being read as a filename. So writing `"Arial"`
gets *"there is no font called `Arial`. The ones scorsese ships are: …"* rather
than a complaint about a missing file, which is the sentence somebody who typed
it actually needs.

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
- **A weight a shipped family does not reach is refused too**, at the render and
  not at validation. Inter starts at 100 and Source Serif 4 at 200, which is a
  fact about a file like any other.
- **A weight a *drawn* family was not drawn at is refused with the weights it
  has.** Liberation is four separate files rather than an axis — there is no
  variable build of it anywhere — so `liberation-sans` has 400 and 700 and
  nothing between. `600` is an error naming both, not a quiet 700: snapping is
  the same silent substitution as clamping, one step along. Every other shipped
  family is variable, where any weight inside the range is a real position on
  the axis.

**A shipped family defaults to weight 400; a font the project carries does
not.** Written
as its own rule rather than left to look like an exception to the one above,
because the two only appear to contradict each other until the reason is on the
page. The refusal protects against a file scorsese cannot know: Manrope defaults to
200 and would render hairline while the document looked entirely correct, and
nothing here has read that file. It knows the shipped ones — their axes are
published in `crates/compositor/fonts/README.md`. Montserrat's own default is
100, and shipping it is exactly why the rule is "400" rather than "whatever the
file says". That is the
same split the format already draws everywhere else, between what the
*document* can answer and what only opening the *file* can, and it lands on the
right side of it.

The rule is also what keeps every project ever written valid. A weight beside
`sans` used to be refused, so **no existing document carries one** — which means
every one of them relies on an unweighted shipped name going on meaning
Regular.

#### Italic

```json asset
{ "id": "aside", "kind": "text", "text": "later that year",
  "style": { "font": "liberation-serif", "italic": true, "size": 0.08 } }
```

**A boolean, not an angle**, because a real italic is a *different drawing*
rather than the upright leaned over — different letterforms, often a
single-storey `a` and an entirely redrawn `f`. A number would promise a
continuum between the two that does not exist.

Every shipped family carries its italic beside its upright, **keyed by weight
the same way**, so `italic` composes with `weight`: `liberation-serif` at `700`
with `italic: true` is the BoldItalic the designer drew, not a bold that has
been slanted.

Inter is the clearest case for why this is a second set of files rather than an
effect. Its variable file has a `slnt` axis, which produces an **oblique** — the
upright leaned over — and it also ships a separate italic where the letters are
actually redrawn. `italic: true` reaches the second, every time.

**`italic` on a font the project carries is refused.** That file is one drawing
and has no second table to reach for, so the way to get an italic there is to
name the italic file: `"font": "assets/Manrope-Italic[wght].ttf"`. Same rule as
a weight beside a static file, and for the same reason — a field nobody reads is
how somebody comes to insist their italic is broken.

A family with no italic drawn would refuse one rather than shear its upright.
None of the eight is that today; the rule is written down because the shape
allows it and a future family might be.

#### The stroke, which is what makes a caption survive the shot behind it

```json asset
{ "id": "leg-01", "kind": "text", "text": "the last lap",
  "style": { "font": "playfair-display", "weight": 600, "size": 0.040,
             "color": "#ffffff", "stroke": "#050b12ff", "stroke_width": 0.0022 } }
```

A caption burned into the picture is the main way a video reaches somebody
scrolling with the sound off, and white letters over a bright frame are
letters nobody can read. `stroke` is the edge that fixes it: the same
notation as `color` and as a shape's `fill` — `#rrggbb`, or `#rrggbbaa` for
one you can see through — and **absent means no edge**, which is what every
document that says nothing about it means.

`stroke_width` is a fraction of the raster's **height**, the unit `size` and a
shape's `stroke_width` already use, because a thickness has no axis of its own
and one chosen unit is easier to remember than three. It defaults to `0.002`,
about two pixels at 1080p. One number means one thickness in every direction
and at every aspect ratio — which offsetting a second copy of the words could
never manage, since a fraction of each axis is 1.4× as far on the diagonals and
a different thickness the moment the render's shape changes.

**The rim is drawn behind the fill, and grows outward only.** This is the one
place `text` and `shape` share a field name and not a geometry, and it is
deliberate: a shape's border straddles its outline so that the shape's stated
size stays the size of the shape, while half a width eating *inward* on a
letter is exactly the half a caption cannot spare. It closes the eye of an `e`
and the bowl of an `a` at the sizes captions are set at, and the failure is
invisible in the document — it shows up as mush on a finished video. So the
glyph is drawn whole on top of its own rim and keeps its shape; what the two
kinds share is the field names and the unit.

**A `stroke` with a width of zero is refused**, the same way a shape's border
without one is. *I meant no edge* and *I meant an edge and got the width
wrong* look identical in the frame, and only one of them is what the document
says. A `stroke_width` with no `stroke` beside it is not refused — it is a
number nothing reads, which is what it is on every text asset ever written.

#### The axes, and the one that is read

`weight` is the *only* axis read. Optical size, width and slant are real axes
and none of them is what "make this bold" means — slant least of all, which is
exactly why `italic` reaches a drawn file instead of that axis. Which axes the shipped faces
carry, and what each is therefore left at, is recorded in
`crates/compositor/fonts/README.md` — Source Serif 4's `opsz` sits at its
text-size default, and that is a stated consequence rather than an oversight.

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

Per-character animation and shadows are not here yet. Bold is
`weight` on a variable font, and nothing more than that: there is no `bold`
flag, because a flag would be a second, coarser way to say a number that
already exists.

### Colour assets

```json asset
{ "id": "black", "kind": "color", "color": "#000000" }
```

A `color` asset is the simplest of the kinds with no file behind it: it has no
content at all, only appearance. It is a background, a
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

### Shape assets

```json asset
{ "id": "box-a", "kind": "shape",
  "shape": {
    "geometry": { "rectangle": { "width": 0.24, "height": 0.12, "radius": 0.1 } },
    "fill": "#1e3a8aff",
    "stroke": "#000000ff",
    "stroke_width": 0.004
  } }
```

The third kind with no file behind it, and the one that draws something with an
edge: a box or an ellipse, for the diagrams, callouts and legends a video is
explained with. The alternative is authoring a PNG with alpha in another
program, importing a megabyte of it, and finding it soft the first time the
render resolution changes — which is the argument the `color` asset already
won.

`shape` is required on this kind and refused on every other. Inside it,
`geometry` is required and is exactly one of:

| Outline | Fields | What it is |
| --- | --- | --- |
| `rectangle` | `width`, `height`, optional `radius` | four corners, square or rounded |
| `ellipse` | `width`, `height` | the ellipse inscribed in that box |
| `arrow` | `from`, `to`, optional `curve`, optional `heads` | a line between two points, with a head on it |

**Everything is a fraction of the raster**, as `transform.position` is:
`width` of the frame's width, `height` of its height. So a shape is drawn at
whatever resolution the render turns out to be, with a clean edge at every one
of them, and there is no size in the document to be wrong at 4K.

That also means `ellipse` rather than `circle` is the primitive. A circle on a
16:9 frame is an ellipse whose two numbers account for the aspect —
`0.1 × 0.178` — and naming it `circle` while quietly measuring both against one
axis would be a different bug on every aspect ratio.

`radius` is the exception, and deliberately: it is a fraction of the **shape's
own shorter side**, not of the frame. `0` is a square corner and `0.5` is a
pill, whatever size the box is. Two reasons. It keeps corners circular rather
than elliptical — one number meaning one distance — and it is checkable from
the document alone, where a rounding larger than the box it rounds could only
be caught once a render had turned both into pixels. Above `0.5` is refused.

**`fill` and `stroke` are separate, and each is optional.** Both take the same
notation a text `style` does: `#rrggbb`, or `#rrggbbaa` for one you can see
through. A border over an absent fill is a callout that does not hide the shot
inside it; a fill with no border is a plain block; a green border round a blue
interior is a legend key. What is refused is *neither* — a shape that would
draw nothing renders exactly like a shape that failed to render, and a diagram
quietly missing a box is the kind of mistake that reaches a published video.

`stroke_width` is a fraction of the raster's **height** — the unit a text
`size` already uses, because a thickness has no axis of its own and one chosen
unit is easier to remember than two. It defaults to `0.004`, about four pixels
at 1080p. The line straddles the outline, half inside and half out, so a
shape's stated size is the size of the shape rather than of its ink. **A
text `stroke` of the same name is drawn the other way** — entirely outside the
letterform — and the text section above says why the two differ.

**Where it sits is `anchor` and `transform.position`**, the same two things
that place a title. `anchor` puts the shape's own edges against the frame's —
`{ "x": "left", "y": "top" }` in the corner, absent for centred — and
`transform.position` offsets from there. There is nothing shape-shaped about
placement, and there deliberately is not: a second way to say where something
goes is two things to disagree the first time one is animated.

`fit` is meaningless here for the reason it is meaningless on a colour or a
title: there is no source raster to reconcile. It is not an error, it is simply
not read.

It composites like any other layer, so a box that fades up or slides in is
keyframes and nothing new. Text inside a shape is likewise nothing new: a text
clip on the track above a shape clip sits on top of it.

#### Arrows

```json asset
{ "id": "a-to-b", "kind": "shape",
  "shape": {
    "geometry": { "arrow": { "from": { "x": 0.30, "y": 0.40 },
                             "to":   { "x": 0.62, "y": 0.62 },
                             "curve": "s", "heads": "end" } },
    "stroke": "#000000ff",
    "stroke_width": 0.004
  } }
```

An arrow is the part of a diagram that carries the meaning — boxes are its
nouns and arrows its verb — and it is the one shape with no workaround: a box
is a scaled colour clip if you are determined, and a line at an angle with a
head on it is not expressible by anything else in this format.

**It is placed differently from every other shape, and the difference is
deliberate.** `from` and `to` are each either a place on the frame or a clip to
follow — see *Arrows that follow a clip* below. A place is written as
**absolute** fractions of the raster: `x` from the left edge, `y` from the top,
so `{ "x": 0.5, "y": 0.5 }` is the middle of the picture at any resolution. That is the one place these fractions differ
from `transform.position`, which offsets a layer from where it already sits —
a line has no "already sits" to offset from. **`anchor` is therefore not read
on an arrow**: there is no rectangle to rest against an edge.

A point outside `0`–`1` is allowed and is not a mistake. An arrow entering from
off-screen is an ordinary thing to draw, and the only refusals are an endpoint
that is not a pair of numbers and two endpoints in the same place — which has
no direction, so no head could be aimed.

`curve` is `straight` (the default) or `s`. The S **leaves the start and
arrives at the end along the same axis**, which is what a connector between two
boxes side by side wants; a straight diagonal between them reads as a mistake.
Which axis it bows along is *inferred* — whichever of the two the ends are
further apart on — and how far it bows is fixed. Neither is a field, because
neither is a question an author drawing a diagram has an opinion about.

`heads` is `end` (the default), `both`, or `none`. **A plain connecting line is
`"heads": "none"`**, which is why there is no `line` outline of its own: it
would be the same geometry, the same path and the same stroke, differing only
in whether one triangle is filled. `end` is the default because an arrow is
drawn to say *this leads to that*, and that reading has a direction. The head
is sized from `stroke_width`, so it stays in proportion when the line thickens,
and on a bowed arrow it is aimed along the curve's own tangent rather than
along the straight line between the ends.

**`fill` is refused on an arrow.** A line encloses nothing, so there is no
inside for a colour to go in, and a fill there would be read by nothing —
usually a `stroke` that was meant. `stroke` is the whole of an arrow's
appearance: no stroke, no arrow.

#### Arrows that follow a clip: `attach`

```json asset
{ "id": "a-to-b", "kind": "shape",
  "shape": {
    "geometry": { "arrow": {
      "from": { "attach": { "clip": "c-box-a", "side": "right" } },
      "to":   { "attach": { "clip": "c-box-b", "side": "left" } },
      "curve": "s" } },
    "stroke": "#000000ff",
    "stroke_width": 0.004
  } }
```

Either end may name a **clip** and a **side** instead of a place. The endpoint
is then worked out from wherever that clip actually is, **on every frame** —
move the box and the arrow follows it, animate the box and the arrow follows it
frame by frame.

**Without this, every arrow is placed twice**: once when you draw it, and again
each time you move a box. That is the difference between a diagram you can edit
and a picture you have to redo, and it is worse for an assistant than for a
person, because an assistant moving a box has no way to see that it broke three
arrows.

`clip`, not `asset`. The same box asset can be on screen twice at once, and an
arrow has to say which of them it means.

`side` is `left`, `right`, `top`, `bottom` or `center`, and `center` is the
default — the honest answer when an arrow is meant to point *at* something
rather than touch it. **The side you name stays the side you get.** Choosing
the nearest one automatically is the kind of helpfulness that is right until it
is wrong, and wrong here is a diagram whose arrows rearranged themselves between
two renders.

**What it attaches to is what the clip *shows*, not the layer it is drawn
into.** For a title that is the **wrapped block** — the same rectangle `anchor`
reasons about, not the longest line. For a shape it is the shape's own box. Both
are drawn into a raster the size of the whole frame, so attaching to the layer
would meet an edge that is not on screen at all. For a picture it is the picture:
a letterboxed `fit` clip is its fitted rectangle, bars excluded.

Three things are refused, all from the document alone: a clip id the timeline
does not have, a clip on an **audio** track — sound has no rectangle — and a
clip that is **itself an arrow**. That last one is a blanket rule rather than a
cycle check, and it is what makes resolving endpoints per frame safe: an arrow
following an arrow following the first has no answer, and a line has no side
worth meeting anyway.

**An arrow whose clip is not on screen while the arrow is** is left out of those
frames, and the render says so in a note. Holding the endpoint where the box
would have been draws a line into empty space pointing at nothing, which is a
worse answer than an absent arrow and a sentence explaining it. Usually it means
the arrow's clip outlasts the box's, or starts before it.

Elbow and orthogonal routing, obstacle avoidance, editable control points,
labels riding along the line, dashes and multi-segment paths are not here.
Polygons, stars, dashed borders, shadows and gradients are not planned at all:
each is a drawing program growing inside a video editor.

### Icon assets

```json asset
{ "id": "play-badge", "kind": "icon",
  "icon": {
    "name": "clapperboard",
    "size": 0.12,
    "color": "#ffffffff",
    "stroke_width": 0.08
  } }
```

The fourth kind with no file behind it, and the one that draws a **symbol this
build already ships** — the play triangle, the clapperboard, the arrow, the
warning triangle a video is annotated with. scorsese carries the
[Lucide](https://lucide.dev) set, so a document names one the way a `style`
names a font.

**A name is portable in a way a path is not.** `"clapperboard"` survives
`scp -r` between machines because the symbols travel with the binary, and it is
a few bytes instead of the megabyte a PNG of the same drawing costs — sharp at
4K, and recoloured by editing one string rather than by going back to the other
program.

`icon` is required on this kind and refused on every other. Inside it:

| Field | Required | Meaning |
| --- | --- | --- |
| `name` | yes | Which symbol, by Lucide's own name for it — lowercase and hyphenated |
| `size` | yes | How big, as a fraction of the raster's **height** |
| `color` | yes | The one colour it is drawn in, as `#rrggbb` or `#rrggbbaa` |
| `stroke_width` | no | How thick its line is, as a fraction of the **icon's own box**. Defaults to `0.0833` — Lucide's own `2/24` |

**`size` is one number against one axis, and the icon stays square.** This is
the one place the unit differs from the neighbouring kind, so it is worth
reading twice: a `shape` takes a `width` against the frame's width and a
`height` against its height, because a rectangle has two independent sides. An
icon does not — every symbol is drawn in a 24×24 square — so it takes one
measurement, against the same axis a text `size` uses. The same number of pixels
comes out both ways, on every aspect ratio. `0.12` on a 1080-line render is a
130-pixel square whether the frame is 16:9 or vertical.

**`stroke_width` is a fraction of the icon, not of the frame**, and that is the
opposite choice from a shape's `stroke_width`. A shape's border is a fraction of
the raster's height and deliberately does *not* scale with the box: a callout
wants the same visible weight whatever size it is. A symbol wants the opposite.
Halve an icon whose stroke is measured against the frame and the line stays as
thick while the drawing shrinks around it, until the counters close up and the
symbol reads as a blob. Written against its own box, a half-size icon is simply
the same picture, half the size. `0.08` is a little heavier than the default;
`0.05` is a fine hairline on a large symbol.

**One colour, because a symbol has one.** The whole visual vocabulary of the set
is a single stroke, so there is no `fill` and there is no second colour — and
`fill` is not merely absent but meaningless: Lucide paths are open strokes, and
painting an interior across them produces garbage rather than a filled icon.
There is no default colour, for the reason a `color` asset has none.

**Where it sits is `anchor` and `transform.position`**, exactly as for a shape
or a title, and `fit` is meaningless here for the same reason — there is no
source raster to reconcile. It composites like any other layer, so an icon that
fades up, slides in or grows is keyframes and nothing new: **nothing about an
icon animates on its own**, and one that grows uses `transform.scale`.

**Finding the name is a search, not a list.** Seventeen hundred symbols is far
too many to read through, so `scorsese icons <word>` — and the `icons` MCP tool
— match a word against every icon's name, against the words upstream files it
under, *and* against the names upstream has retired. The second of those is what
turns "the film camera one" into `clapperboard`, which answers to *movie*,
*film*, *cinema* and eight more, none of them in its name; the third is what
turns `unlock` into `lock-open`. What comes back is written here verbatim — a
hit that only a retired name matched sorts last and says so
(`lock-open (formerly unlock)`), and the name to write is the one before the
brackets. **A retired name is findable, never writable**: `"name": "unlock"` is
refused exactly as any other unknown name is.

**A name the build does not ship is refused**, and the refusal names the close
ones — `scorsese check` reports it as a problem, the way it reports a `style`
naming a face that is not there. The set is too large to list in an error, so
what a wrong name gets back is the near matches — for a typo
(`clapperbord` → `clapperboard`), for a half-remembered compound
(`play` → `play`, `circle-play`, `square-play`), and for the start of a name
(`clapper` → `clapperboard`). Nothing is suggested when nothing is close, which
is an honest answer rather than a guess. A render that meets an unknown name
anyway draws an empty layer and says so in its report rather than stopping.

Not here: user-supplied SVG, multi-colour icons, any set beyond the one that
ships, and gradients — the last for the reason the `color` section already
gives.

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
| `native` | not scaled at all; the source arrives at its **own pixel size**, resting centred — so it covers a **different fraction of the frame at every render size** | a logo or badge at the size it was authored, where its own pixels are the point |

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

**And what that costs: `native` is a promise about pixels.** A pixel is the one
unit the rest of this format refuses to measure anything in — `size`,
`max_width`, `crop` and `transform.position.*` are fractions precisely because
resolution is a render setting. `native` opts out of that rule deliberately,
and it keeps its promise exactly as written: the same source is the same
*number* of pixels at 720p, 1080p and 4K, and therefore a **different fraction
of the picture** at each. `fit` with a `transform.scale` makes the opposite
promise — a fitted layer is derived from the raster by construction, so a scale
on it is a fraction of a fraction, and the layer is the same share of the frame
at every resolution of that shape. Both are coherent and they disagree, so the
sentence above is about the *pixel size* that `0.06` was worked out to produce:
that is what stops being right when the resolution changes, and the fraction is
what survives it.

**A `native` layer previewed at one resolution is not the layout the render
delivers.** `scorsese still` and the `still` tool composite at whatever raster
they are asked for — 1920×1080 by default on the command line, 1280×720 over
MCP — and for everything measured in fractions a smaller one is the same
picture with fewer pixels in it. A `native` layer is the exception, because the
same count of pixels in a smaller frame is a bigger layer: a badge 240 pixels
across is 240 pixels across in either, which is 18.8% of the width of a
1280×720 preview and 12.5% of a 1920×1080 delivery. Still it at the raster the
render will use, or judge everything about it except its size.

**A layer whose size is a proportion of the picture belongs on `fit`.** A
corner logo, a badge, a watermark — anything whose real specification is "about
a tenth of the frame" — is `fit` with a `transform.scale`, and the scale being
a number that means nothing to a reader is what that reading costs. `native` is
for the other case, the one it was named for: where the source's own pixels are
the point.

**Scaling a `native` layer loses both readings at once, and is worth avoiding.**
It is no longer the size the source was authored at, and it was never a fraction
of the frame, so the number means "these pixels, times this, whatever the render
turns out to be" — which is neither thing a fit mode exists to give. A
1402-pixel-wide logo at `transform.scale.x: 0.105` is 147 pixels wide at every
resolution, which is 11.5% of a 1280×720 frame and 7.7% of a 1920×1080 one; the
same `0.105` on the same logo under `fit` is 8.8% of the frame at both. So a
`native` clip carrying a `transform.scale` almost always wants `fit` and the
same number. It is accepted rather than refused because the source's pixel size
is not in the document — only opening the file says what it is — so from here a
deliberate one and a mistake look identical.

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

**Three near-neighbours, and which of the three answers your question.**
`anchor` says which edge of the *frame* this layer rests against — where the
layer sits. `origin`, below, says which point of the *layer* its own scale and
rotation pivot on — what the layer does about itself once it is there. An
arrow's `attach` says which *clip* an end of it follows, and is inside a shape's
geometry rather than on a clip at all. All three deliberately carry different
words, because a format with two things called `anchor` is a format nobody can
read back.

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

**An anchor moves a layer exactly when there is somewhere to move it**, which
falls out of the geometry rather than being a rule about fit modes. A layer the
size of the raster rests at the origin whatever its anchor, because every edge
already meets the matching one. So:

- **`fill` is always a no-op.** Covering the raster is what filling means;
  there is no spare by construction.
- **`fit` is a no-op only when the aspects match.** A letterboxed clip —
  1920×1080 inside a 1920×1440 raster — has 360 pixels of spare, and
  `"anchor": { "y": "bottom" }` rests the picture on the frame's bottom edge.
  That is what a caption band above a wide clip is made of, and the alternative
  is `transform.position.y: 0.125`: a number derived on paper from a raster the
  project is not supposed to know about, which stops being right the moment the
  render's resolution changes.
- **`native` is a no-op only when the source happens to be raster-sized**, for
  the same reason.

An anchor that lands on a no-op is accepted rather than refused. For `fit` and
`native`, whether it does cannot be answered from the document at all — it
depends on the source's pixel size, which only opening the file can say — so
refusing would mean either a `fill`-shaped special case or a validation rule
that needs the media present. Neither is worth it for a field whose no-op is
free.

**It is a field, not an animatable property**, and deliberately: an anchor says
how a coordinate is to be *read*, and animating that would slide a layer by
changing what its number means. `transform.position` stays the animated part.
What scale and rotation happen *about* is a separate question with a separate
answer, and it is the next section.

### Which point it turns about: `origin`

```json clip
{ "id": "c-bar", "asset": "bar", "start": 0, "duration": 1440,
  "anchor": { "x": "left", "y": "bottom" },
  "origin": { "x": "left", "y": "center" },
  "keyframes": [ { "property": "transform.scale.x",
                   "keyframes": [ { "t": 0, "value": 0.0 },
                                  { "t": 1440, "value": 1.0 } ] } ] }
```

`x` is `left`, `center` or `right`; `y` is `top`, `center` or `bottom`. Absent
means the layer's own centre on both axes, which is what every scale and every
turn did before the field existed — so no document written before this changes
by a pixel.

**It is the point `transform.scale` and `transform.rotation` pivot on.** That
is the clip above: a gold rule along the foot of the frame that fills from the
left over sixty seconds, as **one** keyframe track. Without an origin, scale
turns about the middle, so the same bar needs a second track sliding it left by
`(s − 1) / 2` on every frame — a number nobody can read back as *the left edge
stays put*, and one that is only right while the scale is linear in time. Put an
`ease_out` on the scale and the two come apart: the bar slides while it grows.
The document still validates and the render still succeeds; the only symptom is
watching it.

**Scale and rotation both.** A card hinging on its left edge is the same request
as a bar filling from it, and one pivot for the two transforms is the coherent
reading of *the point the layer turns about*. A second field for rotation alone
would be a near-synonym the format cannot afford.

**`transform.position` is applied after both and is unaffected.** A pivot cannot
move a layer that is not being scaled or turned, which is what makes the field
free to set on a clip that is only being placed.

**What it is a point *of* is the layer's own box** — the raster its pixels
arrive on. For a decoded picture that rectangle is the picture. For anything
drawn — a title, a shape, an icon — the layer is the size of the render's
raster, with the content placed inside it by `anchor`, so `left` is the frame's
left edge. That is exactly right for the full-width bar above, and worth knowing
before pivoting a shape that occupies a corner of the frame.

**Named points rather than a pair of fractions**, which would be more general
and would be a fourth coordinate space in a format that already has three —
fractions of the *output* raster (`transform.position`), fractions of the
*source* raster (`crop`), and fractions of the frame's **height** (a text
`size`, an icon's). `anchor` established this vocabulary and this is the same
vocabulary; a reader who knows one knows the other.

**It is a field, not an animatable property**, for the reason `anchor` is one:
it says how a transform is to be *read*, and animating it would move a layer by
changing what its numbers mean. If the pivot itself has to travel, that is
`transform.position` — the property whose job that is.

### How it looks: `grade`

```json clip
{ "id": "c-arrival", "asset": "shot-a", "start": 0, "duration": 120,
  "grade": { "saturation": 0.18, "temperature": 0.12, "vignette": 0.35 } }
```

Five numbers, all optional, every default the **neutral** one — so a grade that
says nothing changes nothing, and a clip with no `grade` at all is the clip
exactly as it arrived.

| property | neutral | which way it runs |
| --- | --- | --- |
| `saturation` | `1.0` | `0.0` is fully grey; above `1.0` oversaturates |
| `temperature` | `0.0` | negative is cooler (blue), positive is warmer (amber) |
| `brightness` | `0.0` | negative darkens, positive lightens |
| `contrast` | `1.0` | below `1.0` flattens, above `1.0` steepens |
| `vignette` | `0.0` | `0.0` is none; higher darkens the corners further |

**It applies to every layer kind, not only video.** The compositor does not care
what produced the pixels it is handed, and a title card that does not warm along
with the shot behind it is the wrong result. An image, a text layer and a
generated shot all take a grade on the same terms.

**Numbers, never named looks.** The generality rule — core defines property
*types*, never property *values* — rules out a `"look": "70s"` field here as
surely as it rules out "make text red". `saturation: 0.18` is a property; the
70s look is five values somebody chose, and those belong in a project, a
document, or an assistant's suggestion.

**A field *and* five animatable properties**, which no other clip property is.
The field is the clip's baseline; a `grade.*` keyframe track **takes that one
property over** for the whole clip, since a track holds its first and last
values outside its own range and so always has an answer. Every property no
track names still comes from the field, which is what makes "warm throughout,
and desaturate over the first second" two lines rather than a choice between
them.

Both readings are ordinary — most of the time a shot has *a* look, written once;
sometimes the look arrives over three seconds, which is a ramp between two
numbers like any other:

```json clip
{ "id": "c-bloom", "asset": "shot-a", "start": 0, "duration": 90,
  "grade": { "saturation": 0.0 },
  "keyframes": [ { "property": "grade.saturation",
                   "keyframes": [ { "t": 0, "value": 0.0 },
                                  { "t": 90, "value": 1.0 } ] } ] }
```

**What is deliberately absent**: curves, LUTs, colour wheels, per-channel
lift/gamma/gain, scopes, and any primary/secondary distinction. Each of those is
what "scorsese is not a compositing suite" is refusing, and each is only legible
to somebody who already grades professionally. `contrast` is in, and is the one
entry that had a real question against that line: it is one number on a slider,
which is the shape this is measured against, and not the beginning of a curves
editor.

**Order of operations**, because a grade is more than one thing happening and
the order changes the picture: saturation, then temperature, then contrast,
then brightness, then vignette. Saturation runs first so that warming a
desaturated shot warms it rather than being undone by the desaturation;
brightness runs after contrast so that the contrast knob does not shift the
exposure as a side effect. Everything is clamped to the displayable range on the
way out — an overshoot is a blown highlight, not a wrapped one.

The vignette is measured from **the layer's own centre**, not the frame's, so
one on a picture-in-picture darkens that picture's corners rather than a ring it
happens to sit inside. Alpha is never touched, by the vignette or anything else:
a grade changes what colour a pixel is, never how much of it there is.

### How soft it is: `blur`

```json clip
{ "id": "c-rooftop", "asset": "shot-a", "start": 0, "duration": 120, "blur": 0.012 }
```

One number. `0.0` — and an absent `blur` — leaves the clip exactly as sharp as
it arrived; small softens it, large takes it to mush. Softening a plate so a
title reads over it, taking a logo or a plate number out of legibility, dropping
a background out of focus: all of them are this field, and none of them is a
round trip through another program any more.

**The unit is a fraction of the layer's own height.** `0.012` on a 1080-tall
source is about thirteen pixels; the same number on the 4K version of the same
shot is about twenty-six, which is the same softness on the same picture. A
pixel count in the document would be wrong the first time the project was
delivered at another size, which is the reason a `text` size and a `shape`
stroke are fractions too. Anything under half a pixel is nothing, and costs
nothing.

**`transform.scale` multiplies the apparent blur.** The softening happens on the
layer's own pixels, before the transform places them on the canvas — so a clip
drawn at 200% looks twice as soft as the same number at 100%, and one at 50%
half as soft. That is what every editor does and what "this shot is soft" means,
and it is written down here because it is otherwise discovered by surprise.

**A field *and* an animatable property**, like `grade`: the field is the clip's
baseline, and a `blur` keyframe track takes it over for the whole clip. That is
what a focus pull is — two numbers and a ramp — with no mechanism of its own:

```json clip
{ "id": "c-resolve", "asset": "shot-a", "start": 0, "duration": 90,
  "keyframes": [ { "property": "blur",
                   "keyframes": [ { "t": 0, "value": 0.05 },
                                  { "t": 45, "value": 0.0 } ] } ] }
```

It is `blur` and not `grade.blur` deliberately. A grade is the closed set of
*colour* properties, and every one of them reads one pixel and writes one pixel;
a blur reads a neighbourhood. Filing it under a struct that says colour would
make that description untrue about what it holds.

**It applies to every layer kind**, for the same reason a grade does: the
compositor is handed a rectangle of pixels and does not know whether a decoder,
a title or a shape produced them. A blurred title is as ordinary a thing to want
as a blurred plate.

**What is deliberately absent**: motion blur, zoom blur, bokeh and tilt-shift,
each of which is a lens being simulated rather than a picture being softened;
blurring a *region* rather than a whole layer; backdrop blur, which softens what
is *behind* a layer and is a different operation in a different place; and
sharpening, which is not negative blur. A negative number here is not an error —
it simply softens nothing, the way `0.0` does.

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
| `blur` | how far the layer's own pixels are softened, as a fraction of its own **height** | `0.0` untouched, higher is blurrier |
| `transform.position.x` | offset right, as a fraction of the raster's **width** | `0.0` unmoved |
| `transform.position.y` | offset down, as a fraction of the raster's **height** | `0.0` unmoved |
| `transform.scale.x` | width multiplier about the layer's `origin` | `1.0` natural size |
| `transform.scale.y` | height multiplier about the layer's `origin` | `1.0` natural size |
| `transform.rotation` | turn about the layer's `origin`, in **degrees clockwise** | `0.0` upright |
| `transform.flip.x` | turn about the layer's own **horizontal** axis, in degrees | `0.0` face on |
| `transform.flip.y` | turn about the layer's own **vertical** axis, in degrees | `0.0` face on |
| `grade.saturation` | how much colour, about each pixel's own grey | `1.0` untouched, `0.0` fully grey |
| `grade.temperature` | which way the whites lean: negative cooler, positive warmer | `0.0` untouched |
| `grade.brightness` | light added to the layer, as an offset | `0.0` untouched |
| `grade.contrast` | how steep the layer's range is about mid-grey | `1.0` untouched |
| `grade.vignette` | how much the layer's own corners are darkened | `0.0` none, `1.0` corners black |
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
`text` or `style`, only a `color` asset carries `color`, only a `shape`
asset carries `shape` and only an `icon` asset carries `icon`, that an icon has
a size and a thickness to draw with, that a shape has area, a corner it has room to round,
and something to draw with — and that an arrow has two ends in different
places and no `fill`, since a line has no inside — that a `style`'s
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

**An `icon`'s `name` splits the same way, and the reason is the generality
rule.** The format says an icon *has* a name; which names exist is a set of
property values, and that list lives beside the code that draws them rather
than in the model. So validation checks the block is on the right kind and that
its numbers describe something with ink in it, and *whether the symbol exists*
is `check`'s answer and the render's — a problem, reported with the near
matches, exactly as an unknown font name is.

`check` also reports what the *picture* stacks, which is the one thing neither
the document nor the pool can show you. Every layer is placed independently, in
fractions, against a frame it does not otherwise know about — so two of them
colliding is not a field anybody can read. It is an emergent fact about numbers
that were each individually reasonable, and the only way to find it used to be to
composite a still and look at it, which finds the collisions at the instants
somebody happened to sample. At every instant where the visible set changes,
`check` compares the content rectangles of clips on different video tracks — the
compositor's own rectangles, the ones `scorsese describe --at` reports — and
names the pairs drawn across each other, with the span and how deep the overlap
gets. Always a warning: layers overlapping on purpose is what most films are made
of, and a title over a shot can never fail a render or move an exit code.

Which is why most of that pass is a filter, and why it deliberately
under-reports — a warning that always fires is a warning nobody reads. A layer
covering 85% or more of the frame is a background or a scrim, and overlaps
everything above it by construction. A pair whose smaller rectangle is under a
quarter of the larger's area is decoration on it rather than a collision with
it — that one is what keeps a caption over a shot, or a badge on a plate,
silent. An overlap covering less than a fifth of the smaller rectangle is two
upright bounding boxes grazing. And one lasting under a second is a transition:
`scorsese dissolve` writes exactly this shape — two clips on different tracks,
over each other — for half a second, by default.

**Text is filtered harder, and the reason is the rectangle.** A text layer's
rectangle is its wrapped block — `max_width` wide, whatever the words come to —
because that is the box the anchor reasons about and the box an arrow attaches
to. It is generous by design: a label reading `T` in a `max_width` of `0.048`
has a box many times the width of the letter, and two centred captions side by
side can share a quarter of their boxes with clear air between the words. So
when either layer is text the overlap has to cover **between a half and 85%** of
the smaller rectangle. Below a half, what the two share may be nothing but the
margin the wrap box added; above 85%, one is inside the other, which is the
label-in-a-box arrangement this format encourages most. A pair with no text in
it has no upper bound — one picture wholly hiding another the same size is not a
label, it is a layer nobody can see.

What is left is partial overlap between two layers of comparable size, held for
seconds, which is what an authoring mistake actually looks like.

`style.weight` splits the same way, and the split is worth reading once. What
the document alone can say is checked here, and it is one thing: a number
outside OpenType's own 1–1000 is not a weight for any face. What only the
*file* can answer — whether it is variable at all, and how far its `wght` axis
runs — is refused at the render, in the same breath as "this is not a font I can
read". That holds for `sans` and `serif` as much as for a font the project
carries; they are files too, and scorsese happens to be the one carrying them.

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
