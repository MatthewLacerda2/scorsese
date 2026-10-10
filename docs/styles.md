# Styles — how each kind of video is made

A person rarely asks for "a timeline with two video tracks and a ducked music
bed". They ask for *a Reels ad like the ones with the words on screen*, or *a
video showing off my app*, or *one of those whiteboard videos*. Each of those
is a **kind of video**, with a shape that is the same every time it is made
well. This guide is that shape, one section per kind.

**Read only the section for the video in hand.** Every section is the whole
recipe for its kind: what it is and when a person asks for it, how the
timeline is laid out, which tools and pages it uses, the timing that makes it
feel right, and the traps already paid for. A section is about *what* and
*why*; the *how* stays in the guide that owns it, and the section points
there — `guide pages` for a page, `guide project-format` for the document.

**Every section comes from a video actually made**, and names it, the way
`guide prompts` keeps what was learned by paying. A kind of video gets its
section after the first video in it, never before: a recipe nobody has
followed yet is a guess.

There are no tools per kind of video, on purpose. Every tool is in every model
call's tool list, so a tool for each genre would be paid for by every call
that is not making one; this page costs nothing until it is read. The
operations stay generic, reusable motion goes in the page kit and the worked
pages, and how a kind of video is made goes here.

## Narrated captions: a vertical ad with the narration as the text

*From the DataForce Reels ad (`dataforce-reels.scor`, 2026-10).*

**What it is.** An upright video for a feed — Reels, Shorts, TikTok, often
paid — where the voice says the message and the same words are on screen as
it says them. The picture behind is support: b-roll, a product, a page. A
person asks for it as "a Reels ad", "one of those videos with the text", or
by bringing a script and nothing to film.

**Write the script first, and agree it.** Everything else is cut to the
voice, so a changed line later is a re-cut. Short spoken sentences, a hook in
the first two seconds that states the payoff, one clear close.

**The voice drives the cut.** Generate the narration as one line per scene
(`asset_set` with a spoken brief, then `generate`), because a generated line
keeps the timing of every word it says, and those timings are what the rest of
the cut is placed on. Each scene ends **0.2 s after its line's last word**, and
the next scene starts there. Not at the end of the clip: a generated line has
a silent tail, and cutting on the clip's end leaves a pause after every
sentence. `cut_to_voice` does this arithmetic: name the scenes in order, each
its line and the visuals seen while it is said, with `gap_seconds: 0.2`. Re-run
it whenever a line is regenerated or re-worded.

**Two overlapping scene tracks.** Give `cut_to_voice` an `overlap_seconds`
and each scene's visuals run on a little under the next one's arrival, so an
exit and an entrance play together instead of the cut landing on a hard edge.
The scenes end up alternating between two video tracks; that is the layout.

**Each chunk of text arrives on its first word, whole.** `caption_narration`
does exactly this: it cuts each line at its sentences, commas and pauses into
pieces of about two lines, and each piece arrives whole on its first spoken
word and leaves when the next arrives. Its default look is the one this ad
landed on: heavy Montserrat, white, rimmed in black, in the lower third, clear
of the buttons a phone draws over the bottom of a reel. Run it **after** the
cut is final: it places captions where the words are now, and a line moved
afterwards leaves its captions behind until it is run again. A page that has to land on a word reads the same timings
(`guide pages`, section "Start on a spoken word").

**The music is ducked.** Lay the music bed under the whole cut, then
`duck_music` against the narration track, last, for the same reason as the
captions: it writes its dips where the narration is now.

## Whiteboard: a board that draws itself

*From a church video (`verdade-liberta.scor`, 2026-10-09), made in the manner
of "Desenhando a Bíblia".*

**What it is.** A narrated explainer where drawings and words appear stroke by
stroke on one large board, in step with the voice, while a camera moves from
one part of the board to the next. A person asks for it as "a whiteboard
video", "one of those drawn explanations", or names a channel that makes them.
Nothing is filmed: the board is a page, and the narration carries it.

**Write the script first, and agree it.** Each beat of the script is a scene:
a region of the board with its drawing and its words. Generate the narration
before drawing anything, because the drawing is timed to its words.

**The board is a page, built on the kit.** `kit.draw` draws any SVG stroke by
stroke, `kit.write` sets text as outlines with one group per word, named the way
the narration's words are, and `kit.camera` moves a canvas larger than the frame
under a fixed camera. The worked page is the starting point: copy it and grow
it (`guide pages`, section "A board that draws itself"), and read the kit's
table before writing more (`guide pages`, section "The kit").

**People are drawn from parts, never as stick figures.** A modern person (at a
desk, in the street, using an app) is put together from the shipped Humaaans
with `kit.person`, recoloured, and drawn with `kit.draw` (`guide pages`,
section "People, put together and drawn on"). A period or biblical figure, like
this video's, is a generated picture traced with `vectorize` instead.

**Every mark is anchored on a spoken word.** A drawing starts when the voice
names it and a word is written as it is said, from `scorsese.words`. A mark
timed in plain seconds drifts the first time a line is re-generated; one
anchored on a word moves with it. The worked page falls back to following the
previous mark when a word has no timing, so nothing goes missing.

**One constant stroke speed.** Draw at `kit.draw`'s `speed`, the same for the
whole board, so a long line takes longer than a short one, as a hand would,
rather than fitting each drawing into a fixed time.

**The camera moves ahead of the line.** The camera arrives on the next scene
just as its first mark starts, so a line's opening words, said before the word
that mark is anchored on, play over the scene it is leaving.

**Hold about a second before each move.** Let a finished scene sit still for
about 1 s before the camera leaves it, so it can be read whole.

**Pull out at the end.** The last move goes back to the whole board, every
scene drawn, as the summary, held to the end of the clip.

## Product tour: real app screens under an informative narration

*From the Truss Cockpit video (`truss-cockpit-promo.scor`, 2026-10).*

**What it is.** A presentation of a piece of software, made of the product's
own screens shown one feature at a time while a narrator explains what each
part does. A person asks for it as "a video of my app", "a demo", "a launch
video". It cannot be made from stock or a generation: a generated screen is a
screen of a product that does not exist.

**Get the real screens first.** Screenshots or screen recordings from the
person, or, when the app runs where you are working and they agree, captured
from it with a headless browser, as the Truss Cockpit screens were, with
puppeteer, then imported like any other media.

**Informative, not ad copy.** The narration says what the product does and
how, in plain sentences, a feature a scene: what this screen is, what you do
on it, what you get. The first draft of the Truss Cockpit narration was written
as a TV ad — slogans, urgency, superlatives — and the person rejected it; the
one that was kept *explained*. Agree the script before generating any of it.

**One screen a scene, cut on the sentences.** Each screen is shown large, in
a soft panel or a device frame, with a slow push-in toward the part being
named, and a short label or highlight marking it. The voice decides when a
screen changes, the same way as in a narrated ad (the section above): the next
screen arrives shortly after the sentence about the last one ends, not on a
fixed beat, and `cut_to_voice` lays that out. Neutral music under it, ducked with `duck_music`.

**Close on where to get it.** The product's name and where to find it, held
long enough to read.
