# Prompts — what a provider does with the words

A `recipe` is cheap to be wrong about: synthesis runs locally, and a bad one
costs a rebake. A `prompt` is not. It goes to a provider over the network,
money is spent, and what comes back is what you have — so a word that means
something unexpected to the model is paid for before anyone finds out that it
did.

This page holds two kinds of entry, kept apart because they are worth
different things.

- **Learned by paying.** Provider behaviour that cannot be guessed from the
  outside, each entry with the incident behind it: something this project
  learned by generating a shot, looking at it, and having already paid.
- **The vendors' own advice**, at the end: what Google and ElevenLabs publish
  about writing a prompt for their models, condensed, linked, and dated with
  when it was read, so it can be checked against their pages again.

**Where the two disagree, the paid lesson wins**, and a line beside the advice
says so: a vendor writes for the shots in its own examples, and a lesson here
is what happened to one of ours.

Neither half is a style guide. What a shot should look like belongs to whoever
is making the video; this page is about how a provider reads the sentence that
asks for it.

**Before a prompt is written at all**, ask whether the shot needs to be
generated: a generic shot — a sunrise, a city at night, an office — is free
from stock, and `guide stock` says when that is the right call. A character,
a mascot, an animated icon or illustration is usually a free Lottie from
LottieFiles (`stock_search` with `kind: lottie`), played by a page.

**Before any generation, the storyboard.** Every paid generation (a shot, a
still, a spoken line) is written down first as a storyboard in words, in the
project's script. It opens with the story in one sentence: what happens and
to whom, not the genre. Then comes one entry per shot:
- what is in the frame;
- how it is framed and how the camera moves;
- what moves in it;
- how long it runs;
- what it tells that the shot before did not.

This holds whoever is deciding. When the person is there, they read it,
change it and agree it before anything is quoted: a paragraph per shot is
where a wrong idea costs nothing to fix. When the choices were left to the
assistant, as in an unattended batch, it is written anyway and checked
against this page before the quote, so that a shot which breaks a lesson
below breaks it on paper first. A storyboard is text, not drawings: drawing
every frame costs time and tokens that a paragraph does not, so pictures are
made only when the person asks for them.

**The lessons on this page are advice, not rules.** The video is the person's.
When they ask for something a lesson advises against (a chase, a line spoken to
camera, a sign to read), say so once, in a line, with what usually goes wrong
and what would hold up instead. If they still want it, it is done their way,
and done as well as it can be.

The paid lessons sit under one heading per provider: Veo first, because it is
what has been generated most, then Nano Banana, then ElevenLabs. Each opens
with its lesson **in a line**, and those lines, with the ones that close the
Veo and Nano Banana vendor sections, are also carried on the tools that send a
prompt: the `video`, `image` and `speech` blocks of `asset_set` and
`voice_design`'s description. They are built from this page when the tools
are, so the two never disagree; the reasons stay here.

## Veo, learned by paying

### Ask Veo for a scene that needs no fine detail

**In a line:** describe the shot fully, but choose a scene that holds up from its
broad forms: lettering, hands, small objects and faces come back wrong.

*From the cyberpunk teaser (`cyberpunk-2077-teaser.scor`, 2026-10-10), and
from the maintainer's own Veo work.*

A prompt is descriptive. It says plainly and fully what the shot is (the
subject, the place, the light, the camera), written from a clear picture of
what is wanted. What comes back keeps the broad forms of that description:
shapes, layout, light, the movement of large things. Fine detail does not
survive: the lettering on a sign, the fingers of a hand, the exact design of
a small object, whatever makes a face a particular face. **The less fine
detail a shot needs to work, the better it comes out.**

So the scene is chosen for this, not only worded for it. When one idea can be
shown two ways, pick the one that holds up from its broad forms: the city as a
skyline of lit towers rather than a street of signs to read, a figure in
silhouette against a window rather than a face reacting. This is no reason to
write less. The prompt stays descriptive and specific, with a clear vision of
the shot; it is the scene that asks for less.

In the teaser, a gang standing in an alley came back with walls of signs in
made-up lettering. It was bought a second time as bare concrete walls with no
signs, for $0.40.

### Veo cannot act, and fast motion falls apart

**In a line:** keep motion slow and faces neutral and away from the camera; no
chase, fight, fast camera move or line spoken on camera unless the person asks,
and a line that is needed is a voice over a shot that does not show the mouth.

*From the cyberpunk teaser (2026-10-10), as the maintainer judged it on
2026-10-11.*

The teaser was written the way trailers usually are: a chase, a gang, three
characters each saying a line in close-up. Its cutting and its cards were
judged excellent, but the shots carrying the story were not:
- **The spoken close-ups read as hammy and overplayed.** Nobody watching
  expects an AI character to deliver a line, and when one does, the delivery
  is what they notice.
- **Fast movement never comes out well.** That holds for action, a chase, or a
  quick camera, in this film and in all the maintainer's own Veo work.

What holds up:
- a neutral expression, never a performed one;
- characters at a distance from the camera, not a close-up on a face;
- a silhouette, or the character seen from behind;
- close-ups on objects (a hand on a door handle, a cracked screen, rain on a
  window), which carry a feeling without asking anyone to act it;
- slow movement, of the camera and in the frame.

What to avoid, unless the person asks for it:
- action at speed: chases, fights, fast camera moves;
- dialogue on camera.

When a line is needed anyway, it is a voice over a shot that does not show the
mouth: a narrator, or a character seen from behind or from far away.

### Never name the medium

**In a line:** never name a physical medium (16mm, film stock, Super 8, VHS,
Polaroid), which Veo can draw into the picture as an object; describe the image
instead: the era, the grain, the lens.

Every shot in Scene 1 of the Summit film wanted a period look, and every one of
them asked for *"shot on grainy 16mm film"*. Veo rendered the film **object**:
sprocket holes down both edges, edge codes and frame numbers, inside the
picture, on all three shots. The recovery was a per-clip
[`crop`](project-format.md#showing-part-of-a-source-crop)
(`guide project-format`, section "Showing part of a source"), which costs
nothing to apply and permanently trims a frame that was bought at full size.

Film stock, 16mm, Super 8, VHS, Polaroid — each of those is a physical thing
the model knows how to draw, and asked for the *look* of one it may draw the
thing instead. What survives the round trip is a description of the image:
the era, the grain, the lens. "1970s documentary footage, fine grain, soft
period lens" asks for the same picture and names nothing that can be
photographed.

The two wordings do not look different from each other, which is the entire
reason this is written down.

### Slow motion is bought, not applied

**In a line:** a shot meant to be slow asks for slow motion in the prompt; a
clip's `speed` only stretches the frames that came back.

A clip's [`speed`](project-format.md#playing-faster-or-slower-speed)
(`guide project-format`, section "Playing faster or slower")
redistributes frames that already exist — at `0.5` each source frame covers two
timeline frames, and nothing new is invented, by design. Slow motion asked for
in a prompt is a different thing in kind: Veo renders the motion slow, which
means it renders motion samples that a normal-speed generation of the same shot
would never have contained.

It is the familiar relationship between 60 fps slowed to 0.5× and 120 fps
played back at 60. Both run half as fast; only the second has the detail,
because the first is holding frames the camera never took.

So a shot that should be slow is bought slow, in the sentence, before any money
changes hands. `speed` is for retiming footage that already exists — and for
generated footage it is the fallback, not the plan.

### Buy content from Veo; do the look in the edit

**In a line:** ask for a neutral picture and leave out the tint, vignette, grain,
faded look, titles and fades, which the edit does for free and can still change.

"Colour film stock, naturally exposed" is a useful sentence: it asks for an
ordinary rendition and leaves the look to be decided afterwards. Asking Veo for
the grade instead bakes it into a file that cannot be regenerated without
paying again, and "a little less brown" stops being an edit and becomes a
purchase. That was the lesson of the 2026-08 cuts, and it generalises: **what
a shot contains is bought from the provider; anything the edit can do is done
in the edit.**

A clip carries a [`grade`](project-format.md#how-it-looks-grade)
(`guide project-format`, section "How it looks"): saturation, temperature,
brightness, contrast, vignette and grain, each animatable across a shot, free
and reversible. So a prompt asks for a neutral picture and leaves the filters
out — the tint, the vignette, the faded look, the grain, a title over the
action, a fade — because each of those is a number on the timeline that can
still be changed tomorrow, and in a prompt it is pixels that cannot.

What stays in the prompt is what no edit can add: the subject, what it does,
the place, the light falling on it, where the camera is and how it moves.

### Veo 3.1 generates sound whether the prompt mentions it or not

**In a line:** every shot comes back with sound, so the prompt says what it should
sound like.

A prompt that says nothing about audio does not come back silent. It comes back
with whatever the model decided the shot sounds like — room tone, footsteps,
wind, sometimes voices — chosen on its behalf and paid for either way.

Since it is arriving regardless, say what it should be. A prompt that names its
own sound is not asking for an extra; it is taking a decision that would
otherwise be made by something that has not read the rest of the cut.

### Veo outputs 24 fps

**In a line:** shots come back at 24 fps, so a cut made mostly of generated shots
is a project created at 24 fps (`scorsese new --fps 24`).

Generated video comes back at 24 fps. A project created at the default 30 will
[conform](project-format.md#conforming-source-fps--timeline-fps)
(`guide project-format`, section "Conforming") every one of
those shots by repeating source frames in the 2:3 pattern — so the judder lands
on exactly the footage that cost money, on a timeline whose rate was never
really chosen.

That is a reason to decide it: `scorsese new film.scor --fps 24` for a cut
built mostly out of generated shots. The
[timeline framerate](project-format.md#the-timeline-framerate)
(`guide project-format`, section "The timeline framerate") is chosen at
project creation and is not a field edit afterwards, so it is worth a moment at
the start rather than a rescale later.

### A shot between two stills is eight seconds, on every tier

**In a line:** a first and a last image together fix the shot at 8 seconds on
every tier, which Google's page does not say.

Google's [Veo page](https://ai.google.dev/gemini-api/docs/veo), read
2026-10-08, says a shot must be eight seconds at 1080p or 4K or with reference
images, and says nothing of the kind about a first and a last image. So on
2026-10-10 (#928) a 720p shot with both was asked for at shorter lengths:

| tier | seconds | what came back |
| --- | --- | --- |
| `lite` | 4 | refused |
| `lite` | 6 | refused |
| `lite` | 8 | an 8.000 s shot, 1280×720, 24 fps |
| `fast` | 4 | refused |

Every refusal was the same 400 at submit, and was not charged: *"Your use case
is currently not supported. Please refer to Gemini API documentation for
current model offering."* It never mentions the length — so someone who meets
it cannot tell from the message that the fix is `8`, and the documentation it
points to does not say so either.

What it means: a first and last image fix a shot at eight seconds whatever the
tier and raster, exactly as reference images do, and `scorsese check` refuses
anything shorter before it is sent. Standard was not tried; the rule is the
same one Lite and Fast keep, and is held for it too.

## Nano Banana, learned by paying

### Nano Banana draws a photograph unless the medium is named

**In a line:** name the kind of picture ("flat vector illustration", "pencil
sketch", "watercolour"): a prompt that only describes its subject comes back a
photograph.

A sentence that only describes its subject comes back **photoreal**. On
2026-10-10 the same character was generated twice for `vectorize` (#1036,
Nano Banana 2.1, 2K, 1:1): *"A friendly young woman waving hello, full body,
standing, on a white background."* came back as a photograph — a real woman in
a studio, soft shading, a shadow on the floor — where an illustration had been
meant.

Nothing in that sentence was wrong; it just never said what kind of picture
it was. The default is a photograph, so anything else is asked for by name:
"flat vector illustration", "pencil sketch", "watercolour". (Veo is the
opposite case: [Never name the medium](#never-name-the-medium) is about a
*physical* medium drawn into a video as an object. An illustration style
named for a still is the picture itself.)

### A picture to trace is asked for flat

**In a line:** a still meant for `vectorize` ends with "flat vector illustration,
thick dark outlines, solid colours, no gradients or shading, plain white
background".

`vectorize` (`guide pages`, section "A traced picture, drawn on") turns a picture
into the strokes a page draws on, and what it makes of one depends almost
entirely on the words that bought it. The two generations above, traced at
`vectorize`'s defaults (8 colours, `detail: medium`, background dropped) and
drawn on with `kit.draw`:

| prompt ended with | Nano Banana drew | `vectorize` said | drawn on |
|---|---|---|---|
| *"Flat vector illustration, thick dark outlines, solid colours, no gradients or shading, plain white background."* | a flat cartoon: dark outlines, flat fills, white ground | 41 pen strokes then 35 filled shapes, 6 colours, 220 KB | a hand drawing it: every outline as one clean pen line — a complete line drawing at one second — then the colours filling in |
| *"…on a white background."* | a photograph | 13 pen strokes then 177 filled shapes, 7 colours, 524 KB | posterised blobs: hair and jeans traced as smears, the face and cardigan as patches of colour, the floor shadow as a grey slab. Unusable as a drawing |

So a picture meant for `vectorize` ends with **"flat vector illustration,
thick dark outlines, solid colours, no gradients or shading, plain white
background"**, and each phrase is doing work: the outlines become the pen
strokes, "solid colours, no gradients or shading" is what keeps the shapes to
a few dozen instead of nearly two hundred, and the plain white background is
what `vectorize` drops so the drawing sits on the page's board. The subject
and what it is doing go before it, as usual.

## ElevenLabs, learned by paying

### For narration, the model carries the language

**In a line:** for a line in a language other than the voice's own, choose the
`expressive` model, since `standard` reads it with the voice's accent; a voice
native to the language is a nicety.

An English voice read Portuguese **cleanly on Eleven v3**, what `expressive`
spoke then, and **with an English accent on `standard`** (Multilingual v2), at
the same price per character (`guide prices`). The voice was the same and the
text was the same; only the model changed. `expressive` is Eleven v4 now
(#1248), which the vendor says covers more languages than v3 and handles
speaking across languages differently on purpose; no paid generation has
re-checked this lesson on v4 yet.

So for a line in a language other than the voice's own, the model is the
choice that matters, and a voice native to the language is a nicety rather
than a requirement. Name `expressive`; hunting the Voice Library for a native
speaker of the language is a search that the model makes unnecessary.

### A free key lists library voices it cannot speak with

**In a line:** a free ElevenLabs key lists Voice Library voices it is refused when
it asks them to speak; on a free key, pick a built-in voice, or `check` the
chosen one with `voices` before writing around it.

An ElevenLabs key on the free plan **lists** Voice Library voices
(`scorsese voices`, the `voices` tool) like any other key, and is refused
when it asks one of them to speak: a `402` with `paid_plan_required`, *"Free
users cannot use library voices via the API."* Nothing was charged, but a
narration planned around a library voice found from the listing is a plan
that cannot be carried out on that key.

The built-in voices speak on a free key the same way they do on a paid one.
So on a free key, pick from the built-in voices, or ask `voices` to `check`
the one chosen (`guide mcp`, section "Choosing a voice") before a script is written
around it. The refusal names the plan limit, never a missing voice.

## What the vendors advise

Everything from here to *Adding to this page* is the vendors' guidance, not
this project's: condensed from their own pages, linked, and dated with when it
was read. It says how to write the sentence. It never says what the sentence
should ask for — that is the person's video.

### Veo, in Google's words

From Google's [Veo guide](https://ai.google.dev/gemini-api/docs/veo), read
2026-10-08 and re-read 2026-10-15 (the page *last updated 2026-10-08 UTC*), for
the Veo 3.1 models scorsese generates with.

**A prompt is descriptive and clear.** Start from the core idea, then refine it
with keywords and modifiers and with the vocabulary of film. The guide names
the parts a prompt can have:

- **Subject** — the object, person, animal or scenery.
- **Context** — the setting or background the subject is in.
- **Action** — what the subject is doing: walking, running, turning its head.
  *But see* [Veo cannot act](#veo-cannot-act-and-fast-motion-falls-apart):
  slow action holds up, and fast action falls apart.
- **Style** — film-style or animation keywords: sci-fi, horror film, film
  noir, cartoon. *But see* [Never name the medium](#never-name-the-medium):
  a style is safe as a genre or a look, and a physical medium (16mm, VHS,
  Polaroid) can come back drawn into the picture as an object.
- **Camera position and motion** (optional) — aerial view, eye level,
  top-down, dolly shot, worm's eye.
- **Composition** (optional) — wide shot, close-up, single shot, two-shot.
- **Focus and lens** (optional) — shallow focus, deep focus, soft focus, macro
  lens, wide-angle lens.
- **Ambiance** (optional) — colour and light: blue tones, night, warm tones.
  *But see* [Buy content from Veo](#buy-content-from-veo-do-the-look-in-the-edit):
  the light in the scene belongs in the prompt; a tint over the whole picture
  is a `grade`, and free.

The page's *Icicles* example names each part where it falls: *"Close up shot
(composition) of melting icicles (subject) on a frozen rock wall (context)
with cool blue tones (ambiance), zoomed in (camera motion) maintaining
close-up detail of water drips (action)."*

The guide's paired examples make the same point each time: the prompt with
more detail — camera movement, lighting, depth of field, the subject's
features, the tone — comes back closer to what was meant than the short one.
Descriptive adjectives and adverbs help, and the word **"portrait"** asks for
facial detail. *But see*
[Ask Veo for a scene that needs no fine detail](#ask-veo-for-a-scene-that-needs-no-fine-detail):
describe richly, but choose a shot that does not depend on fine detail such as
a face.

**Sound is prompted in the same sentence** (and arrives whether asked for or
not: [Veo 3.1 generates sound](#veo-31-generates-sound-whether-the-prompt-mentions-it-or-not)):

- **Dialogue** goes in quotes, attributed: *"This must be the key," he
  murmured.* *But see* [Veo cannot act](#veo-cannot-act-and-fast-motion-falls-apart):
  a line spoken on camera is the weakest part of a shot.
- **Sound effects** are described explicitly: tyres screeching, an engine
  roaring.
- **Ambient sound** is described as the place's soundscape.

More detail in the audio part gives a richer soundtrack, the guide says.

**Pictures as part of the brief** — scorsese's `video` block carries each of
these (`guide project-format`, section "What a generated video asks for"):

- **A first image** (`first_image`) becomes the shot's opening frame: pick the
  picture closest to how the shot should begin, and let the prompt describe
  what happens from there.
- **A first and a last image** (`last_image` beside it) fix where the shot
  starts and where it ends; the prompt describes the action between them.
  Such a shot is only generated at eight seconds, which the page does not
  say ([paid for](#a-shot-between-two-stills-is-eight-seconds-on-every-tier)).
- **Reference images** (`reference_images`, up to three, not on `lite`) are
  pictures "of a single person, character, or product": three views of one
  subject, never three subjects. They keep it looking the same, and the
  prompt then says what it does.

**Extending a shot**, once scorsese offers it (#892), continues from the generated video's last second,
so its prompt says *what happens next*, not the whole shot again — "the
paraglider slowly descends". A voice cannot be carried on if there is none in
that last second.

**Limits worth knowing before paying:** a prompt is at most 1,024 tokens
(the *Model versions* table, all three 3.1 models), and `generate` refuses one
clearly past that before it is sent (`guide project-format`, section "What a
generated video asks for"); English is the language Google has evaluated:
"English (EN) is fully supported, but other languages have not been evaluated,
so they may work but results can vary"; a shot the safety filters, or Veo's
audio processing, block "will not be charged".

**In a line each**, as the `video` block carries them (where the advice above
has a *But see*, the line takes the paid lesson's side):

- A prompt names the subject, the context, the action and the style, then if
  wanted the camera position and motion, the composition, the focus and lens
  and the ambiance — "Close up shot of melting icicles on a frozen rock wall
  with cool blue tones, zoomed in, maintaining close-up detail of water drips".
- More detail (the camera, the light, the depth of field, the subject's
  features, the tone) comes back closer to what was meant than a short prompt.
- Sound goes in the same sentence: dialogue in quotes and attributed, sound
  effects described explicitly, ambience as the place's soundscape.
- With `first_image` the prompt describes what happens from that opening
  frame; with `last_image` too, the action between the two.
- `reference_images` are up to three pictures of one single person, character
  or product, never three subjects, and the prompt says what it does.
- Write the prompt in English, the only language Google has evaluated, even
  when the rest of the video is in another.
- A prompt is at most 1,024 tokens.

### Nano Banana, in Google's words

From Google's [image generation guide](https://ai.google.dev/gemini-api/docs/image-generation),
read 2026-10-08 and re-read 2026-10-15 (the page *last updated 2026-10-09
UTC*), for Nano Banana 2.1, the model scorsese's `generated_image` uses.

**In a line each**: Google's six best practices, then the rest of the page,
as the `image` block carries them:

- Be hyper-specific: describe the thing ("ornate elven plate armor, etched with
  silver leaf patterns") rather than name it ("fantasy armor"), with the
  setting, the light, the lens and where things sit in the frame.
- Say what the picture is for: "a logo for a high-end, minimalist skincare
  brand" beats "a logo".
- To change one thing in a still, keep the previous one as a reference and say
  "Using the provided image, change only the… Keep everything else exactly the
  same", rather than rewrite the prompt from scratch.
- For a complex scene, give it in steps: "First, a background of… Then, in the
  foreground… Finally…".
- Describe the scene positively ("an empty, deserted street with no signs of
  traffic", not "no cars"); a short plain exclusion like "No text." is fine.
- Control the camera in words: wide-angle shot, macro shot, low-angle
  perspective.
- Settle a picture's text first, then quote its exact words in the prompt with
  the font and the placement.
- Say what each reference is for: the logo on the bottle, these people in that
  office.
- It works best in English and fourteen other languages, Portuguese (pt-BR)
  among them.
- The aspect ratio and size are this block's settings, never words in the
  prompt.

Google's *Iterate and refine* is follow-up turns in a chat; a still here is
one call per brief, which is why the third line is an edit with a reference.
Its *semantic negative prompts* and its own icon ending *"No text."* read as
one rule, the fifth line. The page's templates:

- **A photograph**: *A photorealistic [type of shot] of [subject] in
  [setting]. [The light]. Shot from [camera angle] with [lens].*
- **An illustration or sticker**: *A [style] of [subject, with its details]
  doing [activity]. The design features [bold outlines, cel shading…] and
  [colour or background].* Naming the visual qualities keeps a series
  consistent. *But see* [A picture to trace is asked for flat](#a-picture-to-trace-is-asked-for-flat):
  shading of any kind traces as extra shapes.
- **A product shot**: high-resolution, studio-lit, the surface it stands on,
  the lighting set-up, the camera angle, the detail in sharp focus.
- **Room for a title**: *A minimalist composition with a single [subject] in
  the [bottom-right…] of the frame*, over an empty background; the title is
  then a text layer or a page, not pixels.
- **A comic**: *Make a 3 panel comic in a [style]. Put the character in a
  [type of scene].*
- **Text in a picture**: *the large bold words "…" in a serif font… No other
  text.* "Gemini works best if you first generate the text and then ask for an
  image with the text."
- **Change only one part**: *Using the provided image, change only the
  [element] to [new element]. Keep everything else exactly the same,
  preserving the original style, lighting and composition.* The model matches
  the original's style, light and perspective.

The pictures in scorsese's `image` block are `reference_images` (objects, at
most 10) and `character_images` (characters to keep looking like themselves,
at most 4); there is no style reference, a style is described in words. The
languages the page lists are "EN, ar-EG, de-DE, es-MX, fr-FR, hi-IN, id-ID,
it-IT, ja-JP, ko-KR, pt-BR, ru-RU, ua-UA, vi-VN, zh-CN", so a still can be
prompted in Portuguese, unlike a Veo shot. And "All generated images include a
SynthID watermark": invisible, marking the picture as AI-made.

### ElevenLabs, in its own words

From ElevenLabs' [prompting best practices](https://elevenlabs.io/docs/best-practices/prompting),
read 2026-10-08, and its *Prompting Eleven v4* section re-read 2026-10-14. The
advice depends on the model, and scorsese's `speech` block names one of three:
`expressive` (Eleven v4), `standard` (Multilingual v2) and `fast` (Flash v2.5,
the default). `expressive` was Eleven v3 until #1248. The vendor says the v4
techniques also apply to v3, so what was learned on v3's tags carries over,
until a paid generation shows a difference.

**On `expressive` (v4):**

- **Audio tags in square brackets** direct the reading: `[whispers]`,
  `[sighs]`, `[laughs]`, `[curious]`, `[shouts]`. A tag describes the voice
  quality wanted (`[low, gravelly voice]`), not something that could be read
  as a sound to make. Tags are not said, so they are never words in the
  line's timings: captions and pages never show one. The vendor calls v4's
  handling of tags more nuanced than v3's, and "not perfect yet".
- **Pauses come from the text**: ellipses add a pause and weight, and the
  sentence's own structure does the rest. v4, like v3, does **not** honour
  `<break>` tags; the vendor says SSML is not supported.
- **Capitals add emphasis.**
- **The voice still matters most**: a tag that fights the voice's natural
  delivery (whispering on a voice that shouts) works badly. A delivery that is
  in the voice's own range is easy to get.
- **A word said wrongly can be written in IPA between forward slashes**, which
  the vendor says v4 reads without phoneme tags; results still vary by voice.
- Very short lines read less consistently; the guide tests with more than 250
  characters.

**On `standard` and `fast` (v2, v2.5):**

- **A pause is a break tag**: `<break time="1.5s" />`, up to three seconds.
  Too many in one line can make the reading unstable; a dash or an ellipsis is
  a gentler, less reliable pause.
- **A word said wrongly is respelled** the way it sounds ("Claughton" as
  "Cloffton"), with capitals, dashes or apostrophes if they help. Phoneme tags
  are not supported on these two.
- **Numbers and symbols** are read out by the model's own normalisation, which
  Multilingual v2 does well and Flash v2.5 can get wrong ("$1,000,000"): for a
  `fast` line, write numbers the way they should be said.

**The language** is set on the `speech` block, not in the text, and
`standard` ignores it (`guide project-format`, section "What a spoken line
asks for"). Writing the
line in the language it should be spoken in is what every model honours.

## Adding to this page

**Every paid lesson opens with `**In a line:**`** and one sentence saying what
to do, under the heading of the provider it is about, and a vendor section the
tools carry closes with an `**In a line each**` list. Those lines are what the
tools put in front of every assistant that writes a prompt, so each is a
sentence that stands on its own, without a link. A test holds every paid
lesson to exactly one, and a heading that is no provider's to none.

A paid lesson belongs here when it is a **fact about what a provider does**
that cost a generation to learn, and it should say what happened rather than
only what to do. A rule with no incident behind it is advice, and advice goes
in a vendor section, attributed to the vendor — the incident is what lets the
next reader tell whether their case is the same one.

A vendor section is the vendor's page condensed, never this project's opinion
of it, with the link and the date it was read. When the page changes, the
section is re-read against it and re-dated. When a paid lesson contradicts it,
the lesson is written up first and a line beside the advice points at it.

The gap this page fills has a mirror image on the free side: #189 says
`guide recipes` describes how a source is *built* and never what it *sounds
like*. Both are the same shape of missing. A document can specify every field
exhaustively and still not say what putting a particular word in one of them
does, and only one of those two omissions can be discovered without a bill.
