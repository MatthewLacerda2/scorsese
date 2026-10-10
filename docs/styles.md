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
by bringing a script and nothing to film. Most feed viewers watch with the
sound off, which is why the words are the picture's job too.

**Write the script first, and agree it.** Everything else is cut to the
voice, so a changed line later is a re-cut. Short spoken sentences, a hook in
the first two seconds that states the payoff, one clear close.

**The voice drives the cut.** Generate the narration as one line per scene
(`asset_set` with a spoken brief, then `generate`), because a generated line
keeps the timing of every word it says, and those timings are what the rest of
the cut is placed on. Each scene ends **0.2 s after its line's last word**, and
the next scene starts there. Not at the end of the clip: a generated line has
a silent tail, and cutting on the clip's end leaves a pause after every
sentence that reads as hesitation. Lay the scenes out from the line lengths,
reading where each last word ends from `project_describe` at the line's end,
and move them with `clip_move`.

**Two overlapping scene tracks.** Put the scenes on two video tracks,
alternating, so a scene can run on a little under the next one's arrival
instead of the cut landing on a hard edge. One track forces every scene to
end exactly where the next begins.

**Each chunk of text arrives on its first word, whole.** `caption_narration`
does exactly this: it cuts each line at its sentences, commas and pauses into
pieces of about two lines, and each piece arrives whole on its first spoken
word and leaves when the next arrives. Its default look is the one this ad
landed on: heavy Montserrat, white, rimmed in black, in the lower third, clear
of the buttons a phone draws over the bottom of a reel. Run it **after** the cut is final: it places captions where the words
are now, and a line moved afterwards leaves its captions behind until it is
run again. A page that has to land on a word reads the same timings
(`guide pages`, section "Start on a spoken word").

**The music is ducked.** Lay the music bed under the whole cut, then
`duck_music` against the narration track, last, for the same reason as the
captions: it writes its dips where the narration is now.

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
fixed beat. Neutral music under it, ducked with `duck_music`.

**Close on where to get it.** The product's name and where to find it, held
long enough to read.
