# Prompts — what a provider does, learned by paying

A `recipe` is cheap to be wrong about: synthesis runs locally, and a bad one
costs a rebake. A `prompt` is not. It goes to a provider over the network,
money is spent, and what comes back is what you have — so a word that means
something unexpected to the model is paid for before anyone finds out that it
did.

This page is where those words get written down. Every entry is something this
project learned by generating a shot, looking at it, and having already paid.

It is **not a prompt cookbook and not a style guide.** Scorsese has no business
teaching anyone to write, and what a shot should look like belongs to whoever
is making the video. What is here is narrower and duller: provider behaviour
that cannot be guessed from the outside, each entry with the incident behind
it.

Everything below is Veo, because that is what has been generated so far.
ElevenLabs entries belong on this page too, when there are some.

## Never name the medium

Every shot in Scene 1 of the Summit film wanted a period look, and every one of
them asked for *"shot on grainy 16mm film"*. Veo rendered the film **object**:
sprocket holes down both edges, edge codes and frame numbers, inside the
picture, on all three shots. The recovery was a per-clip
[`crop`](project-format.md#showing-part-of-a-source-crop), which costs nothing
to apply and permanently trims a frame that was bought at full size.

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

## Keep colour neutral if the shot will be graded

"Colour film stock, naturally exposed" is a useful sentence: it asks for an
ordinary rendition and leaves the look to be decided afterwards. Asking Veo for
the grade instead bakes it into a file that cannot be regenerated without
paying again, and "a little less brown" stops being an edit and becomes a
purchase.

Today grading afterwards means an ffmpeg pass outside the project, which is its
own problem and is filed as #250. Once a clip can carry a grade, this stops
being a workaround and becomes the recommended split: buy the picture from the
provider, decide how it looks here — for free, reversibly, and animatable
across a shot rather than fixed for its whole length.

## Veo 3.1 generates sound whether the prompt mentions it or not

A prompt that says nothing about audio does not come back silent. It comes back
with whatever the model decided the shot sounds like — room tone, footsteps,
wind, sometimes voices — chosen on its behalf and paid for either way.

Since it is arriving regardless, say what it should be. A prompt that names its
own sound is not asking for an extra; it is taking a decision that would
otherwise be made by something that has not read the rest of the cut.

## Veo outputs 24 fps

Generated video comes back at 24 fps. A project created at the default 30 will
[conform](project-format.md#conforming-source-fps--timeline-fps) every one of
those shots by repeating source frames in the 2:3 pattern — so the judder lands
on exactly the footage that cost money, on a timeline whose rate was never
really chosen.

That is a reason to decide it: `scorsese new film.scor --fps 24` for a cut
built mostly out of generated shots. The
[timeline framerate](project-format.md#the-timeline-framerate) is chosen at
project creation and is not a field edit afterwards, so it is worth a moment at
the start rather than a rescale later.

## Adding to this page

An entry belongs here when it is a **fact about what a provider does** that
cost a generation to learn, and it should say what happened rather than only
what to do. A rule with no incident behind it is advice, and this page is not
for advice — the incident is what lets the next reader tell whether their case
is the same one.

The gap this page fills has a mirror image on the free side: #189 says
`docs/recipes.md` describes how a source is *built* and never what it *sounds
like*. Both are the same shape of missing. A document can specify every field
exhaustively and still not say what putting a particular word in one of them
does, and only one of those two omissions can be discovered without a bill.
