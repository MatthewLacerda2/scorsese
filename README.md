# scorsese

**A video editor you talk to.** You bring the idea and whatever footage you
have; an AI assistant — Claude Code, Codex, anything that speaks MCP — does the
editing: the cuts, the titles, the narration, the music, the pacing, the render.
Where the video needs a shot nobody filmed or a line nobody recorded, scorsese
generates it, and tells you what that will cost before it spends a cent.

Named as a nod to Martin Scorsese.

## What it is for

Making a real, finished video without learning an editing program and without
paying a studio — a product teaser, an opening film for an event, a technical
explainer, a narrated walkthrough.

You describe what you want in plain language and go back and forth, the way you
would with a human editor: *"open on the pit lane, hold the title two seconds
longer, bring the music down under the voice, the second shot feels slow."* The
assistant edits, looks at its own frames to check the work, and shows you.

Three videos made this way, to give the scale of it:

| The video | What was generated | What it cost |
| --- | --- | --- |
| A 90-second opening film for a company event — five scenes, period footage, narration, score | 17 Veo shots, 5 narration lines, synthesised music | **about $16** |
| A 9-minute technical explainer — diagrams, captions, icons, narrated end to end | 14 narration blocks; every picture is drawn, none generated | **$0.22** |
| A one-minute title piece with an original score | nothing — text, colour and synthesised music | **$0** |

Those figures are scorsese's own estimates from the providers' published rates,
not a bill — no provider reports back what a generation cost. The people who
watched those videos called them good; cheap and easy is what they were to make.

## How it helps

**Generation is cheap, as long as it is deliberate.** That is the idea the whole
tool is shaped around:

- **Sketch first, for free.** A shot or a narration line starts as a *sketch*:
  just its description. A sketch renders as a card showing its text, so you can
  watch the entire cut — timing, order, titles, music — before anything is
  bought.
- **Then say GO.** Generation realises only what is still a sketch. It prints
  the quote and asks first. A spending ceiling you set is refused even when the
  assistant is running unattended.
- **Never pay twice.** Whatever was generated is kept and keyed to its
  description. Re-running costs nothing; only a description you actually changed
  is bought again.
- **Most of a video is not generated at all.** Titles, captions, shapes, arrows,
  icons, colour cards, transitions, colour grading and music are all made
  locally — free, instant, and identical every time.

**The assistant can see and hear its work.** It can pull a single frame of the
edit as an image, tile a clip into a contact sheet, draw a sound as a waveform,
and read the whole cut back shot by shot — all without rendering. So it catches
a title that runs off the frame or a line that overlaps the next one before you
do, and you are only asked to watch when there is something worth watching.

**A project is just a folder.** `myvideo.scor/` holds one readable
`project.json`, the media, and the script the edit is being cut from. Copy it to
another machine and it still works. Come back to it next month and the assistant
reads the script and picks up where you stopped.

## What it does

**Editing.** Any number of video and audio tracks. Place, trim, move and layer
clips; crop them, fit or fill the frame, speed them up or slow them down;
dissolve between shots; stretch or tighten the pacing of a whole section around
one moment. Anything numeric can be animated over time — position, scale,
rotation, flips, opacity, blur, volume, every grading control.

**Titles and graphics.** Text with built-in fonts (or your own font file),
weight, italics, outlines that keep a caption readable over busy footage, and
emoji. Rectangles, ellipses and arrows — including arrows that follow a moving
clip. Seventeen hundred built-in icons, found by a word. Solid colour cards.

**Looks.** Colour grading (saturation, temperature, brightness, contrast,
vignette, grain), blur, lens fringing, green-screen removal, and a VHS look.
All applied in the edit, so changing your mind is free.

**Generated footage.** Shots from Google Veo, at 720p or 1080p, landscape or
vertical, optionally guided by a starting image, an ending image, or reference
images that keep a character or an object consistent from shot to shot.

**Narration.** Lines read by ElevenLabs voices, in whatever language you write
them. Browse the voices, or design a new one from a description.

**Music and sound effects, from nothing.** A built-in synthesiser turns a
written recipe — chords, rhythm, instruments, a build — into a score or an
effect, fitted to the length of your cut. No key, no network, no cost, and no
licence to worry about. Or import any audio file and use that.

**Sound mixing.** Volume fades between any two points, music that ducks itself
under narration, the sound of your own footage mixed in or muted, and a
loudness report for the finished file.

**Delivery.** Resolution, frame rate and bitrate are chosen per render, never
baked into the project — the same edit goes out as 1080p for a screen and as a
small file for a chat. MP4 by default; MKV, AVI and WMV when something
asks for them.

What it is **not**: a compositing or VFX suite. No node graphs, no motion
tracking, no rotoscoping. It makes and edits videos, and stays approachable.

## Getting the most out of it

Everything here was learned by making the videos above — some of it by paying
for the mistake first.

- **Start from a script.** Even a rough outline. Ask the assistant to save it in
  the project; it becomes the thing every later session reads first, and the
  thing you argue with instead of arguing with a timeline.
- **Approve the sketch before you buy anything.** Watch the cut made of cards.
  Order and timing problems found here cost nothing; found after generation,
  they cost shots you no longer need.
- **Ask for the quote, and set a ceiling.** *"What would generating this cost?"*
  is free to answer. A budget ceiling in your settings means an assistant left
  working overnight cannot overspend.
- **Buy content, edit looks.** Ask Veo for what only it can give you — the
  scene, the action, the era, the lens. Do the rest in the edit: a colour grade,
  a crop, a square frame, a VHS look are all free and reversible there, while a
  look baked into a generated shot costs money to change. The one exception is
  slow motion: ask for it in the shot, because slowing a clip down afterwards
  only repeats frames.
- **Describe the picture, never the medium.** Ask for *"shot on 16mm film"* and
  you may get sprocket holes drawn inside the image. *"1970s documentary
  footage, fine grain"* gets the look without the object.
- **Keep text out of generated shots.** Name a readout or a sign and it gets
  drawn, misspelled. Ask for *"no readable text"* and put the words on in the
  edit, where they are crisp and editable.
- **Give it references, and clean them first.** A reference image is how a car
  or a face stays the same across shots. The model copies what it sees —
  including a watermark — and telling it not to does not help.
- **Buy shots at full length and cut them down.** 1080p shots come at eight
  seconds; use the best four. If most of your video is generated, ask for a
  24 fps project, since that is what Veo delivers.
- **For narration, the model matters more than the voice.** An English voice on
  the `expressive` model reads Portuguese naturally; the same voice on
  `standard` has an accent, at the same price. A voice native to your language
  is nice, not necessary.
- **Try the synthesiser before hunting for music.** A score written as a recipe
  is free, fits your cut to the frame, and can be changed by asking — *"darker,
  slower, bring the strings in later."*
- **Ask to see frames, not renders.** A still takes seconds; a render takes
  minutes. Iterate on stills, render when you actually want to watch.
- **Not every video needs generated footage.** The nine-minute explainer above
  is drawn text, shapes and icons under a narration. It cost twenty-two cents.

[docs/prompts.md](docs/prompts.md) is the running record of what the providers
really do with certain words, and [docs/prices.md](docs/prices.md) has the
rates.

## Running it

You need [Rust](https://rustup.rs) and `ffmpeg` on your PATH.

```sh
git clone https://github.com/MatthewLacerda2/scorsese && cd scorsese
cargo build --release

# point your assistant at the MCP server. Claude Code shown — Codex, or any
# other MCP client, takes the same binary as a stdio server
claude mcp add scorsese -- "$PWD/target/release/scorsese-mcp"
```

To generate footage or narration, copy `.env.example` to `.env` and fill in a
Gemini key (Veo) and/or an ElevenLabs key. Neither is needed to edit, title,
score or render — [docs/credentials.md](docs/credentials.md) has the rest,
including the spending ceiling.

Then open your assistant in the folder where you keep your footage and say what
you want to make. Every tool describes itself, so it knows where to go from
there. The same operations exist as a plain CLI — `scorsese --help`.

## Going deeper

| | |
| --- | --- |
| [docs/mcp.md](docs/mcp.md) | every tool the assistant gets, and what each one costs to call |
| [docs/project-format.md](docs/project-format.md) | the `project.json` format — everything a project can say |
| [docs/recipes.md](docs/recipes.md) | writing music and sound effects as recipes |
| [docs/prompts.md](docs/prompts.md) | provider behaviour, learned by paying |
| [docs/prices.md](docs/prices.md) | what generation costs, and why it is always an estimate |
| [docs/output-formats.md](docs/output-formats.md) | the file formats a render delivers |
| [docs/credentials.md](docs/credentials.md) | keys, and the ceiling on what they may spend |

## Contributing

How the code is organised and how work gets merged lives in
[CLAUDE.md](CLAUDE.md); `make help` lists the checks a change has to pass. The
[issue tracker](https://github.com/MatthewLacerda2/scorsese/issues) is the plan.
