# `project.json` — schema v47

The contract between the CLI, the MCP server and the GUI — the contract *now*,
not across time. It is meant to be hand-written: an agent should be able to
author a whole video in this file and render it without touching a mouse.

Changing this format is `architecture` work — it needs a `schema_version`
bump **and a migration** from the previous version, in the same change
(`CLAUDE.md`, *A schema bump ships with a migration*). Projects stored by the
web app belong to other people, and a bump that stranded them would break
them. The bump is still what makes a break honest — a document whose version
is not this build's is refused on sight instead of being read as something it
no longer means — and the migration is the only way past that refusal: the
server runs it over every stored document when it starts, and `scorsese
migrate` runs the same steps over a local `.scor` folder. The steps live in
`scorsese_core::migrate`, one per version from v33 (the oldest this build
carries forward) up to this one.

| step | what changed | what the step does |
| --- | --- | --- |
| v33 → v34 | the `group` asset kind (#586) | nothing: a kind was added and nothing a v33 document says changed meaning, so it passes through and only its version moves |
| v34 → v35 | the overshooting easings and `cubic_bezier` (#587) | nothing: values an `easing` may take were added, and every easing a v34 document names is the same curve, so it passes through and only its version moves |
| v35 → v36 | a shape's optional `dash` (#583) | nothing: a field was added whose absence is the solid line every v35 shape already draws, so it passes through and only its version moves |
| v36 → v37 | a text style's `reveal` and `number` blocks (#590) | nothing: two optional blocks were added, absent in every v36 document, and a `{n}` in a v36 text stays three ordinary characters without a `number` block — it passes through and only its version moves |
| v37 → v38 | a clip's `shadow`, `glow` and `blend` (#585) | nothing: three optional clip fields were added, each absent meaning what every v37 clip already drew — no shadow, no glow, `normal` — so it passes through and only its version moves |
| v38 → v39 | a clip's optional `follow` (#584) | nothing: a field was added whose absence is a clip placed by its transform alone, which every v38 clip is, so it passes through and only its version moves |
| v39 → v40 | gradient fills (#588) | nothing: a shape's `fill` and a colour asset's `color` gained a gradient object beside the colour string, which still means the colour it did, so it passes through and only its version moves |
| v40 → v41 | a clip's `matte` (#589) | nothing: one optional clip field was added, absent meaning what every v40 clip already drew — the clip shown whole — and no v40 clip names another as its matte, so it passes through and only its version moves |
| v41 → v42 | the `generated_image` kind and its `image` block (#461) | nothing: a kind was added with a block only it carries, and a shot's stills may now name a generated still — which only admits documents v41 refused — so every v41 document passes through and only its version moves |
| v42 → v43 | the `image_sequence` kind and its `sequence` block (#462) | nothing: a kind was added with a block only it carries, and the stills it plays are ordinary `image` assets, so every v42 document passes through and only its version moves |
| v43 → v44 | the `html` kind and the `pages/` directory (#774) | nothing: a kind was added that no v43 document can contain, with no block of its own — a page is a `path` like any file's — so every v43 document passes through and only its version moves |
| v44 → v45 | a shot's `standard` tier and `4k` resolution (#891) | nothing: two values were added to fields that already existed, and every tier and raster a v44 shot names is the same one at the same price, so every v44 document passes through and only its version moves |
| v45 → v46 | stills at parity with Google's image models (#893): `nano_banana_2.1` (the new default) and `pro`, the four long strips, `thinking`, and references split into `reference_images` (objects), `character_images` and `style_images` | two rewrites, so every still asks for what it did: a still that named no model is written out as `"model": "flash"`, the old default; and a `flash` still's references past the tenth move to `character_images`, the same pictures in the same order. Neither moves a brief's fingerprint, so nothing already drawn turns `stale` |
| v46 → v47 | a still may wait in a half-price batch (#894): a `generated_image` may be `queued` with the batch job's name as its `operation` | nothing: v46 refused an `operation` on a still, so this only admits documents v46 refused — every v46 document passes through and only its version moves |

A complete worked example lives in
`crates/core/tests/fixtures/narrated_teaser.json`.

## The document

```json project
{
  "schema_version": 47,
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
| `kind` | all | `video`, `image`, `audio`, `text`, `color`, `shape`, `icon`, `group`, `image_sequence`, `html`, `generated_video`, `generated_image`, `generated_audio`, `synth_audio` |
| `path` | file-backed kinds | Relative to the project root; an `html` asset's ends in `.html` |
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
| `group` | `group` | The tracks it holds, which render as one layer — see below |
| `sequence` | `image_sequence` | The stills it plays in order, how many frames each is held, and whether it loops — see below |
| `note` | optional | Why this asset is what it is. Never rendered — see above |
| `video` | optional, `generated_video` only | The rest of the brief: `model`, `resolution`, `seconds`, `aspect`, `first_image`, `last_image`, `reference_images` — see below |
| `image` | optional, `generated_image` only | The rest of the brief: `model`, `resolution`, `aspect`, `thinking`, `reference_images`, `character_images`, `style_images` — see below |
| `speech` | optional, `generated_audio` only | The rest of the brief: `model`, `voice_id`, `language`, `seed` — see below |
| `created_at` | optional | When the asset joined the table, as UTC RFC 3339 (`2026-08-04T14:20:00Z`) |
| `queued_at` | optional, generated kinds | When a provider took the request. Not the same fact as `created_at` |
| `operation` | optional, `generated_video`, `generated_image` | The provider's name for work in flight, while `queued`: a shot's Veo operation, or the batch a still waits in |
| `estimated_cost_cents` | optional, prompted kinds | What realising it was *calculated* to cost, in US cents — our arithmetic, never a bill. See [prices.md](prices.md) (`guide prices`) |

```json asset
{ "id": "shot-city", "kind": "generated_video", "state": "sketch",
  "prompt": "wide aerial of a city at dawn, slow push in" }
```

`media` is what something measured, never what anyone chose — so an asset you
write by hand leaves it out and has it filled in later. `scorsese probe` reads
every asset that has a file and no `media`, and `project_write` over MCP does
it for every document it writes;
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

Four kinds are generated, and they differ in what the brief *is*:

| Kind | Brief | Realised by | Costs |
| --- | --- | --- | --- |
| `generated_video` | `prompt` — a sentence | Veo, over the network | money |
| `generated_image` | `prompt` — a sentence | Gemini's image models, over the network | money |
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

A render can leave the bands out (`scorsese render --no-narration-bands`, or the
MCP `render` tool's `narration_bands: false`): for a preview whose captions
already carry every word, where a band would cover them. That is a choice made
per render, like its resolution, and nothing in the project records it. The
clips stay where they are — silent, still ducking the music — and the render
reports each line it left out. A recipe not yet baked is a sound not yet made
too, and its band goes with them; a page's card is not a sound and is always
drawn.

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
| `model` | `standard`, `fast`, `lite` | `fast` |
| `resolution` | `720p`, `1080p`, `4k` | `1080p` |
| `seconds` | `4`, `6`, `8` | `8` |
| `aspect` | `16:9`, `9:16` | `16:9` |
| `first_image` | a still's asset id — an `image` or a generated one | — |
| `last_image` | likewise | — |
| `reference_images` | up to 3 stills' asset ids | — |

Only `prompt` is required. Every field above has a default, so an absent
`video` and an empty one mean the same thing: a request made of a sentence and
nothing else.

The fields are the part that can be tabulated; the sentence is not. **What
certain words do to the shot that comes back is [`prompts.md`](prompts.md)
(`guide prompts`)** —
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
| `seconds` other than `8` at `1080p` or `4k` | those rasters are only generated at eight seconds |
| `seconds` other than `8` with `reference_images` | likewise |
| `seconds` other than `8` with a first **and** last image | likewise |
| `reference_images` on `lite` | that tier does not take them |
| `4k` on `lite` | that tier does not sell it |
| more than 3 `reference_images` | the provider accepts three |
| `last_image` without `first_image` | there is no journey from nowhere |
| a still that is not an `image` or `generated_image` asset, or not in the table at all | every still handed over is a picture |

The first three are one rule wearing three hats, and the message always names
the *other* choice — the one that fixed the length — because that is the one
worth reconsidering. Dropping to `720p` costs less than giving up the stills a
shot is built from.

Switching a shot to `lite` to save money is the case to watch: it is the one
change that can invalidate a brief rather than merely cheapen it, which is why
it is refused rather than honoured with the images dropped or the raster
shrunk.

These are Google's options, all of them: every tier and raster its
[Veo page](https://ai.google.dev/gemini-api/docs/veo) sells is here, so that
page is the checklist when it changes. Two differences are deliberate. Video
extension — a longer shot grown from a generated one — is a different kind of
request and is not modelled. And a first and last image fix the length at
eight seconds although the page no longer says so: confirming otherwise costs
a generation, so the refusal stays until one is paid for (#928).

### What a generated still asks for

A `generated_image` is a still that does not exist yet: a prompt, drawn by one
of Google's four Gemini image models. Once drawn it is a picture like any
imported `image` — no length of its own, held for as long as its clip says, and
moved by the same keyframes: a slow push in, a pan across a wide frame, a grade
that turns afternoon into dusk. It is the cheapest picture scorsese can
generate (about a sixteenth of a Veo shot, [`prices.md`](prices.md),
`guide prices`), and
unlike a shot it is reused: the same backdrop three scenes later.

```json asset
{ "id": "hero-running", "kind": "generated_image", "state": "sketch",
  "note": "the chase opens on this; she must read as the same woman as the sheet",
  "prompt": "the woman from the reference, mid-stride across a wet rooftop at dusk, wide",
  "image": { "model": "nano_banana_2.1", "resolution": "2K", "aspect": "16:9",
             "character_images": ["hero-sheet"] } }
```

| Field | Values | Default |
| --- | --- | --- |
| `model` | `nano_banana_2.1`, `flash`, `lite`, `pro` | `nano_banana_2.1` |
| `resolution` | `0.5K`, `1K`, `2K`, `4K` | `2K`; `1K` on `lite` |
| `aspect` | `16:9`, `9:16`, `1:1`, `4:3`, `3:4`, `3:2`, `2:3`, `5:4`, `4:5`, `21:9`, `4:1`, `1:4`, `8:1`, `1:8` | `16:9` |
| `thinking` | `minimal`, `medium`, `high` | the model's own: `medium` on `nano_banana_2.1`, `minimal` on `flash` and `lite` |
| `reference_images` | asset ids of **objects** to include faithfully — a product, a logo, a place | — |
| `character_images` | asset ids of **characters** to keep looking like themselves | — |
| `style_images` | asset ids of a **style** to draw in | — |

Only `prompt` is required, and an absent `image` means every default. Every
field — and the bytes of every reference — is hashed into the brief with the
sentence, so editing any of them makes the asset `stale`. Writing a default out
explicitly (the size, the thinking level) is the same brief as leaving it out,
and so is moving a reference from one kind to another: the vendor is never told
the kind, so the request is the same.

**Which model.** Speed is not a factor; quality against price per picture is,
and what a model cannot do decides the rest. As Google documents them, read
2026-10-08:

| Model | For | Sizes | Shapes | References at most | Thinking |
| --- | --- | --- | --- | --- | --- |
| `nano_banana_2.1` | the default: Google's recommended model, the best text rendering and consistency, and cheaper than `flash` at every size it draws | 1K, 2K, 4K | all 14 | 10 objects, 4 characters | `minimal`, `medium`, `high` |
| `flash` (Nano Banana 2) | 0.5K, the cheapest test of a prompt | 0.5K, 1K, 2K, 4K | all 14 | 10 objects, 4 characters | `minimal`, `high` |
| `lite` (Nano Banana 2 Lite) | the cheapest picture, where the money matters more | 1K | the first 10 | 14 objects | `minimal`, `high` |
| `pro` (Nano Banana Pro) | the most complex compositions, at the highest price | 1K, 2K, 4K | the first 10 | 6 objects, 5 characters, 3 styles | none to choose |

**`resolution` is the money lever**: the picture is billed per image at a price
fixed by its model and size. The default is `2K` because a 16:9 `2K` still is
2752×1536, which covers a 1080p frame with room to push into; a `1K` still is
enlarged before it moves. `aspect` is its own field rather than a consequence
of the size: a model draws every shape it offers at every size it offers.

**The `note` is what the still is for; the `prompt` is what is sent.** Write
the note first — *the chase opens on this*, *must match the sheet* — and the
prompt from it. The note is never sent to anybody and is not part of the brief,
so rewording it re-bills nothing.

**References keep a subject looking like itself.** Name pictures by asset id,
as Veo's `reference_images` does, in the field for their kind; they are handed
over objects first, then characters, then styles, and the prompt says what
each is for. A reference may itself be a `generated_image`: generate one
character sheet first, then name it in every still after it. It has to be
generated before a still drawn from it can be — a `generate` call draws the
sheet and reports the other as *not yet*, and the next call draws it.

| Refused | Because |
| --- | --- |
| a `resolution` the model does not draw | `0.5K` is `flash`'s alone; `lite` draws `1K` only |
| `4:1`, `1:4`, `8:1` or `1:8` on `lite` or `pro` | only `nano_banana_2.1` and `flash` draw the long strips |
| a `thinking` level the model does not offer | `medium` is `nano_banana_2.1`'s alone; `pro` offers none |
| more references of a kind than the model takes | refused with the number, never truncated — see the table above |
| a reference that is not an `image` or `generated_image`, or not in the table | every reference handed over is a picture |
| a still naming itself as a reference | it could never be drawn |

A still drawn now has no ticket: it comes back on the call that asked for it.
One ordered in a **batch** — half price, ready within 24 hours (`generate`
with `batch`; see [prices.md](prices.md), `guide prices`) — is `queued`, with
the batch job's name as its `operation` and `queued_at` stamped, until a later
`generate` collects it. Whether a still was batched is not part of its brief: the same
brief draws the same kind of picture either way, and lands at the same
`generated/<id>-<hash of the brief>.jpg`.

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

#### When each word is said

**A spoken line comes back with its word timings**, and they are kept beside
its audio: `generated/vo-open-<hash>.mp3` has
`generated/vo-open-<hash>.words.json` next to it, holding each word as it was
written — punctuation included — with when it starts and ends, in seconds of
the audio file. `generate` says where it saved them. Nothing in
`project.json` changes, and a line generated before timings were kept, or
speech that was imported rather than generated, has none: they are never
guessed.

**Anything timed to the voice reads them, never an ear or a silence
detector.** `caption_narration` (`scorsese caption`) turns them into captions:
each line cut at its sentences, commas and pauses into pieces of about two
lines, each an ordinary `text` clip arriving whole on its first spoken word
and leaving when the next arrives. It writes them on a video track of their
own (`captions` unless named) as assets and clips called
`caption-<narration clip>-<n>`, which is how a re-run finds and replaces its
own work after a line is regenerated or moved — and why a hand edit to one is
kept by renaming it. Pages read the same timings as `scorsese.words`
([pages.md](pages.md), `guide pages`), and `project_describe` names the word being said at an
instant.

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
same bytes on any machine, on any run of a given build. So the generated file
is named for a hash of the recipe's bytes *and* the version of the synthesiser
that rendered them, and `path` pointing at a hash the recipe no longer has —
or that this build's synthesiser no longer produces — *is* what `stale` means
for this kind. Nothing else has to record it, and a bake that a newer
synthesiser has superseded is redone by the next `synth bake` without anyone
finding it first. [`recipes.md`](recipes.md) (`guide recipes`) has the whole
of it.

The recipe is a separate file rather than inline JSON because a recipe is
long: a song is tracks, patterns and an arrangement, and inlining one would
bury the timeline under note lists in the document an agent reads to
understand the edit. It also makes the edit-and-rebake loop a single-file
diff. **What to write in one is [`recipes.md`](recipes.md)
(`guide recipes`).**

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
| `stroke` | *none* | `#rrggbb` or `#rrggbbaa` — a rim round the letters; absent means no edge |
| `stroke_width` | `0.002` | How far that rim reaches outward, as a fraction of the frame's **height** |
| `reveal` | *word by word* | How the text arrives piece by piece when the clip animates `reveal` — see below |
| `number` | *none* | The figure written where the text says `{n}`, and how — see below |

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

#### An emoji in a caption renders, and nothing selects it

Write `"text": "Ship it 🔥"` and the fire appears, in colour, at the size the
rest of the line is set at. No field turns that on, no field turns it off, and
no asset kind is involved — a standalone 🎉 filling half the frame is a `text`
asset holding one character at a large `size`, composited and faded like any
other layer.

**It works because a font is the head of a chain rather than the whole of it.**
No face covers Unicode; none of the eight above has a fire. So a character the
face a document *named* has no glyph for is drawn by the next face that does,
and one such face ships: **Noto Color Emoji**, in its COLRv1 vector build, so a
large emoji stays sharp at 4K. Everything the named face *can* draw, it draws —
a text face with a `☺` in it sets `☺` in text, not in colour — and the fallback
is reached by a gap, or by a caption that asked for colour in so many words.

Five consequences worth knowing before writing one:

- **`❤️` and `❤` are different pictures, and a keyboard sends the first.**
  `U+FE0F`, the emoji presentation selector, is an invisible character that
  means *draw the one before me in colour*, and iOS and Android put it after
  every emoji whose base character predates emoji — `❤️ ☀️ ⚠️ ▶️ ⬆️` are each
  two characters, not one. So a character **both** faces have goes to the emoji
  face when the caption carries one and stays with the named face when it does
  not. `U+FE0E` asks the other way, for the named face's own outline. Neither
  is ever visible in the string, and neither is a field.

- **Line height comes from the named face alone.** A caption with an emoji in
  it is exactly as tall as the same caption without one, and its words wrap in
  the same places. The fallback contributes its glyphs' widths and nothing else.
- **A colour glyph is not tinted.** `color` sets the letters; the fire is the
  fire's own orange whatever colour the caption is. The one exception is a
  glyph the font itself asks to be drawn in the text's colour, which some
  symbols do.
- **Sequences are one drawing.** 👍🏽 is a thumb plus a skin-tone modifier and
  👨‍👩‍👧 is three people joined by zero-width joiners; each sets as a single glyph,
  and no line ever breaks inside one.
- **`stroke` is for letters.** A caption's rim is grown off a letterform's
  outline, and a colour glyph has none — so the words carry their rim and the
  emoji between them is drawn as itself. The stroke section below says why.

**A character no face at all covers is still dropped and still reported.** That
is unchanged: it draws nothing, takes no width, and `scorsese check` names it
with its code point before an encode is ever started. What changed is only that
the report is now about the whole chain, so it no longer objects to an emoji
that renders perfectly well.

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

**A colour glyph takes no rim.** A rim is a stroke of the path being filled,
and an emoji is not that path — it is a layered drawing in its own colours,
with no single outline to grow anything off. So `Ship it 🔥` with a stroke on
it comes out as rimmed letters beside a fire drawn as itself. That is also the
reading worth having: the letters get the legibility they asked for, and the
emoji does not acquire a sticker border it never needed, being its own
high-contrast shape already.

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
all, which is why a title that slides and fades needs nothing here. `fit` is
meaningless on a text clip: there is no source raster to reconcile, since the
text is drawn at whatever size the render is.

#### Revealing by character, word or line: `reveal`

```json asset
{ "id": "caption", "kind": "text", "text": "Three brokers, 144 partitions",
  "style": { "size": 0.06, "reveal": { "unit": "word", "rise": 0.2, "stagger": 0.5 } } }
```

```json clip
{ "id": "c-caption", "asset": "caption", "start": 0, "duration": 90,
  "keyframes": [
    { "property": "reveal", "keyframes": [
        { "t": 0, "value": 0.0, "easing": "back_out" },
        { "t": 30, "value": 1.0 }
    ]}
  ] }
```

The two things a whole layer cannot do are the two text has properties of its
own for. The first is arriving a piece at a time — the word-by-word caption, the
typewriter, the list that builds line by line. **The `reveal` property says
when**: `0` shows nothing, `1` shows all of it, animated on the clip with
keyframes like any other. **The `reveal` block says how**, and every field of it
has a default — so a `reveal` track on a text with no block at all reveals word
by word, and the block is only written to change that.

| Field | Default | Meaning |
| --- | --- | --- |
| `unit` | `word` | `char`, `word` or `line` — what the text is cut into |
| `rise` | `0.2` | How far below its place a piece starts, as a fraction of the text's **size**; `0` fades in place, negative drops in from above |
| `stagger` | `0.5` | How far one piece gets through its entrance before the next starts: `1` is strictly one after another, `0` is every piece at once |

**The pieces are counted in reading order**, line by line, and `reveal` sweeps
them evenly — at `0.5`, the first half have arrived or are arriving. Whitespace
is never a piece. **A character is what the eye reads as one**: an emoji with a
skin tone, a flag, a letter with its accent, and a glyph drawn by the fallback
emoji face each arrive whole, never in halves.

**Nothing reflows.** The text is laid out exactly as it would be drawn whole and
only then cut up, so every word of a half-revealed caption is already where it
will finish, and a finished reveal is the same pixels as the text with no
`reveal` track at all.

**The keyframe's easing belongs to each piece, not to the sweep.** The sweep
across the pieces is always even; the easing shapes every piece's own
entrance. So `back_out` makes each word overshoot its line and settle — the pop
of a title preset — and `ease_out` has each one decelerate into place. A
piece's opacity never goes past solid or below nothing, however the curve
overshoots; its rise overshoots freely, which is the point. `hold` is the
exception: it holds the sweep where it was until the next keyframe, as it holds
any other property. A `reveal` going *down* is an exit, and the easing then runs
in that direction of time — a `back_in` exit winds up before it leaves.

A typewriter is `"unit": "char", "rise": 0, "stagger": 1`.

#### A number that counts: `number`

```json asset
{ "id": "partitions", "kind": "text", "text": "{n} partitions",
  "style": { "number": { "value": 144, "locale": "pt-BR" } } }
```

```json clip
{ "id": "c-partitions", "asset": "partitions", "start": 0, "duration": 60,
  "keyframes": [
    { "property": "number", "keyframes": [
        { "t": 0, "value": 0.0, "easing": "ease_out" },
        { "t": 24, "value": 144.0 }
    ]}
  ] }
```

The second is a figure that counts. The text carries `{n}` where the figure
goes; the `number` block says how it is written; and the `number` property —
animated on the clip, like `reveal` — says what it is at each instant. With no
`number` track the figure is the block's own `value`. Everything around the
placeholder is ordinary text, so a prefix and a suffix are just words:
`R$ {n} mi`, `{n}%`, `{n} partitions`. Every `{n}` gets the same figure.

| Field | Default | Meaning |
| --- | --- | --- |
| `value` | `0` | The figure when no `number` track animates it — usually the one the count ends on |
| `decimals` | `0` | Digits after the decimal mark, `0` to `6`; the figure is rounded to them, half away from zero |
| `locale` | `en` | `en` writes `1,234.5`; `pt-BR` writes `1.234,5` |
| `grouping` | `true` | Whether thousands are grouped; off is what a year wants |

**The locale is the document's, never the machine's**, because a render has to
look the same on every computer. A minus is written only when something other
than zero survives the rounding, so a count through zero never shows `-0`.

**A counter never shows a figure past the keyframes it is between.** Scale and
position overshoot freely; a figure that overshoots is a number nobody wrote —
`151 partitions` on its way to 144 — so an overshooting easing still shapes
*when* the count arrives (early, and then it holds) and never *what* it says.

**The line does not move while it counts.** A figure growing from `0` to
`1,234` gains characters, and a centred line would slide sideways each time it
gained one. So the figures are set in the face's **tabular** forms — every digit
the same width, the font's `tnum` feature — and the figure is padded on the left
to the widest one the count will show (its `value` and its track's keyframes,
which is everything it can reach), with a figure space where that one has a
digit and a punctuation space where it has a separator. The digits finish in the
columns they started in, the way an odometer's do. The padding is room, not a
place to wrap: it is never broken at or collapsed.

**Which faces have tabular figures.** `inter` (`sans`), `source-serif`
(`serif`), `montserrat` and `lora` have `tnum`; `liberation-sans`,
`liberation-serif` and `jetbrains-mono` draw every digit the same width
already. `playfair-display` has neither: its digits keep their own widths, so
the padding still holds the line to the right number of columns but a `1` is
narrower than an `8`, and the line can drift by a fraction of a digit as it
counts. A font file the project carries is whatever it is — if it has no
`tnum`, the same. A face with no figure space of its own is given one by the
shaper, as wide as its `0`.

A `number` block on a text with no `{n}` in it is refused: a counter showing
nothing looks exactly like a broken one. A `{n}` on a text with no `number` block
is three ordinary characters.

Shadows are not here, and native text gains no new styling: a look this
section cannot describe is a page's to draw ([Web pages](#web-pages),
`guide pages`). Bold is
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
`#rrggbb`, or `#rrggbbaa` for one you can see through — or a gradient, below.
There is no default. A
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

#### Gradients

```json asset
{ "id": "backdrop", "kind": "color",
  "color": { "radial": { "center": { "x": 0.5, "y": 0.45 }, "radius": 0.8,
                         "stops": [["#1b2440", 0.0], ["#0b1020", 1.0]] } } }
```

A colour asset's `color` and a shape's `fill` take either the colour string
above or a **gradient object**: `linear` or `radial`, one key naming which. A
flat full-frame colour is what makes an explainer look like a slide deck; a
radial gradient lighter behind the subject is the ordinary motion-graphics
backdrop, and a gradient panel is the ordinary caption plate. The string form
is unchanged — every document that wrote a colour still means that colour.

```json asset
{ "id": "plate", "kind": "shape",
  "shape": {
    "geometry": { "rectangle": { "width": 0.6, "height": 0.2, "radius": 0.5 } },
    "fill": { "linear": { "angle": 90,
                          "stops": [["#e8590c", 0.0], ["#7048e8", 1.0]] } }
  } }
```

| Form | Fields | What it is |
| --- | --- | --- |
| `linear` | `stops`, optional `angle` (default `180`) | colours along a straight line across the box |
| `radial` | `stops`, `radius`, optional `center` (default the middle) | colours in circles out from a point |

- **Every coordinate is a fraction of the painted box**: the shape's own box
  for a shape, the whole raster for a colour asset. A gradient pill looks the
  same wherever it is placed and at every resolution.
- **`angle`** is in degrees, the CSS way: the direction the colours *travel*,
  clockwise from upwards — `0` bottom to top, `90` left to right, `180` top to
  bottom. The line runs through the box's centre and is just long enough that
  the first stop lands on one corner and the last on the opposite one.
- **`center`** is `{ "x", "y" }` across and down the box from its top-left.
  **`radius`** is a fraction of the box's **shorter side** — the unit a
  rectangle's corner `radius` uses, so circles stay circles on any box: `0.5`
  from the middle touches the nearer pair of edges, and on a 16:9 frame about
  `1.02` reaches the corners. Past the radius the last stop's colour carries on.
- **`stops`** is a list of `[colour, offset]` pairs, offsets from `0` to `1`.
  At least two, rising or level — two at one offset are a hard edge, as in
  CSS. Colours may carry alpha; they are blended premultiplied, so a stop
  fading to transparent does not drag a grey fringe through the middle.

**Gradients are dithered**, because an 8-bit ramp across a large dark area
bands, and H.264 at a low bitrate keeps the bands. The compositor adds noise
two levels either side before rounding — well under what an eye resolves —
which is what survives an encoder: a patterned (ordered) dither does not.
Nothing to set. On the one frame x264 opens a very starved encode with, a dark
gradient can still band; a higher bitrate is the fix, and `grade.grain` on top
adds little the dither has not already done.

Only the inside of a shape takes a gradient. A border (`stroke`), a caption's
colour and an icon's colour stay one colour each, and the gradient itself is
not animatable — a gradient that fades or slides does it through `opacity` and
`transform.*`, like any other layer. Conic and mesh gradients are not here, and
none are coming: gradients are frozen, and a richer backdrop is a page.

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
through — and `fill` may instead be a gradient across the shape's own box (see
*Gradients* above). A border over an absent fill is a callout that does not hide the shot
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

**An arrow follows only clips on its own timeline.** Inside a
[group](#group-assets) an arrow may follow a member of the same group, and it
resolves in the group's own space — so when the group moves, the arrow moves
with the boxes, drawn into the one layer. An arrow on the timeline following a
member *inside* a group, or a member following a clip outside, is refused: the
two sides are drawn in different spaces, one before the group's transform and
one after, and the same group can be on screen twice at once, so "that member"
would not even name one place. To point at a whole diagram, attach to the
**group clip**, whose rectangle is the raster the group is drawn on, moved by
the group clip's transform; to point at a box inside it, put the arrow in the
group beside the box. Arrows are frozen — bugs are fixed, nothing is added — so
this stays; a diagram that needs more is a page.

**An arrow whose clip is not on screen while the arrow is** is left out of those
frames, and the render says so in a note. Holding the endpoint where the box
would have been draws a line into empty space pointing at nothing, which is a
worse answer than an absent arrow and a sentence explaining it. Usually it means
the arrow's clip outlasts the box's, or starts before it.

Elbow and orthogonal routing, obstacle avoidance, editable control points,
labels riding along the line and multi-segment paths are not here, and shapes
gain no new styling: they are frozen, and a graphic that needs more is a page
([Web pages](#web-pages)). Polygons, stars and shadows are not
planned at all: each is a drawing program growing inside a video editor.

#### A line that draws itself on, and dashes that move

```json asset
{ "id": "broker-to-kafka", "kind": "shape",
  "shape": {
    "geometry": { "arrow": { "from": { "x": 0.30, "y": 0.40 },
                             "to":   { "x": 0.62, "y": 0.62 }, "curve": "s" } },
    "stroke": "#ffffffff",
    "stroke_width": 0.004,
    "dash": [0.02, 0.012]
  } }
```

```json clip
{ "id": "c-broker-to-kafka", "asset": "broker-to-kafka", "start": 30, "duration": 90,
  "keyframes": [
    { "property": "shape.trim_end", "keyframes": [
        { "t": 0, "value": 0.0, "easing": "ease_out" },
        { "t": 20, "value": 1.0 } ] },
    { "property": "shape.dash_offset", "keyframes": [
        { "t": 0, "value": 0.0 },
        { "t": 90, "value": 0.3 } ] }
  ] }
```

A diagram whose connectors are on screen whole or not at all reads as a slide.
Two things a line can do make it read as motion instead, and both work on
**every shape's line** — an arrow, a box's border, an ellipse's.

**A trim draws part of the line.** `shape.trim_start` and `shape.trim_end` are
keyframed on the clip, each a fraction of the way along the outline **by
distance**: `trim_end` going `0 → 1` is the line drawing itself on from its
start, and `trim_start` going `0 → 1` afterwards erases it forward, which is the
other half of the classic draw-on. By distance, not by how the curve happens to
be built — half way along an S is half its length, so a line draws on at the
speed its keyframes say. With neither keyframed the whole line is drawn.

A trim outside `0`–`1` is **clamped, not refused**: an easing that overshoots
is an ordinary animation and must not fail a render. `trim_end` at or before
`trim_start` draws no line at all.

Where the distance is measured from is where the outline starts: an arrow at
`from`, running to `to`; a rectangle at its **top-left corner** (a rounded one
where its top edge leaves that corner's curve); an ellipse at its **rightmost
point**. Both closed outlines run **clockwise**.

**An arrow's head rides the trimmed end.** While the line is part drawn, the
head sits on the end of what has been drawn, aimed along the line there, so an
arrow drawing itself on is led by its head — and a head is never waiting at
`to` for a line that has not reached it. A head at `from` does the same at the
trimmed start. Trimmed to nothing, there is no line and no head. The head is
full size from the first frame it appears.

**A dash breaks the line into pieces.** `dash` is a pattern on the shape —
lengths along the line, on, off, on, off…, each a fraction of the frame's
**height** like `stroke_width`. An odd count is read twice over, so `[0.02]` is
dashes and gaps of one length. It has to hold at least one length and each has
to be above zero. It is not animated; **where it sits is**: `shape.dash_offset`,
in the same unit, is how far the pattern has moved along the line **toward its
end**. Keyframing it upward is "marching ants" — a connector whose dashes flow
from its tail to its head, the standard way of saying *data moves along here*.
A pattern moved by its own length looks exactly as it started.

The two combine. A dashed line drawing itself on lays each dash down where it
will stay rather than sliding the pattern along as it grows.

**Both act on the line only.** A filled box whose border is trimmed or dashed
keeps its whole fill — a trim says how much of the *line* has been drawn, and a
fill has no line to be part way along.

A shape with its line keyframed is drawn again on every frame, where any other
shape is drawn once for as long as nothing about it changes. That costs one
shape's worth of drawing per frame, which is nothing next to decoding a video.

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

### Group assets

```json asset
{ "id": "pipeline", "kind": "group",
  "group": { "tracks": [
    { "id": "pipeline-boxes", "kind": "video", "clips": [
      { "id": "c-ingest", "asset": "box-a", "start": 0, "duration": 180,
        "keyframes": [{ "property": "transform.position.x", "keyframes": [
          { "t": 0, "value": -0.25 } ] }] },
      { "id": "c-store", "asset": "box-b", "start": 30, "duration": 150,
        "keyframes": [{ "property": "transform.position.x", "keyframes": [
          { "t": 0, "value": 0.25 } ] }] } ] },
    { "id": "pipeline-links", "kind": "video", "clips": [
      { "id": "c-link", "asset": "a-to-b", "start": 30, "duration": 150 } ] }
  ] } }
```

Several clips that render as **one layer**, so the layer can be moved, scaled,
faded or blurred as a unit — Filmora's *compound clip*. A diagram of thirty
boxes, arrows and captions that pulls back as a whole would otherwise be thirty
clips each carrying the same scale and position keyframes, worked out by hand
to stay in formation and rewritten, every one, the first time the move changes.
Grouped, it is one clip with two keyframes:

```json clip
{ "id": "c-pipeline", "asset": "pipeline", "start": 240, "duration": 180,
  "keyframes": [
    { "property": "transform.scale.x", "keyframes": [
        { "t": 0, "value": 1.0, "easing": "ease_in_out" }, { "t": 90, "value": 0.6 } ] },
    { "property": "transform.scale.y", "keyframes": [
        { "t": 0, "value": 1.0, "easing": "ease_in_out" }, { "t": 90, "value": 0.6 } ] }
  ] }
```

The fifth kind with no file behind it. `group` is required on this kind and
refused on every other, and it holds `tracks` — the same shape as the
document's own, **first at the bottom**, and all of them `video`. Their clips
name assets in the project's one assets table, by id, like any other clip; a
group carries placements, never media. A member may be a clip of another group,
which is how groups nest. Nobody has to write one by hand: `clip_group` over MCP
wraps clips already on the timeline into a new group and puts one clip of it
where they were, and `clip_ungroup` is the inverse — see
[`mcp.md`](mcp.md).

**Why an asset kind, and not a `parent` on clips.** A parent pointer gives a
transform that children inherit, and nothing else: every child is still its own
layer, so a group at half opacity would show its overlapping members through
each other, and there would be no single picture for a blur — or a glow, or a
mask — to act on. A group asset is a nested composition that renders **once per
frame, to one layer**, and a clip of it then composites like any other: its
`opacity`, transforms, `grade`, `blur` and the rest apply to the finished group.
It also has a clock of its own, which a pointer does not, so a group can be
trimmed and reused like a shot.

**Time.** The group's tracks run from the group's own zero, and a clip of it is
a window onto them exactly as a clip of footage is: `source_in` is the group
frame it opens on, and `duration` how much of the group it shows. So every
member's `start` — and, since keyframes count from a clip's start, every
member's animation — is in **group** time, and plays at the same moment of the
group wherever the group clip is placed or trimmed. A group clip plays at one
group frame per timeline frame: a `speed` on one is refused, because at any
other rate a group frame, and every member's keyframes with it, would fall
between two timeline frames. Retime the members instead.

**A group's length is derived, never declared**: it is where its last member
ends. A declared length would be one more number to disagree with what it
describes, and neither disagreement reads well — shorter hides members nobody
deleted, longer is empty picture nobody drew. So a group bounds a clip of it the
way footage does, and a clip may not play past the group's end.

**Space.** Members are laid out on the **project's raster** — the frame the
render is — so a fraction means the same inside a group and outside it, and a
box copied into a group sits where it sat. The group clip's transform then
applies to the rendered layer as a whole: its `origin` is the centre of the
frame unless the clip names another point, so a group scaled down shrinks
toward the middle of the picture, and one scaled from `{ "x": "left" }` shrinks
toward the left edge. `anchor`, `fit` and `crop` are not read on a group clip,
for the reason they are not read on a colour: the layer *is* the raster, so
there is nothing to fit and no edge to rest. The rectangle an arrow attached to
the group clip meets is that raster too, moved by the group clip's transform —
members move about inside it, so no smaller rectangle holds for the whole of
the group.

**Picture only.** A group's tracks are video tracks. Sound has no layer to be
part of, and mixing members' audio through a group's clock is a feature of its
own rather than something to half-do here. A member whose file has sound on it
is drawn and not heard, and the render says so in a note; put the same file on
an audio track beside the group clip to hear it.

**Ids are one namespace for the whole document.** A member's clip id may not
also name a clip on the timeline or in another group, and a group's track ids
are unique the same way — arrows name clips by id, and a render report names
them, so an id has to name one thing.

What validation refuses about groups, all from the document alone: a `group`
block missing from a group or present on anything else; a group with **no clip**
in it (a layer that can never show anything looks exactly like a render that
failed); an **audio track** in one; a group that **contains itself**, directly
or through another group; a clip of a group at a `speed`; and an **arrow
attached across a group's edge**. What it checks about the members is what it
checks about any clip — their assets resolve, their kind suits a video track,
they do not overlap on their track, their keyframes are well formed.

**How it renders.** Once per frame the members are composited, bottom track
first, into a transparent raster the size of the frame; that raster is then the
group clip's picture, and is composited with the group clip's own properties
like any other layer. Nothing is cached yet: a group costs one extra offscreen
composite a frame on top of what its members cost anyway.

Not here: groups with their own frame rate or raster, and editing inside a
group with the place, trim and move tools — those reach the project's own
tracks. A member is edited by ungrouping, editing and grouping again, or in the
document itself. A template carries a group whole: saving a group clip as a
template brings the group and everything its members show, nested groups
included, and inserting it renames the members' clip ids and the group's track
ids wherever the project already uses them.

### Image sequences

```json asset
{ "id": "spin", "kind": "image_sequence",
  "sequence": { "stills": ["spin-0001", "spin-0002", "spin-0003"], "hold": 2, "loop": true } }
```

Stills played in order, each held for a number of frames, once or round and
round: a directory of frames a renderer or an upscaler wrote, a timelapse of
four hundred photographs, stop motion, a flickering sign, a few drawings
cycling. A **picture that carries its own timeline** — the same idea as an
animated gif under `image`, arriving from the other direction.

| field | what it is | default |
| --- | --- | --- |
| `stills` | the `image` assets it plays, in order, **by id** — one may appear more than once | required, at least one |
| `hold` | how many **timeline** frames each still stays on screen; one number for the whole sequence | `1` |
| `loop` | start again from the first still when it runs out, instead of holding the last | `false` |

**Its length is never written down.** It is the stills times the hold, and a
duration stored beside them could only ever disagree — the same reason an
animated gif's length is measured rather than recorded. A clip of a sequence is
a window onto that timeline, like a clip of footage: `source_in` skips into it,
a shorter clip shows less of it, and `speed` plays it faster. **Past its end** a
looping sequence starts again, and one that does not **holds its last still**
for as long as the clip lasts — never a hole in the middle of a timeline. So a
sequence bounds no clip: there is nothing to trim past.

**The stills are assets, named by id**, as a clip names its asset: each is an
imported picture, hashed and probed like any other, so the missing-file check,
relinking and a hosted library all look after them with nothing added. Every
still is an `image` (not a sketch, not another sequence), all in **one format**
— `png`, `jpg`, `bmp`, `tif` or `webp`, since a sequence is decoded as one
stream and a gif or avif can carry an animation of its own — and, where they
have been measured, all **one size**: a clip fits the sequence to the frame once,
from its first still.

`scorsese sequence import <folder>` — or `import` with `"sequence": true` over
MCP — brings a folder of frames in as one: each frame an `image` asset under
`assets/<sequence>/`, with the sequence's id as a prefix, played in the order
their **numbers** say (`frame_9` before `frame_10`). A gap in the numbering is
reported and never refused, because a frame missing from a numbered run is
nearly always one somebody deleted on purpose. `scorsese sequence set` (and the
`sequence` tool) makes a sequence from stills already in the pool or changes
one's stills, `hold` or `loop`.

**How it renders.** Which still each output frame shows is worked out before
ffmpeg starts — the position in the sequence at that instant, through the
clip's speed, on the timeline's grid — and ffmpeg is handed that list to decode,
one file per frame, rather than being asked to time anything itself. Not here:
a hold per still, tweening between stills, and anything that picks a still from
a sound (lip sync) — that last one is character-animation software.

### Web pages

```json asset
{ "id": "title", "kind": "html", "path": "pages/title.html" }
```

A web page the project carries, played as a **moving picture with alpha**: a
title, a lower third, an animated graphic built from HTML and CSS. Placed by a
clip on a video track like any other picture, and everything a clip does —
transform, opacity, blur, mattes, groups, keyframes — applies to it unchanged.
Where the page draws nothing, the tracks below show through.

**The page is a file under `pages/`**, the directory beside `recipes/`, and for
the same reason: it is a document somebody wrote, so deleting one loses work.
Its `path` is relative to the project root like every path and must end in
`.html`. The page may load other files **inside the project** — a picture
(`../assets/photo.png`), a font file the project carries — and never anything
outside it, so a project with pages in it still survives `scp -r`.

**Authored, not generated.** An `html` asset has no `prompt`, no `recipe`, no
`state` and no sketch lifecycle: drawing a page costs nothing, so there is no
money for a sketch to save. The frames drawn from it are rebuildable cache, not
`generated/` output. Import records no `sha256` for it either — a page is meant
to be edited, and a recorded hash would call every edit a changed file — and no
`media`, since ffprobe has nothing to read off a document; it is never probed.
Collecting unused assets drops a page's row and **leaves its file** in
`pages/`, as a recipe is left in `recipes/`.

**Its length is the clip's.** Like a still, a page has no duration of its own,
so it bounds no clip and a shorter clip is a shorter page, never a faster one.
The page has a clock of its own, and it runs like footage's: at the clip's first
frame it reads `source_in` (in seconds), and it advances `speed` seconds per
second. The page is told where the clip ends on that clock — `source_in` plus
`duration` × `speed` — so an animation timed to end at the length it is told
ends on the clip's last frame.

`scorsese page <id> <file>` (and the `page_write` tool) writes one in place,
making the asset when the id is new; **[`pages.md`](pages.md) (`guide pages`) is
how to write one well**. `scorsese import page.html` (and the `import` tool)
copies a page into `pages/` as an `html` asset. Pages come in one at a time: a
directory import passes them over, because a page is a document rather than media.

**How it renders: captured by a headless browser, then played like footage.**
Before a render draws, each page clip is captured, or found already captured in
`cache/pages/`. The pinned `chrome-headless-shell` draws the page frame by frame
at the render's raster and frame rate into a lossless video with alpha — only
the frames the clip shows, from where it enters the page to the length the page
is told: the frames before are run without being drawn, so the page arrives in
the state it would have reached from clock 0, and a long stretch is drawn in
pieces at once (#809). A `still` draws only the few frames around its instant.
From then on the clip decodes like any video with alpha, starting at
`source_in` and running at `speed`.

**The browser needs no setup** (#776). It is looked for in one order:
`SCORSESE_CHROME`, then the pinned build this machine downloaded (beside the
settings file, `docs/credentials.md`), then `chrome-headless-shell` on `PATH`.
When none is there, the CLI, the MCP server and the desktop app **download the
pinned build the first time a page needs drawing** — about 100–150 MB, once,
verified against the pin's sha256 — and say so in one line: the CLI on stderr,
the MCP server in the reply that follows, the window under its preview. A
download that fails, offline say, leaves the clip as its slug card with the
reason on the report, never a failed render. The hosted server never downloads
one. `tools/chromium/fetch` downloads the same build for a developer and prints
the path to put in `SCORSESE_CHROME`.

The browser is headless: it never opens a window, and it runs only while a page
is being captured. **The desktop app's preview never waits for one**: it shows a
page's slug card until the page is captured in the background, at the preview's
own raster, and then draws it from the capture.

A capture is cached by everything that changes its pixels: the page and every
file it loaded, the raster, the frame rate, the length, the browser's build and
the capture method's own version. A page edited, or a picture it shows replaced,
is captured again. Anything else is reused.

What the page can count on:

- **`window.scorsese`**, set before any of its scripts run: `width` and
  `height` (its viewport in CSS pixels), `fps`, and `duration` (in seconds,
  where its clock is at the clip's last frame).
- **A viewport whose shorter side is 1080 CSS pixels**, whatever the raster.
  A page written for 1920 × 1080 lays out the same in a small preview and a 4K
  delivery, only softer or sharper.
- **A clock that only moves when a frame is drawn.** `performance.now()`,
  `Date`, `setTimeout`/`setInterval`, `requestAnimationFrame` and every CSS
  animation, transition and Web Animation all follow the clip. Each frame is a
  pure function of its time.
- **A transparent background.** Where the page draws nothing, the tracks below
  show through.
- **The shipped faces, by name** (`font-family: Inter`, `"Playfair Display"`,
  and the rest of the catalogue), plus any font file the project carries, loaded
  with `@font-face`. A font it names and was not given falls back to a shipped
  face, the same one on every machine.
- **scorsese's motion kit**, at `https://lib.scorsese/kit.js`: a frame loop on
  the page's own seconds, easings, entrances and exits, a count-up, a seeded
  random (`guide pages`, section "The kit").
- **anime.js 3.2.2**, at `https://lib.scorsese/anime.min.js`, and
  **lottie-web 5.13.0**, at `https://lib.scorsese/lottie.min.js`, for a Lottie
  file the page loads (`guide pages`, section "A Lottie animation").

**Pages render offline.** The page is served from `https://page.scorsese/`,
whose paths are the project's, so `../assets/photo.png` from `pages/` is the
project's `assets/photo.png`. Any other request is refused, whether to another
host or to a path outside the project, and a WebSocket or WebRTC connection
reaches nothing. A refused request or connection, a file the page asked
for that is not there, a script that threw, or text laid out off the frame,
out of its box or over other text becomes a **warning on the render**. The page
is drawn without whatever it was, or as it was laid out.

**Not covered:** `<video>` and `<audio>` inside a page (they play on their own
clock; a warning says so, so put footage and sound on the timeline), workers'
clocks, and `requestIdleCallback`.

**A page that cannot be captured shows its slug card**, a translucent band
across the foot of the frame naming the page, so the shot below stays visible.
The reason goes on the report. That is a stand-in and never a failed render:
the usual cause is no browser on the machine. Capturing on Windows is not
supported yet (#797).

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
render will use, or judge everything about it except its size. The editors'
own previews — the desktop app's picture and the web editor's preview video —
are not caught by this: a preview *quality* (#542) is a fraction of a
1080p-class delivery, and it shrinks a `native` layer by the same fraction, so
every quality shows that delivery's layout with fewer pixels.

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

**It is the point `transform.scale`, `transform.rotation` and `transform.flip`
pivot on.** That
is the clip above: a gold rule along the foot of the frame that fills from the
left over sixty seconds, as **one** keyframe track. Without an origin, scale
turns about the middle, so the same bar needs a second track sliding it left by
`(s − 1) / 2` on every frame — a number nobody can read back as *the left edge
stays put*, and one that is only right while the scale is linear in time. Put an
`ease_out` on the scale and the two come apart: the bar slides while it grows.
The document still validates and the render still succeeds; the only symptom is
watching it.

**Scale, rotation and flip alike.** A card hinging on its left edge is the same
request as a bar filling from it, and one pivot for all three is the coherent
reading of *the point the layer turns about*. A second field for rotation alone
would be a near-synonym the format cannot afford. A flip is included because a
flip *is* a foreshortening — the same part of the same matrix a scale uses — so
`transform.flip.y` about a `left` origin is the page-turn hinged on the layer's
left edge rather than swung about its own spine.

**`transform.position` is applied after all of them and is unaffected.** A pivot
cannot move a layer that is not being scaled, turned or flipped, which is what
makes the field free to set on a clip that is only being placed.

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
  "grade": { "saturation": 0.18, "temperature": 0.12, "vignette": 0.35,
             "grain": 0.08 } }
```

Six numbers, all optional, every default the **neutral** one — so a grade that
says nothing changes nothing, and a clip with no `grade` at all is the clip
exactly as it arrived.

| property | neutral | which way it runs |
| --- | --- | --- |
| `saturation` | `1.0` | `0.0` is fully grey; above `1.0` oversaturates |
| `temperature` | `0.0` | negative is cooler (blue), positive is warmer (amber) |
| `brightness` | `0.0` | negative darkens, positive lightens |
| `contrast` | `1.0` | below `1.0` flattens, above `1.0` steepens |
| `vignette` | `0.0` | `0.0` is none; higher darkens the corners further |
| `grain` | `0.0` | `0.0` is a perfectly clean picture; `1.0` is the heaviest |

**It applies to every layer kind, not only video.** The compositor does not care
what produced the pixels it is handed, and a title card that does not warm along
with the shot behind it is the wrong result. An image, a text layer and a
generated shot all take a grade on the same terms.

**Numbers, never named looks.** The generality rule — core defines property
*types*, never property *values* — rules out a `"look": "70s"` field here as
surely as it rules out "make text red". `saturation: 0.18` is a property; the
70s look is five values somebody chose, and those belong in a project, a
document, or an assistant's suggestion.

**A field *and* six animatable properties**, which no other clip property is.
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
then brightness, then vignette, then grain. Saturation runs first so that
warming a desaturated shot warms it rather than being undone by the
desaturation; brightness runs after contrast so that the contrast knob does not
shift the exposure as a side effect; grain runs last because it is the emulsion
and not the scene, so the amount asked for is the amount seen rather than an
amount the contrast knob has since steepened. Everything is clamped to the
displayable range on the way out — an overshoot is a blown highlight, not a
wrapped one.

The vignette is measured from **the layer's own centre**, not the frame's, so
one on a picture-in-picture darkens that picture's corners rather than a ring it
happens to sit inside. Alpha is never touched, by the vignette or anything else:
a grade changes what colour a pixel is, never how much of it there is.

**`grain` is the one of the six that moves.** A picture can be warmed,
flattened, darkened, softened and vignetted and still read as computed, because
a photochemical image is never clean and a computed one always is; grain is what
closes that gap, and it does a second job nothing else does — a common grain
over a whole cut is the oldest way of making shots that were generated or
sourced separately sit together. It is **monochrome**, because silver halide
crystals are not coloured and independent noise per channel is the speckle of a
video sensor rather than the texture of film; and it is **strongest through the
midtones**, falling away to nothing at both ends of the range, because uniform
noise in the blown highlights and the crushed blacks is exactly what reads as
noise added in post. Its size is a fraction of the layer's own height, like
`blur`, so the same number is the same texture at 1080p and at 4K. Sensible
values are small: `0.05` is a clean stock, `0.15` is heavy, and `1.0` is the
most the field will do.

**Nothing seeds it, deliberately.** The noise field is derived from the clip's
own `id` and the frame the clip has reached, so two clips of the same footage
never carry the same grain, and the pattern **animates** rather than sitting
still like dirt on the lens. It is a hash and never a generator: the same
project renders the same grain on every machine and in every run, a frame never
depends on a frame rendered before it, and rendering a `--range` out of the
middle of a timeline gives the same frames the whole render would have. A seed
in the document would be a number nobody could choose meaningfully and everybody
would copy by accident.

**Grain costs bitrate, and a lossy delivery will eat some of it.** Noise is the
most expensive thing there is to encode, so what reaches a viewer is always less
grain than the compositor drew — at a low bitrate, or a light amount, possibly
none of it. That is a fact about video compression rather than about this field:
if a render comes back cleaner than the preview looked, the answer is a higher
bitrate or a larger number, and `docs/golden-renders.md` has the measurements
that make the same point about the pixel gate.

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

It is `blur` and not `grade.blur` deliberately. **The test for belonging to a
grade is one pixel in, one pixel out**, and every field of one passes it — a
`vignette` also consults where the pixel is and `grain` also consults which
frame it is, and both still write one pixel from one pixel. A blur reads a
neighbourhood, which is a different shape of operation and belongs beside a
grade rather than inside one.

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

### What the lens did to it: `aberration`

```json clip
{ "id": "c-rooftop", "asset": "shot-a", "start": 0, "duration": 120, "aberration": 0.0015 }
```

Radial chromatic aberration: red and blue sampled at slightly different
distances from the layer's own centre than green, so the three channels separate
as they move away from the middle of frame. `0.0` — and an absent field — is a
picture no glass ever touched.

**This is the other half of `grade.grain`.** Grain is the film; this is the
lens. Every real lens splits the channels a little, because it does not bring
every wavelength to the same place, and the total absence of that is one of the
quiet tells of an image that was computed rather than photographed. Ordinary
values are small — `0.001` to `0.003` on a full-frame plate is a fringe you feel
rather than see, which is the point of it.

**Zero at the centre, worst at the edges**, which is what makes it read as a
lens rather than as a misregistration. The displacement grows in proportion to
how far a pixel sits from the layer's own centre, so a title in the middle of
frame is untouched by the same number that fringes the corners.

**The unit is a fraction of the layer's own height**, measured at its top and
bottom edges — half the height out from the centre, which is where the height is
the thing to measure against. `0.001` on a 1080-tall layer moves red about a
pixel there and blue about a pixel the other way; at the corners of a 16:9 frame
it is roughly twice that, because the corners are further out. The same number
is the same fringing at 1080p and at 4K, for the reason `blur` is a fraction too.
Anything under half a pixel everywhere on the raster is nothing, and costs
nothing.

**Red goes outward and blue inward**, stated here so nobody has to render a
frame to find out: red is sampled *nearer* the centre than the pixel it lands
on, so a bright edge picks up a warm fringe on its outer side and a cool one on
its inner side. Green does not move at all — the conventional choice, and the
one that keeps the picture from softening or shifting as a whole while its
colours separate.

**One number**, deliberately. Independent per-channel amounts, the
lateral/longitudinal distinction, and lens distortion of any kind — barrel,
pincushion, anything needing to know a focal length — are a compositing suite's
answers to a question nobody editing a video asks.

**A field *and* an animatable property**, like `blur` and `grade`: the field is
the clip's baseline and an `aberration` keyframe track takes it over for the
whole clip.

It is `aberration` and not `grade.aberration` for the reason `blur` is not
`grade.blur`, and more plainly than blur has it. **The test for belonging to a
grade is one pixel in, one pixel out**; this reads *three* source pixels — one
per channel, from three different places — to write one, so it fails that test
as squarely as a neighbourhood does and sits beside a grade rather than inside
one.

**It runs after the blur, and a clip carrying both gets both.** Which of the two
runs first was measured rather than argued and turns out not to matter: a
ten-pixel blur and a two-pixel split on the same plate come out within two levels
of 255 of each other whichever order they run in, because a blur is a linear
filter and the split is locally a shift, and those commute. So neither knob eats
the other, and turning the blur up does not quietly delete the fringing.

**It applies to every layer kind**, for the reason a grade and a blur do: the
compositor is handed a rectangle of pixels and does not know what produced them.

### Taking the background out: `chroma_key`

```json clip
{ "id": "c-host", "asset": "shot-b", "start": 0, "duration": 240,
  "chroma_key": { "color": "#00b140", "tolerance": 0.25, "softness": 0.1, "spill": true } }
```

A chroma key: the colour of the screen the shot was filmed against, how far
from it still counts as screen, how soft the edge is, and whether the colour
the screen bounced back onto the subject is pulled off again. It is **the only
thing in scorsese that can make a source's own pixels transparent** — every
other property answers what colour a pixel is, and this one answers whether
there is a pixel at all.

Absent — and it is absent on nearly every clip — means no key. That is the only
neutral there is: a key needs a colour, and there is no colour that means "do
not key", which is why this is one object rather than four fields with
harmless defaults.

**Four values, and that is the whole feature.** Garbage mattes, animated masks
or splines, light wrap, per-channel despill controls and combined
luma-plus-chroma mattes are a compositing suite's answers to a question nobody
editing a video asks.

**`color` is the colour the camera saw**, not the colour the screen was sold
as. A lit green screen is never `#00ff00` — it is something like `#00b140`, and
under a warm key light something else again — so sample it from a frame rather
than guessing. Its alpha is ignored; a recorded colour is opaque, and the
notation carries an alpha because one notation reads every colour in this
document.

**`tolerance` and `softness` are the two thresholds every keyer has.** Below the
tolerance a pixel is entirely gone; past tolerance plus softness it is entirely
kept; between them the alpha ramps straight down. `0.0` softness is a hard
cutout, which reads as a paper doll on anything with a soft edge — a little is
what lets hair and motion blur survive.

**What they are a distance in**, because a number needs a scale. The key
measures colour with the light divided out: each pixel is reduced to the
proportions of its three channels, and those are laid out as a triangle with
white at the centre, **each pure primary exactly `1.0` from white**, and two
primaries `√3` apart. So `0.25` is a quarter of the way from a pure primary to
neutral grey. For orientation, against a `#00b140` screen: skin sits about
`0.70` away, and a strand of hair three-quarters covered by the screen sits
about `0.29`.

**It survives an unevenly lit screen, and that is what the plane is for.** A
real screen is one chroma and many lumas — brighter where the lights point,
darker in the subject's shadow — and dividing the light out is exactly what
makes those the same point. The same `#00b140` at full light, at 42% and at 20%
sits `0.003` and `0.009` from itself here, where a plain RGB distance would put
them `0.41` and `0.53` apart and one key could not take both.

**A pixel with almost no light in it is kept**, because proportions of nearly
nothing are noise rather than a colour. The shadow so deep it stays opaque is a
mark on the matte somebody can see and fix; the alternative speckles the
subject.

**`spill` is one boolean.** With it on, the amount of the keyed hue a pixel
carries beyond what its other channels account for is taken back off it, and
the pixel is then put back at the brightness it had — because green carries 72%
of perceived luma, and a despill that skips that step leaves a grey rim where
there was a green one. It is about the hue that was keyed and not about green:
a magenta screen spills magenta and the same boolean pulls red and blue down
instead.

It costs something, and the cost is worth knowing: the suppression applies to
the whole layer, so a genuinely green jacket in front of a green screen comes
back less green. That is the price of one boolean, and turning it off is the
escape hatch.

**`tolerance` and `softness` are animatable**, as `chroma_key.tolerance` and
`chroma_key.softness` — the field is the clip's baseline and a track takes that
one setting over for the whole clip, exactly as `grade.*` does. `color` and
`spill` are not: a colour is not a number, and a boolean ramped halfway is not a
state anything is in. **A track on either does nothing on a clip with no
`chroma_key`**, because there is no key for it to be a setting of.

**The key runs first**, before the grade and so before everything else. Grading
first shifts the screen's own colour and a key aimed at what it used to be then
misses it — a matte that quietly stops working the moment somebody warms the
shot.

**Footage only.** A `text`, `color`, `shape` or `icon` asset is drawn by this
build with exactly the alpha the document asked for, so a key on one asks it to
undo its own drawing; validation refuses it. And **the screen colour must have a
hue** — its strongest and weakest channels at least 16 levels apart — because a
key on a grey separates pixels by brightness alone. That is a luma key, and
scorsese deliberately does not have one: knocking out a white or a black
background also knocks out every white and black *in the picture* — eyes, teeth,
the shine on hair. The answer to "I want a cutout" is to shoot or generate
against a saturated colour.

### What the tape did to it: `vhs`

```json clip
{ "id": "c-rooftop", "asset": "shot-a", "start": 0, "duration": 120,
  "vhs": { "chroma_bleed": 0.4, "noise": 0.2, "scanlines": 0.3, "jitter": 0.1,
           "head_switch": 0.25 } }
```

The VHS look, computed. Five numbers and a mode, all five running `0.0` (none)
to `1.0` (the heaviest this offers), and an absent `vhs` — or `"vhs": {}` — is a
picture no tape ever held.

**Nothing is composited here.** No overlay footage, no noise plate, no imported
asset of any kind: every artefact below is arithmetic over the clip's own
pixels. That is what makes the look free, deterministic, and portable — a
project carrying it survives `scp -r`, and rendering it twice gives the same
picture.

| field | what it is | at `1.0` |
| --- | --- | --- |
| `chroma_bleed` | colour smeared sideways, as a fraction of the layer's **width** | a twelfth of the picture |
| `noise` | snow, on the colour as well as the brightness | the heaviest this offers |
| `scanlines` | how dark the alternate lines go | most of the way to black |
| `jitter` | the tracking wobble, as a fraction of the layer's **width** | a twentieth of the picture |
| `head_switch` | the torn band at the bottom, where the heads hand over | a twelfth of the height, torn a third of the width |
| `mono` | whether the chroma path is modelled at all | — |

**Why one named effect and not five loose ones.** Nobody assembles a VHS look
from first principles by accident: these five are always set together, and five
top-level properties that only ever move as a group are one thing wearing five
hats. And not one `vhs: 0.7` either — a single knob is unfixable the moment it
is wrong on a shot, because turning the wobble down turns the whole look down
with it. A named effect *with parameters* is the middle of those two, and it is
also what squares the generality rule with the Filmora one: every field here is
a property **type** with a neutral, and none of them is a value somebody chose.

**`chroma_bleed` runs one way only, and that is the point.** A tape recorded
colour at a small fraction of the brightness's bandwidth, so the colour arrives
*late*: it runs on past the right-hand side of an edge and the left-hand side
stays clean. A smear that fringed both sides equally is a lens, which is what
`aberration` already is, and the two do not look alike. The errors land on the
colour-difference axes rather than on red, green and blue independently, which
is where the green and magenta of a bad tape comes from.

**`noise` is the tape's snow and `grade.grain` is the film's emulsion**, and a
clip may carry both without them cancelling or doubling — each draws its own
noise field from the same instant. They differ in what they colour: grain is
monochrome, because silver halide is not coloured; tape snow speckles the
colour-difference channels too, which is why it reads as coloured.

**`jitter` is the one measurement in this format taken against the width.**
Everything else — `blur`, `aberration`, a `text` size — is a fraction of the
layer's height. A tracking error displaces a row *along itself*, so how far it
can go is bounded by how long the row is, and the width is the honest thing to
measure it against. It varies smoothly down the frame and changes every frame,
because a line read late is a fresh accident each time.

**`scanlines` is a count and not a pixel pitch.** The picture is divided into a
fixed number of line pairs top to bottom, so the same number reads as the same
texture at 480 lines and at 2160 — the same reasoning that makes a blur a
fraction rather than a count of pixels.

**`head_switch` sets both how tall the band is and how far it is torn**, because
a band nobody can see and a tear nobody can see are the same thing not
happening. The band loses its colour along with its position: what is left of
the signal where the heads hand over is not a picture with a cast, it is not a
picture.

**`mono` is a mode, not a preset**, and the difference is whether the chroma
path is modelled at all. With it there is colour to smear and colour to speckle,
and the result reads green and magenta. Without one the picture is grey and what
is left is snow, scanlines and a wobble — which is a different real artefact:
early black-and-white tape, or a security camera, rather than a rented film. It
is a field only and not animatable, for the reason `anchor` is one: it decides
what the other five *mean*, and a mode ramping between two readings over half a
second is not a thing anybody means.

**A field *and* five animatable properties**, like `grade`: the field is the
clip's baseline and a `vhs.jitter` track — or any of the other four — takes that
one number over for the whole clip. A tape that goes wrong for a second and
settles is one keyframe track.

**Softness and ringing are deliberately absent.** A tape is soft, and `blur`
already softens a clip honestly and composes with this on the same clip. What is
left of the sixth artefact is the *ringing* a sharpener leaves on either side of
an edge, which is a second filter for a halo most people would read as "somebody
sharpened this". Five knobs that each name an artefact anybody can point at is
worth more than six where the sixth needs explaining.

**It runs last of everything applied to the layer's own pixels** — after the
key, the grade, the blur and the aberration, and before the transform places it.
Those are what a camera did, or what was taken out of what it saw; a tape is what
held the result. Only a `shadow` and a `glow` come after it, because they are
grown from the finished picture rather than applied to it. It also applies to every layer kind,
for the reason a grade does: the compositor is handed a rectangle of pixels and
does not know whether a decoder, a title or a shape produced them.

**What is deliberately out of scope**: emulating a named tape format to spec,
dropout compensation, timebase-corrector modelling, and any imported overlay of
noise or tracking damage. The whole point is that this is computed.

### Light of its own: `shadow`, `glow` and `blend`

```json clip
{ "id": "c-link", "asset": "arrow", "start": 0, "duration": 120,
  "glow": { "radius": 0.04, "intensity": 3.0 }, "blend": "add" }
```

```json clip
{ "id": "c-vessel", "asset": "vessel-cutout", "start": 0, "duration": 120, "fit": "native",
  "shadow": { "color": "#000000", "offset_x": 0.02, "offset_y": 0.03, "softness": 0.04,
              "opacity": 0.6 } }
```

Three looks computed from the layer's own alpha. On a dark background a line, an
icon or a caption with no light of its own reads as flat vector art; the same
element glowing reads as a lit display. A cut-out picture with no shadow looks
pasted onto the frame; a soft contact shadow seats it. And overlapping glowing
dots should brighten each other rather than hide each other, which is what
`add` and `screen` are.

**`shadow`** is the layer's silhouette — its alpha, not its colours, so a red
ball and a blue one cast the same shadow — softened, tinted, offset, and drawn
**under** the layer. Every field is optional, and `"shadow": {}` is a shadow at
the defaults:

| field | what it is | default |
| --- | --- | --- |
| `color` | the shadow's colour; its alpha multiplies `opacity` | `#000000`, black |
| `offset_x` | how far right it falls, as a fraction of the layer's own **height**; negative is left | `0.01` |
| `offset_y` | how far down it falls, as a fraction of the layer's own **height**; negative is up | `0.01` |
| `softness` | how soft its edge is, measured exactly as `blur` is | `0.02` |
| `opacity` | how dark it is, `0.0` none to `1.0` the full colour | `0.5` |

**Both offsets are measured against the height**, like `softness` and `blur`,
so equal `x` and `y` fall at exactly 45° whatever the aspect of the layer — and
the same numbers are the same shadow at 1080p and at 4K. The offset is in the
layer's own pixels, so a layer that turns takes its shadow round with it and a
layer scaled up casts a longer one, the way CSS's `drop-shadow` behaves. An
offset past a whole height either way is held at one.

**`glow`** is a soft halo round whatever the layer draws: its alpha — and, by
default, its colours — blurred and drawn **under** it, centred. `"glow": {}` is
a glow at the defaults:

| field | what it is | default |
| --- | --- | --- |
| `color` | the light's colour; its alpha multiplies the halo | absent: **the layer's own colours** |
| `radius` | how far the halo reaches, measured exactly as `blur` is — and never further than the longest side of what the layer actually draws, the box round its non-transparent pixels | `0.02` |
| `intensity` | how bright it is: `0.0` none, `1.0` the layer's own light spread out | `1.0` |

Absent `color` means the layer's own, so a cyan line glows cyan and a two-colour
icon glows in both — what "make it glow" means before anybody has an opinion
about colour. **An intensity above `1.0` is more light than the layer has**, and
a thin line needs it: spreading a line two pixels wide over forty leaves very
little of it anywhere, so `2.0`–`4.0` is where a glowing stroke lives. The halo
reaches three radii past the layer's edge, because that is how far three passes
of `blur`'s kernel reach.

**`blend`** is how the layer lands on what is beneath it, and the list is four
words and closed:

| `blend` | what it does | over black |
| --- | --- | --- |
| `normal` | covers what is beneath, in proportion to the layer's alpha — what every clip has always done | itself |
| `add` | adds the layer's light to what is beneath, clipping at white | itself |
| `screen` | brightens what is beneath without ever passing white — a gentler `add` | itself |
| `multiply` | darkens what is beneath by the layer's colour: white changes nothing, black makes black | black |

An absent `blend` is `normal`. A transparent pixel blends to nothing in every
mode, so a shape's empty surround set to `add` does not light the frame. **`add`
or `screen` over nothing is exactly `normal`**, and `multiply` over nothing is
black — the layer disappears — so `scorsese check` warns about a blended clip
that is the lowest thing on screen for its whole length. Inside a group the
canvas beneath the lowest member is transparent rather than black, and there
every mode over nothing draws exactly as `normal`.

**Shadow, then glow, then the layer, as one picture — then the blend.** The
three are stacked source-over into a single picture of the layer, and only that
picture meets the frame, once, with the clip's `opacity` and its `blend`. So a
layer fading out takes its shadow with it without the shadow showing through the
layer, and an `add` layer adds its glow as light too. That is also why a shadow
on an `add` or `screen` layer is invisible over black, and should be: dark light
is no light.

**The order of operations**, which is the whole of what happens to a layer's
pixels before its transform places it:

1. `chroma_key` — before anything touches a colour, or the key misses the screen.
2. `grade` — the colours, on the layer's own pixels.
3. `blur` — its focus.
4. `aberration` — its glass.
5. `vhs` — the tape that held the result.
6. `shadow` and `glow` — **grown from the finished picture**, so a blurred layer
   casts a blurred shadow, a keyed one casts the shadow of what the key left, and
   a taped one a taped shadow.

Then the transform places the whole of it, a `matte` — when the clip has one —
decides where it shows, and the `blend` lands it.

**The picture grows to hold its light.** A halo reaches past the layer's own
edges, so the lit picture is padded out to that reach on every side before it is
placed — a cut-out picture at its `native` size casts its shadow onto the frame
beyond its own rectangle rather than having it cut off square at the old edge.
Where a layer *is* — what an arrow attaches to, where a layout says it landed
— is still the layer's own rectangle and never its halo: a shadow is not part of
the box an arrow should meet.

**On a group clip, it is the whole group's light.** A group is one picture by
the time it meets its shadow and glow, so one `glow` on the group clip lights
every member — thirty boxes and arrows glowing as one diagram — and one `shadow`
is the silhouette of the whole arrangement.

**Animatable: `shadow.opacity`, `glow.radius` and `glow.intensity`**, with the
field-plus-track bargain `chroma_key` makes: each does nothing on a clip with no
`shadow` or `glow` to take the number over, since a shadow needs a colour and an
offset and neither is a number a track can carry. Colours, offsets, `softness`
and `blend` are fields only. **Each is clamped where it is drawn**, which matters
now that an easing can overshoot: `shadow.opacity` to `0.0`–`1.0`, `glow.intensity`
to `0.0`–`4.0`, and `glow.radius` to nothing below zero and never wider than the
layer is tall — so a `spring` on a pulse flashes brighter and settles, rather
than going negative.

**A glow's radius is also held to what the layer draws.** A shape, a title or
an icon is drawn on a raster the size of the frame, so "a fraction of the
layer's height" alone would let `radius: 1` on a 23-pixel dot spread its light
over the whole frame — under a tenth of a level anywhere, which is no halo at
all. So the radius never exceeds the **longest side of the box round the layer's
non-transparent pixels**: a large radius on a small shape gives a halo about the
shape's own size, and a long thin line can still glow along its whole length.

**What is deliberately absent**: inner glow, bevel and emboss, strokes as an
effect, "layer styles" as a family, the other dozen blend modes, and anything
reading what is *behind* a layer beyond the blend itself. That is the
compositing-suite line.

### Revealed through another clip: `matte`

```json project
{
  "schema_version": 47,
  "name": "wipe",
  "timeline_fps": { "num": 30, "den": 1 },
  "assets": [
    { "id": "map", "kind": "color", "color": "#1a5fb4" },
    { "id": "wipe", "kind": "shape",
      "shape": { "geometry": { "rectangle": { "width": 1.0, "height": 1.0 } },
                 "fill": "#ffffffff" } }
  ],
  "tracks": [
    { "id": "v1", "kind": "video", "clips": [
      { "id": "c-map", "asset": "map", "start": 0, "duration": 90,
        "matte": { "clip": "c-wipe" } } ] },
    { "id": "v2", "kind": "video", "clips": [
      { "id": "c-wipe", "asset": "wipe", "start": 0, "duration": 90,
        "origin": { "x": "left", "y": "center" },
        "keyframes": [ { "property": "transform.scale.x", "keyframes": [
          { "t": 0, "value": 0.0, "easing": "ease_in_out" }, { "t": 30, "value": 1.0 } ] } ] } ] }
  ]
}
```

A **track matte**: the clip is shown only where another clip's picture is —
`c-map` above is revealed left to right as the `c-wipe` rectangle grows from its
left edge. Until a clip has one, anything can only *arrive* (fade, slide,
scale); a matte is how a picture is **revealed**: wiped in from an edge, opened
out of a circle, seen through the letters of a title.

| field | what it is | default |
| --- | --- | --- |
| `clip` | the id of the clip whose picture is the mask | required |
| `invert` | `true` shows the clip where the matte is **not** — a hole the matte's shape | `false` |

**The matte is another clip, not a shape of its own**, so everything a clip can
do, the mask does: a wipe is a rectangle whose `transform.scale.x` runs `0 → 1`
with an `origin` on one edge, an iris is an ellipse whose scale grows, footage
through a title is a `text` clip as the matte, a matte clip's own `blur` is a
feathered edge, its `opacity` fades the reveal, and a `group` clip is as good a
matte as a single shape. Keyframes, easings and groups all apply without a
property of their own.

**Alpha only.** Where the matte's picture is opaque the clip shows fully, where
it is transparent not at all, and a half-transparent edge is a half-revealed
edge. Its colours are never read — a white rectangle and a red one are the same
matte.

**Naming a clip as a matte is what makes it one: it is never drawn itself.**
Whether or not the clip it masks is on screen at that instant, a clip any clip
names in its `matte` is used only as a mask. There is no flag on the matte clip
saying so — a flag would be one more thing to disagree with the reference.
Remove the `matte` and the clip is drawn again.

**When the two meet.** The matte is drawn with its own transform, where it lands
on the frame, and the masked clip is drawn through it as the very last step —
after its own transform, and after its `shadow` and `glow`, so a matte reveals a
layer *with* its light. On a `group` clip it reveals the whole group. The full
order is key → grade → blur → aberration → tape → shadow & glow → transform →
**matte** → blend. Where the matte clip is not on screen, nothing shows the
masked clip — it is not drawn at all — and, inverted, nothing cuts anything out
of it. `scorsese check` warns about a clip whose matte is **never** on screen at
the same time as it, since that clip never appears.

**What validation refuses**, each with the reason:

- a matte naming a clip that is nowhere in the document, or the clip itself;
- a matte on the other side of a group's edge from the clip it masks — the two
  must be on the same timeline, both on the project's own tracks or both inside
  one group, for the reason an arrow across the edge is refused;
- either of the two on an audio track — a matte is picture;
- a matte that has a `matte` of its own. A chain of mattes is a compositing
  graph; draw the masks as one group and use the group clip instead.

A matte clip's sound, if it has any, is heard exactly as before: a matte decides
what is seen, and nothing about what is heard.

**What is deliberately absent**: luminance mattes, bezier and freehand masks,
masks animated vertex by vertex, and anything tracked. That is the rotoscoping
line.

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

### Travelling along an arrow: `follow`

Between two keyframes `transform.position.x` and `.y` interpolate on their own,
so a clip moved by position alone always travels in a **straight line**. To send
one along a curve — a packet down a bowed connector, a boat along a route — the
clip names an **arrow clip** as its path, and the `follow.progress` keyframe
track says how far along it is:

```json clip
{ "id": "c-packet", "asset": "dot", "start": 30, "duration": 60,
  "follow": { "clip": "c-arrow-broker-kafka", "orient": true },
  "keyframes": [ { "property": "follow.progress",
                   "keyframes": [ { "t": 0, "value": 0.0, "easing": "ease_in_out" },
                                  { "t": 45, "value": 1.0 } ] } ] }
```

- **`clip`** names the arrow's **clip**, never its asset. An attached arrow is
  only resolved per frame and per clip, and one asset can be on screen twice, so
  a clip is the one thing that names a single line on the frame. It must show an
  arrow shape, sit on the same timeline as the follower — both inside one group,
  or both outside every group — and not be the follower itself.
- **`follow.progress`** is a fraction of the arrow's **length**, measured along
  the line and not along the curve's own parameter: `0` is its tail (`from`),
  `1` its head (`to`), and `0.5` is half the distance whatever the bow — so an
  S-shaped connector is travelled at an even pace. It is **clamped to `0`–`1`**:
  an overshooting easing (`back_out`, `spring`) rests at the end rather than
  running off it, as a trim does. Nothing about it is stored in `follow`; like
  `shape.trim_end`, a held value is one keyframe.
- **`orient`** (default `false`) turns the clip to face the way the line runs at
  its place on it — draw the clip pointing right, and it points along the arrow.

**What the path does to the transform.** The path places the point of the
clip's *content* that its `origin` names — the dot's middle, by default — and
then the clip's own transform carries on as before:

- `transform.position` is an **offset** from the point on the line, so a small
  bob keyframed on top rides along with the path;
- with `orient`, the line's heading is **added** to `transform.rotation`;
- scale, opacity, blur and the rest are untouched.

**The path is the arrow as drawn**, including the arrow clip's own transform: an
arrow that is moved, turned or mirrored is followed where it lands. It is the
whole line whatever its `shape.trim_*` — a connector drawing itself on does not
drag its follower with it; key both tracks together for that.

**The arrow need not be on screen.** Its line is geometry, so a dot may set off
along a connector before the connector appears, or after it has gone. The one
exception is an arrow with an **attached** end: where it runs is known only
while it and the clips it points at are drawn, so while it is not, its follower
is left out and the render says so — the rule an attached arrow with nothing to
point at already follows.

**One level deep.** The arrow a clip follows may not be attached to a clip that
itself follows a path: a path moving because another path moved is a chain, and
a dot following the arrow attached to that same dot is a loop. Validation
refuses it; attach the arrow to something placed by its own transform instead.

`scorsese check` warns about a clip with a `follow` and no `follow.progress`
track — it renders, and sits at the arrow's tail the whole time.

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
corners sit, to 1/30s, which is well below audible for a fade. `easing`
belongs to the keyframe the value travels **from** — so one written on the last
keyframe of a track does nothing at all, and the ramp into it is the straight
line it would have been anyway.

`property` is a dotted string — `opacity`, `transform.position.x`, `volume`
— and core does **not** check that it names a property that exists. That is
the generality rule: core defines property types, never property values. The
compositor resolves paths; adding a new animatable property costs nothing
here.

### Easing

| `easing` | the value… |
| --- | --- |
| `linear` (default) | travels at one rate throughout |
| `ease_in` | starts slow and accelerates into the next keyframe |
| `ease_out` | starts at full rate and settles as it arrives |
| `ease_in_out` | is slow at both ends and quickest in the middle |
| `hold` | stays put, then jumps on the next keyframe |
| `back_in` | pulls back the other way by about a tenth of the move, then accelerates in — a wind-up |
| `back_out` | arrives fast, **overshoots** by about a tenth of the move and settles back — the pop of a title landing |
| `back_in_out` | winds up, crosses quickly, **overshoots** and settles back |
| `spring` | **overshoots** by about a sixth, swings slightly back under, and comes to rest on the keyframe |
| `{ "cubic_bezier": [x1, y1, x2, y2] }` | follows the CSS `cubic-bezier()` curve with those handles |

Every preset is a bare word. A cubic bezier carries its four numbers, exactly
as CSS writes them — `[0.25, 0.1, 0.25, 1]` is CSS's `ease` and
`[0.34, 1.56, 0.64, 1]` a gentle back-out — so a curve copied from a stylesheet
or an easing site moves the same here:

```json clip
{
  "id": "c-card", "asset": "title", "start": 0, "duration": 90,
  "keyframes": [
    { "property": "transform.scale.x", "keyframes": [
        { "t": 0, "value": 0.6, "easing": { "cubic_bezier": [0.34, 1.56, 0.64, 1] } },
        { "t": 12, "value": 1.0 }
    ]}
  ]
}
```

`x1` and `x2` must lie in `0..=1` — outside it the curve runs time backwards
and validation refuses it. `y1` and `y2` may be anything, and outside `0..=1`
is how a bezier overshoots.

**An overshoot goes past the keyframe's value, and nothing clamps it on the
way.** A scale eased `0.6 → 1.0` on `back_out` reaches about `1.04` before it
settles; that value was never written, and it is the point. Scale, position,
rotation and flip overshoot freely. A property that cannot go past its ends
clamps **at the property**, not the curve: `opacity` never shows above `1.0` or
below `0.0`, and `volume` never goes below silence (above `1.0` it is gain, so
an overshooting volume ramp is briefly louder). Every curve still starts and
arrives exactly on its keyframes.

### Tracks a tool wrote

A keyframe track may carry an optional `by` naming the tool that generated it:

```json clip
{
  "id": "c-bed", "asset": "bed", "start": 0, "duration": 300,
  "keyframes": [
    { "property": "volume", "by": "duck", "keyframes": [
        { "t": 0, "value": 1.0, "easing": "ease_out" },
        { "t": 9, "value": 0.25 }
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
| `aberration` | how far the layer's colour channels are pulled apart from its centre outward, as a fraction of its own **height** | `0.0` untouched, higher fringes harder |
| `chroma_key.tolerance` | how far a pixel's colour may sit from the keyed screen colour and still be keyed out | `0.25` by default; nothing without a `chroma_key` |
| `chroma_key.softness` | how wide the ramp from screen to subject is, outward from the tolerance | `0.1` by default; `0.0` is a hard cutout |
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
| `grade.grain` | how much grain is laid over the layer, strongest through the midtones | `0.0` none, `1.0` heaviest |
| `vhs.chroma_bleed` | how far the tape smeared the layer's colour sideways, as a fraction of its own **width** | `0.0` none, `1.0` heaviest |
| `vhs.noise` | how much snow the tape laid over the layer, on its colour as well as its brightness | `0.0` none, `1.0` heaviest |
| `vhs.scanlines` | how dark the tape's alternate lines are | `0.0` none, `1.0` darkest |
| `vhs.jitter` | how far the tape's tracking wobbles the layer sideways, as a fraction of its own **width** | `0.0` holds still |
| `vhs.head_switch` | how torn the band at the bottom of the layer is, where the tape's heads hand over | `0.0` none, `1.0` worst |
| `shape.trim_start` | where a shape's drawn line starts, as a fraction of its outline's **length** | `0.0` its start; clamped to `0`–`1` |
| `shape.trim_end` | where a shape's drawn line ends, as a fraction of its outline's **length** | `1.0` all of it, `0.0` none; clamped to `0`–`1` |
| `shape.dash_offset` | how far a dashed shape's pattern has moved along its line toward the end, as a fraction of the raster's **height** | `0.0` unmoved; nothing without a `dash` |
| `reveal` | how much of a text layer has arrived, piece by piece — [see above](#revealing-by-character-word-or-line-reveal) | `1.0` all of it, `0.0` none |
| `number` | the figure a text layer writes where its text says `{n}` — [see above](#a-number-that-counts-number) | its `number` block's `value` |
| `shadow.opacity` | how dark the layer's drop shadow is, clamped to `0.0`–`1.0` | `0.5` by default; nothing without a `shadow` |
| `glow.radius` | how far the layer's glow reaches, as a fraction of its own **height**, held to the longest side of what it actually draws | `0.02` by default; nothing without a `glow` |
| `glow.intensity` | how bright the layer's glow is, clamped to `0.0`–`4.0` | `1.0` by default; nothing without a `glow` |
| `follow.progress` | how far along the arrow in its `follow` a clip is, as a fraction of the arrow's **length** | `0.0` its tail, `1.0` its head; clamped to `0`–`1`; nothing without a `follow` |
| `volume` | how loud a clip plays, on either kind of track | `1.0` as recorded, `0.0` silent |

Scale, rotation and flip all pivot on the clip's `origin`, which is the layer's
own **centre** unless the clip names another point — so by default shrinking a
clip does not also slide it into a corner and turning one does not swing it
around an edge. Rotation is in degrees and **positive turns clockwise** — nobody
should have to render a frame to find that out. The layer is scaled first and
then turned, both about that one point; position is applied after both, and an
origin does not change what a position is worth.

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

A project directory holds five of its own:

| Directory | What is in it | Survives a delete? |
| --- | --- | --- |
| `assets/` | imported media, copied in on import | no — the originals are elsewhere |
| `generated/` | provider and synthesis output, named for the hash of its brief — a narration's word timings beside its audio, as `<same name>.words.json` (see *When each word is said*) | yes — it can be made again |
| `recipes/` | authored synthesis documents | **no** — deleting one loses work |
| `pages/` | authored web pages, played by `html` assets | **no** — deleting one loses work |
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
asset carries `shape`, only an `icon` asset carries `icon` and only a `group`
carries `group` and only an `image_sequence` carries `sequence`, that an `html`
asset's path ends in `.html`, that an icon has
a size and a thickness to draw with, that a shape has area, a corner it has room to round,
something to draw with and a `dash` of at least one length, each above zero —
and that an arrow has two ends in different
places and no `fill`, since a line has no inside — that a gradient `fill` or
`color` has at least two stops rising from `0` to `1`, a finite angle, and a
centre and a radius above zero — that a `style`'s
font path, a `synth_audio`'s `recipe` and the document's `script`
obey the project-path rules, and that each generated kind carries exactly the
brief it takes: a `prompt` or a `recipe`, never both and never the other's —
clip references resolving, asset kind against track kind, non-zero durations,
clip overlap, no clip reaching past the end of the source it was measured to
have — or the group it shows — and keyframe shape, on the timeline and inside
every group alike; what only a group raises (see *Group assets*) and what only
an image sequence raises (see *Image sequences*); and that a
`follow` names an arrow clip on the follower's own timeline, not itself, and not
one attached to another follower (see *Travelling along an arrow*).

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
- **The title states this build's schema version.** A test holds the page's
  first line to `SCHEMA_VERSION`, so a bump that forgets the title fails the
  build instead of leaving it behind.
- **Every command and flag in `scorsese --help` says something**, so the only
  interface an agent has today cannot grow a silent flag.

**What none of this proves.** A green CI run says every property is mentioned,
every example still parses, and every flag has help. It says nothing about
whether the sentence next to any of them is still *true* — that is the limit
of any documentation gate, and it is written here so a green check is never
mistaken for an accurate page. Prose that describes behaviour is still checked
by reading it.
