//! What each style tells the assistant: how this kind of video is made.
//!
//! Each is written to be read cold by whoever does the editing, on any
//! surface: what the video is, its shape from first second to last, its
//! pacing, and — first, so it is never hidden — what it needs from the
//! person. A style that needs footage the person did not bring is still
//! offered, with that said. Craft that has a guide is left to the guide.

/// [`narrated_captions`](super::STYLES).
pub(super) const NARRATED_CAPTIONS: &str = "\
Narrated captions: the narration is the on-screen text. Each chunk of a few words arrives \
on its first spoken word and leaves when the next arrives, over b-roll; the voice drives \
the cut, so every picture change lands on a word.\n\
Needs from the person: the message, or a script to read. B-roll is theirs if they bring \
it, stock otherwise; the voice is theirs recorded, or generated.\n\
Shape: a hook in the first two seconds that states the payoff, then the argument in short \
spoken sentences, then one clear close (a call to action on an ad). Write the script \
first and agree it before building anything.\n\
Make: the narration, then its words, then the captions from those words — large, centred \
in the safe middle of the frame, two to five words a chunk, the current word picked out. \
Cut the b-roll to the chunk boundaries, one shot every one to three seconds. Music sits \
low and ducks under the voice.";

/// [`kinetic_type`](super::STYLES).
pub(super) const KINETIC_TYPE: &str = "\
Kinetic typography: bold words arriving on the beat, the type itself the picture. No \
footage is needed.\n\
Needs from the person: the message and a mood; a brand colour and typeface if they have \
them. Music is theirs, or synthesised.\n\
Shape: a short line a beat, building to the one sentence the video exists to say, held \
longest, then the close. Thirty words carry a fifteen-second video; cut words, not \
beats.\n\
Make: the music first, because its beats are the grid; place each word or phrase on a \
beat, one to four words on screen at once, big and high-contrast on a plain or slowly \
moving ground. Vary scale and position between beats so the eye is pulled, but keep one \
typeface and two colours. Motion graphics are pages (`guide pages`).";

/// [`whiteboard`](super::STYLES).
pub(super) const WHITEBOARD: &str = "\
Whiteboard: a white board that draws itself under the narration, each drawing appearing \
stroke by stroke as the voice reaches it.\n\
Needs from the person: the idea to explain, or a script. Nothing to film.\n\
Shape: a question or a problem, then the explanation in three to six beats, each a \
drawing that stays on the board as the next is added, ending on the whole board as the \
summary.\n\
Make: write and agree the script, record or generate the narration, then draw each beat \
as a page whose strokes are timed to the words that name them (`guide pages`). Dark ink \
on white, one accent colour, simple line art; the camera may push in to the drawing \
being made and pull back for the summary. Light music, well under the voice.";

/// [`flat_explainer`](super::STYLES).
pub(super) const FLAT_EXPLAINER: &str = "\
Flat illustrated explainer: flat, friendly characters (Humaaans-style) and simple sets \
acting out an idea while a narrator explains it.\n\
Needs from the person: the idea, who it is for, and their brand colours if any. No \
footage; the illustrations are drawn as pages, or generated if the person prefers and \
agrees the price.\n\
Shape: a character with a problem the viewer recognises, the turn where the idea or \
product arrives, the character's life after, and a close that names what to do next.\n\
Make: the script first, then the narration; one scene per sentence or two, each scene a \
page with a few flat shapes that move a little (a wave, a nod, an icon popping in), cut \
on the narration's sentences (`guide pages`). Keep a palette of four or five colours \
across every scene. Upbeat music under the voice.";

/// [`product_tour`](super::STYLES).
pub(super) const PRODUCT_TOUR: &str = "\
Product tour: the product's real screens, shown one feature at a time, under an \
informative narration.\n\
Needs from the person: screenshots or screen recordings of the product — this style \
cannot be made without them, so ask for them first — and what each screen does.\n\
Shape: the problem the product solves in one line, then three to five features each \
shown on its own screen, then where to get it.\n\
Make: the script follows the screens, a sentence or two each. Show every screen large, \
framed inside a device or a soft panel, and push in slowly to the part being named; a \
short label or highlight marks it. Cut on the narration's sentences. Neutral music, \
ducked under the voice. On an ad, the product and the call to action both appear in \
the first five seconds and again at the end.";

/// [`photo_montage`](super::STYLES).
pub(super) const PHOTO_MONTAGE: &str = "\
Photo montage: stills, each with a slow push-in or drift, cut to the music.\n\
Needs from the person: the photos — this style is made of them, so ask first — and the \
occasion or story they tell. Music is theirs, from stock, or synthesised.\n\
Shape: an opening image that sets the subject, the rest in the order of the story (often \
time), and a closing image held a little longer, with a title or a line if one helps.\n\
Make: the music first; cut on its beats, a still every one to four beats, faster in the \
energetic parts. Each still gets a slow scale from 100% to about 108% or a gentle pan, \
alternating direction so the montage breathes. Fit each photo to the frame without \
stretching it; a short dissolve between stills suits a sentimental piece, a hard cut an \
energetic one.";

/// [`top_list`](super::STYLES).
pub(super) const TOP_LIST: &str = "\
Top N list: a countdown, one item a scene, the best kept for last.\n\
Needs from the person: the topic and the items (or ask Claude to propose them), plus any \
footage or images of the items; stock fills what they do not have.\n\
Shape: a hook naming the list and promising the number one, then each item from N down \
to 1 — a big number, the item's name, one reason — then the number one given the most \
time, then the close.\n\
Make: write and agree the list and the narration, then one scene per item of equal \
length, the number large in a corner or centre as the scene opens. Keep the number's \
look identical across items so the count reads at a glance. Music with a steady pulse; \
a short sting or whoosh on each new number.";

/// [`before_after`](super::STYLES).
pub(super) const BEFORE_AFTER: &str = "\
Before and after: the problem, then the result, side by side or cut, so the difference \
speaks for itself.\n\
Needs from the person: footage or photos of the before and the after — the style is \
the comparison, so ask for both — and what changed between them.\n\
Shape: the before shown long enough to feel (two or three seconds), a clear marker of the \
change (a wipe, a flash, a word), the after held longer, then what made it happen and, on \
an ad, the call to action.\n\
Make: label the two states plainly (\"Antes\" / \"Depois\" in the person's language), at \
the same place in the frame both times. Match the framing of the two shots where you can \
so only the change moves. Put the reveal on a strong beat of the music.";

/// [`narrated_documentary`](super::STYLES).
pub(super) const NARRATED_DOCUMENTARY: &str = "\
Narrated documentary: cinematic b-roll under a storytelling voice, with room to breathe.\n\
Needs from the person: the story, or the subject to research it from; footage if they \
have it, stock or generated otherwise (quote the price before generating).\n\
Shape: a cold open on a striking image and line, then the story told as a sequence of \
moments, each a few shots, with pauses where the music carries it alone, and a closing \
thought held over a final image.\n\
Make: write and agree the script first, then the narration — calm, unhurried — then lay \
the b-roll on it, longer shots than a feed video (three to six seconds), slow moves, \
dissolves between moments. Lower-third titles for places and dates. Music is the score: \
it swells in the pauses and ducks under the voice.";

/// [`testimonial`](super::STYLES).
pub(super) const TESTIMONIAL: &str = "\
Testimonial: customer quotes and reviews as cards, one at a time, closing on the call to \
action.\n\
Needs from the person: real quotes or reviews, with the customer's name and, if they \
have one, a photo — never invent a testimonial, ask for them — plus the product and its \
call to action.\n\
Shape: a hook (the strongest quote, or the result customers got), three to five quotes \
each on its own card, a rating or count if there is one, then the product and the call \
to action.\n\
Make: each card is a quote in large type with the name small beneath it, star rating or \
photo beside it, on the brand's colours; hold each card long enough to read twice. A \
narrator may read the quotes, or the music carries them alone. On an ad, the product \
appears in the first five seconds.";

/// [`pov_hook`](super::STYLES).
pub(super) const POV_HOOK: &str = "\
POV: a meme-style hook line over footage (\"POV: …\"), the text doing the work while the \
picture plays out the situation.\n\
Needs from the person: the situation to joke about or relate to, and footage that plays \
it out — their own is best; stock or generated otherwise.\n\
Shape: the POV line on screen from the first frame, the footage playing the situation, \
an optional second line for the twist, and, on an ad, the product as the punchline.\n\
Make: the hook text in the platform's native look — plain bold sans, white with a dark \
outline or a white box behind, in the upper third, clear of the interface. Usually one \
take or a few quick cuts, eight to fifteen seconds, with a trending-feel track or the \
original sound. Write the line short enough to read in under two seconds.";

/// [`flash_offer`](super::STYLES).
pub(super) const FLASH_OFFER: &str = "\
Flash offer: the product, the price, the urgency and the call to action, in a few \
seconds.\n\
Needs from the person: the product (photos or footage), the offer exactly as it is — \
price, discount, deadline — and where to buy. Never invent a price or a deadline: ask.\n\
Shape: the product and the offer in the first second, the price large, the urgency \
(\"Só hoje\", a countdown) and the call to action, then the offer repeated at the end. \
Six to fifteen seconds.\n\
Make: high-energy music with a clear beat; cut on it. The price is the largest thing on \
screen, in a contrasting colour; an old price struck through beside it if there is one. \
Keep text in the safe middle of the frame, clear of the platform's buttons. Every \
element pops in on a beat.";

/// [`step_by_step`](super::STYLES).
pub(super) const STEP_BY_STEP: &str = "\
Step by step: a tutorial, one step a scene, each numbered and shown while it is \
explained.\n\
Needs from the person: what is being taught, and footage or screen recordings of each \
step being done — ask for them; without them the steps are drawn or illustrated instead.\n\
Shape: what the viewer will be able to do by the end, the steps in order, each with its \
number and one instruction, then the finished result.\n\
Make: write and agree the steps and narration, then one scene per step, its number and \
a short label on screen for the whole scene, the footage cut to what the narration \
describes. Keep the number's look the same on every step. Calm music, low under the \
voice.";

/// [`numbers_story`](super::STYLES).
pub(super) const NUMBERS_STORY: &str = "\
Numbers that tell: animated figures and charts carrying the argument, one figure at a \
time.\n\
Needs from the person: the figures and where they come from — never invent a number; \
ask — and the conclusion they support.\n\
Shape: the most surprising figure first, as the hook, then each figure that builds the \
case, a scene each, then the conclusion and what to do about it.\n\
Make: write and agree the script, then the narration; each figure counts up to its value \
or its chart draws as the narration names it, large, one idea per scene, the source in \
small type beneath (`guide pages` has the motion). Two or three colours, the accent kept \
for the figure that matters. Music steady and understated.";
