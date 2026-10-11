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
there — `guide pages` for a page, `guide project-format` for the document,
and for the score `guide recipes` (how to write one) and `guide references`
(which record to hold it against, and what that record measures).

**Every section comes from a video actually made**, and names it, the way
`guide prompts` keeps what was learned by paying. A kind of video gets its
section after the first video in it, never before: a recipe nobody has
followed yet is a guess.

There are no tools per kind of video, on purpose. Every tool is in every model
call's tool list, so a tool for each genre would be paid for by every call
that is not making one; this page costs nothing until it is read. The
operations stay generic, reusable motion goes in the page kit and the worked
pages, and how a kind of video is made goes here.

## Starting from a style

This page has a section for each kind of video somebody has made. The
**style library** is longer: fourteen kinds, compiled in
(`crates/core/src/style/library.rs`), each with a placement list and a long
prompt saying how that kind is made and what to ask the person for. A project
starts from one with `scorsese new --style <id>` or `project_new`'s `style`,
and from a placement with `--platform` / `platform`; either or both write a
brief into the project's `script.md`, and a style not made for the placement
asked for is refused with the ones that are.

| id | the video | made for |
| --- | --- | --- |
| `narrated_captions` | the narration as the text on screen, over support footage | every vertical placement |
| `kinetic_type` | big words moving to the music, no footage | everywhere |
| `whiteboard` | a board that draws itself under a narration | YouTube, Shorts, Reels, TikTok |
| `flat_explainer` | flat illustrated characters acting out an idea | YouTube, Shorts, Reels, Reels ad, TikTok |
| `product_tour` | real product screens, one by one, narrated | YouTube, Shorts, the three ads |
| `photo_montage` | the person's photos, slow push-ins, cut to music | YouTube, Shorts, Reels, Stories ad, TikTok |
| `top_list` | a countdown, one item a scene, best last | YouTube, Shorts, Reels, TikTok |
| `before_after` | the problem, then the result | Reels, TikTok and the three ads |
| `narrated_documentary` | cinematic pictures under a told story | YouTube, Shorts, Reels |
| `testimonial` | customers' words on cards, then the call to action | YouTube and the three ads |
| `pov_hook` | a meme-style "POV: …" hook over footage | Shorts, Reels, TikTok, TikTok ad |
| `flash_offer` | product, price, urgency and the buy button, in seconds | the three ads |
| `step_by_step` | a tutorial, one numbered step a scene | YouTube, Shorts, Reels, TikTok |
| `numbers_story` | animated figures and charts carrying the argument | YouTube, Shorts, Reels ad |

The placements are `youtube`, `youtube_shorts`, `instagram_reels`, `tiktok`
and three ads: `instagram_reels_ad`, `instagram_stories_ad` and `tiktok_ad`. A placement also sizes the render (`render --platform`). Where a
style has a section below, the section is what a video in it has actually
taught; read both.

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
captions: it writes its dips where the narration is now. Its defaults are the
level to start from: a quarter of the bed's own level under every line (about
−12 dB), down in 0.3 s and back up over 0.6 s.

**The score is a bed under talk.** Write it as a recipe (`guide recipes`)
with a `fit` to the clip that plays it, so it lasts exactly as long as the cut,
and hold its bake report against the reference `guide references` gives for a
groove under talk (*Billie Jean*).

**What the Stories ad's score was.** *From the scorsese Stories ad
(`scorsese-story-15s.scor`, 2026-10-10), 15 s upright, of the same kind
except that the screen says the message in other words rather than captioning
the voice.* 120 bpm in A♭ major, 30 beats for exactly 15 s, fitted to its clip
with `stretch` and four sections anchored to the clips where the chat, the
timeline, the second shot and the call to action start, so every cut is on a
beat. The groove is full from frame 0 (kick, clap, syncopated bass, stabs, pad)
with no intro, has a small drop on the call to action and ends on a held
A♭(add9) on the last frame. Held against *Billie Jean*: 79/19/2 % low/mid/high,
crest 12.5–13.4 dB (more squeezed than the record's ~15.5, on purpose for an
ad), corr +0.87. Under a voice at about −18.7 LUFS the music sat at 0.25 (about
9 dB below), rose to 0.5–0.6 in the pauses and to 0.9 after the last line. That
curve was written by hand: with pauses of about 0.3 s between lines,
`duck_music` made one dip from the first frame to the last line.

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

**Music under everything, ducked.** A recipe (`guide recipes`) fitted to the
clip, ducked with `duck_music` against the narration as in a narrated ad.

**What its score was.** The delivered cut is the second version
(`verdade-liberta-v2.scor`, 2026-10-10): 76 bpm in D major, with piano, strings,
cello, harp, celesta and flute in a room. It has seven sections, one per spoken
line: the first starts with the cut and each of the others is anchored to its
line's clip. It is fitted to its clip with `once`. The harmony follows the
script: B minor when the listeners object, a lament bass and a slow low drum
under the prison, up a tone to E major for the freedom line, ending on a held
E(add9). None of the references is calm or church music, so it was held against
the nearest, *Fuego, breaks*: crest 12–17, low 27–38 % (64 % in the prison, on
purpose), and narrower than the reference (corr +0.8 against +0.2 to +0.5). It
was ducked with `duck_music` at 0.4 against the narration, with the default
attack and release. The pencil scratches sit on their own track at 0.16 and are
never ducked. The first version's score was simpler: 72 bpm in D major, "light
and low, just so there is no silence", stretched to the 65.74 s cut.

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
fixed beat, and `cut_to_voice` lays that out.

**Neutral music under it, ducked.** A recipe (`guide recipes`) fitted to the
cut, held against the reference `guide references` gives for a groove under an
explainer (*Billie Jean*), and ducked with `duck_music` as in a narrated ad.

**Close on where to get it.** The product's name and where to find it, held
long enough to read.

## Technical explainer: a diagram that builds with the narration

*From the hivemind explainer (`hivemind-explained.scor`, 2026-10-10, 16:9,
198 s) and the Dataserver explainer (`dataserver-video.scor`, 2026-10-01).*

**What it is.** A narrated explanation of how a piece of software works, drawn
as one diagram whose parts keep their places from scene to scene while each
mark arrives on the word that names it. A person asks for it as "explain how
this works", "a video of the architecture". The hivemind cut is one transparent
page per spoken line (`guide pages`), sharing one script and one stylesheet,
on two scene tracks that alternate and crossfade over a single plain ground.
The Dataserver cut was drawn in native shapes, which are frozen now: make the
next one in pages.

**Start without a score, or keep it far down.** The maintainer took the
hivemind score out on review because it got in the way of hearing the
narration, and that cut was delivered with no music. Dense technical
narration is the case where a bed costs more than it gives. What was tried, for
the next attempt: hivemind's was D minor, written at 98 bpm, 82 bars fitted to
the clip with `stretch` and seven sections anchored to the scene pages, so the
tempo moved between about 92 and 100. A drop-out bar landed on a laptop closing
and a lift on its return. Held against *Billie Jean*: mean −16.2 dBFS, 82/16/2 %
low/mid/high, corr +0.85, body crest 12–13 (the record's ~15.5). It sat at 0.5
between lines and 0.2 under them, set by hand after `duck_music` at 0.28,
because the bed at full level between lines made the render's limiter work
7 dB. Dataserver's was 96 bpm in D minor (pad, sub, plucked arpeggio, bass, kick,
hat) building from pad alone to the full groove and back. It was not fitted:
its clip ends it at the cut, fading to silence over the last second. It was held
flat at 0.24 under the whole narration, at 0.6 before the first line and after
the last.

## Teaser trailer: generated shots cut on a beat grid

*From a teaser for a film that does not exist (`cyberpunk-2077-teaser.scor`,
2026-10-10, 16:9, 64 s).*

**What it is.** A trailer: generated shots, title cards and a score the whole
cut is made to, with no narrator. The only spoken lines are the characters'
own, generated with their shots. The timeline is 24 fps and the score 120 bpm,
so a beat is exactly 12 frames and every cut is on one. The cards are pages
sharing one glitch, and one character's face holds across shots by drawing a
character sheet first, then first frames from it, then the shots from those
frames (`guide prompts`).

**The score is the cut's spine.** E minor at 120 bpm, 18 sections, four of them
anchored to the cards (the first card, "this winter", the title, the date). It
has no `fit`, because the cut was laid on the same grid. It has a braam on every
card with a riser into it, a half-time beat in the second act, a four-on-the-floor
montage, silence with a heartbeat under the lead's line, and the drop on the
title. It was held against *Fuego* (a build to a payoff): 71/26/3 %
low/mid/high, body crest 12–14 dB, and the first act came out narrow (corr about
0.9). It plays at 0.85, and at 0.9 from the title on, with dips to 0.4, 0.6
and 0.5 under the three spoken lines, written by hand because the lines are
inside video clips and `duck_music` ducks only under audio tracks.

## Animated short: a picture book with dialogue

*From* Every Morning *(`every-morning.scor`, 2026-10-10, 16:9, 50 s), made
without Veo.*

**What it is.** A short story told by characters, with no narrator and no
captions. Each shot is a painted background plate with the characters drawn on
a green screen and keyed in with `chroma_key`, on layers that a camera moves at
different depths. Every character has their own dialogue track, so lines can
overlap.

**The score follows the arc, a section a beat of the story.** 72 bpm, D major
and B minor, nine sections anchored to the scene cuts and to two lines (the
swell lands on the line that names the title, the first major chord on the
turn). It is not fitted: it rings out after the last section. It opens on a
music box alone, adds cello and pizzicato, swells onto the title line and leaves
one violin note in the silence, goes silent while the clocks strike, and ends on
the music box alone again. It was held against *Fuego, breaks*: crest 12–17 dB
(ref 13–17.5), low 4–54 % (ref 9–39, high in the cello sections), and narrower
than the reference (corr +0.6 to +0.87 against +0.2 to +0.5). It was ducked with
`duck_music` against both dialogue tracks at 0.45, with a 0.25 s attack and a
0.8 s release, and the dialogue clips play at 1.6.

## Music-led promo: pages cut to the score, no voice

*From the scorsese for teams promo (`scorsese-b2b-promo.scor`, 15 s, and
`scorsese-b2b-promo-30s.scor` and `-mobile.scor`, 26 s in 16:9 and 9:16,
2026-10-08).*

**What it is.** A short promo with no narration: a few transparent scene pages
over one continuous background page, so the cuts never show a seam, each cut
on a downbeat or a bar line of the score. The music is the only sound, so it
plays at full level and nothing ducks it.

**The score is written to length.** 120 bpm in C major, with drums, bass, keys,
pad, pluck, lead, crash, boom and risers. It is fitted to an exact length with
`once` and `tail: exact` (15 s with a 1.5 s fade, 30 s with 2 s), and a pluck's
filter sweeps open and shut across it. It goes from an intro, to drums and
harmony on a hit, to a lift with the lead, a riser and a crash, to an outro. The
15 s cut changes scene at 0, 3, 7 and 11 s. The 26 s cuts play the 30 s score in
two clips with its 20–24 s breakdown left out, because the scene it scored was
dropped, so the build ending at 20 s runs into the logo hit on a bar line. The
9:16 cut also changes a benefit on two of the score's accents.
