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

The paid lessons are all Veo, because that is what has been generated most.
Image and speech lessons belong beside them, when there are some.

## Never name the medium

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

## Slow motion is bought, not applied

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

## Buy content from Veo; do the look in the edit

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

## Veo 3.1 generates sound whether the prompt mentions it or not

A prompt that says nothing about audio does not come back silent. It comes back
with whatever the model decided the shot sounds like — room tone, footsteps,
wind, sometimes voices — chosen on its behalf and paid for either way.

Since it is arriving regardless, say what it should be. A prompt that names its
own sound is not asking for an extra; it is taking a decision that would
otherwise be made by something that has not read the rest of the cut.

## Veo outputs 24 fps

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

## A shot between two stills is eight seconds, on every tier

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

## What the vendors advise

Everything from here to *Adding to this page* is the vendors' guidance, not
this project's: condensed from their own pages, linked, and dated with when it
was read. It says how to write the sentence. It never says what the sentence
should ask for — that is the person's video.

### Veo, in Google's words

From Google's [Veo guide](https://ai.google.dev/gemini-api/docs/veo), read
2026-10-08, for the Veo 3.1 models scorsese generates with.

**A prompt is descriptive and clear.** Start from the core idea, then refine it
with keywords and modifiers and with the vocabulary of film. The guide names
the parts a prompt can have:

- **Subject** — the object, person, animal or scenery.
- **Context** — the setting or background the subject is in.
- **Action** — what the subject is doing: walking, running, turning its head.
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

The guide's paired examples make the same point each time: the prompt with
more detail — camera movement, lighting, depth of field, the subject's
features, the tone — comes back closer to what was meant than the short one.
Descriptive adjectives and adverbs help, and the word **"portrait"** asks for
facial detail.

**Sound is prompted in the same sentence** (and arrives whether asked for or
not: [Veo 3.1 generates sound](#veo-31-generates-sound-whether-the-prompt-mentions-it-or-not)):

- **Dialogue** goes in quotes, attributed: *"This must be the key," he
  murmured.*
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
- **Reference images** (`reference_images`, up to three, not on `lite`) of
  one person, character or product keep that subject looking the same; the
  prompt then says what the subject does.

**Extending a shot**, once scorsese offers it (#892), continues from the generated video's last second,
so its prompt says *what happens next*, not the whole shot again — "the
paraglider slowly descends". A voice cannot be carried on if there is none in
that last second.

**Limits worth knowing before paying:** a prompt is at most 1,024 tokens;
English is the language Google has evaluated, and others "may work but results
can vary"; a prompt the safety filters block is not charged.

### Nano Banana, in Google's words

From Google's [image generation guide](https://ai.google.dev/gemini-api/docs/image-generation),
read 2026-10-08, for the Gemini image models (Nano Banana) scorsese's
`generated_image` uses. Much of the advice is in the page's examples rather
than in one section; this gathers it.

**Be specific.** "The more specific you are, the more control you have over
the results": the subject, the setting, the light, the camera angle and lens,
the background, where things sit in the frame, the colours. The guide's
templates are shaped like this:

- **A photograph**: *A photorealistic [type of shot] of [subject] in
  [setting]. [The light]. Shot from [camera angle] with [lens].*
- **An illustration or sticker**: *A [style] of [subject, with its details]
  doing [activity]. The design features [bold outlines, cel shading…] and
  [colour or background].* Naming the visual qualities is what keeps a series
  consistent.
- **A product shot**: high-resolution, studio-lit, the surface it stands on,
  the lighting set-up, the camera angle, the detail in sharp focus.
- **Room for a title**: *A minimalist composition with a single [subject] in
  the [bottom-right…] of the frame*, over an empty background. In scorsese the
  title itself is then a text layer or a page, not pixels in the picture.
- **A comic**: *Make a 3 panel comic in a [style]. Put the character in a
  [type of scene].*

**Say what it is not, when it could be misread.** The guide's isometric pool
photograph says *"It is not a miniature, it is a captured photo that just
happened to be perfectly isometric"*, and its icon ends *"No text."* A plain
statement of what to leave out works where a list of banned words does not.

**Text in a picture** is asked for in quotes, with the font described and the
placement stated: *the large bold words "…" in a serif font… No other text.*

**Editing with a picture given** — reference images in scorsese's `image`
block:

- **Add, remove or change one thing**: *Using the provided image of
  [subject], [add/remove/modify] [element]. Make sure the change [fits how].*
  The model matches the original's style, light and perspective.
- **Change only one part**: *Using the provided image, change only the
  [element] to [new element]. Keep everything else exactly the same,
  preserving the original style, lighting and composition.*
- **Several pictures together** say what each one is for: the logo on the
  bottle, these people in that office.

**The aspect ratio and size are settings, not words**: scorsese's `image`
block sets them, and a ratio written only in the prompt is not one.

### ElevenLabs, in its own words

From ElevenLabs' [prompting best practices](https://elevenlabs.io/docs/best-practices/prompting),
read 2026-10-08. The advice depends on the model, and scorsese's `speech`
block names one of three: `expressive` (Eleven v3), `standard` (Multilingual
v2) and `fast` (Flash v2.5, the default).

**On `expressive` (v3):**

- **Audio tags in square brackets** direct the reading: `[whispers]`,
  `[sighs]`, `[laughs]`, `[curious]`, `[shouts]`. A tag describes the voice
  quality wanted (`[low, gravelly voice]`), not something that could be read
  as a sound to make.
- **Pauses come from the text**: ellipses add a pause and weight, and the
  sentence's own structure does the rest. v3 does **not** honour `<break>`
  tags.
- **Capitals add emphasis.**
- **The voice still matters most**: a tag that fights the voice's natural
  delivery (whispering on a voice that shouts) works badly. A delivery that is
  in the voice's own range is easy to get.
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
