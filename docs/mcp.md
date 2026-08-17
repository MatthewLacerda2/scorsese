# The MCP server

`scorsese-mcp` exposes scorsese over the Model Context Protocol, so an
assistant can read a project, change it, and render it — without a person
typing commands.

**MCP is a protocol, not a Claude feature.** This server speaks it to whatever
client is on the other end. Claude is who it is developed and tested against,
not a dependency, the same way an HTTP API does not care whether a browser, a
phone, or curl is calling.

## Pointing a client at it

The server is a plain binary that talks over stdin and stdout. A client spawns
it; there is nothing to configure and no port to pick.

**Working in this repo, point the client at the crate rather than at a built
binary.** In the client's MCP configuration, with the repo as the working
directory:

```json
{ "command": "cargo",
  "args": ["run", "--release", "--quiet", "--bin", "scorsese-mcp"] }
```

For Claude Code that is:

```
claude mcp add scorsese -- cargo run --release --quiet --bin scorsese-mcp
```

A build that is already current is a no-op costing a fraction of a second
before the server starts speaking, and one that is not gets made — so a `git
pull` is picked up on the next client start, with nothing to remember.

**For an installed build, name the binary** — `/path/to/scorsese-mcp`, or
`target/release/scorsese-mcp` for one built here. The tradeoff genuinely
reverses there: a shipped binary should not need a toolchain, a source tree or
a compile to start.

### Why the default is the crate and not the artifact

A client pointed at a built binary keeps launching whatever was last compiled,
and **`git pull` alone changes nothing**. The failure that follows is silent:
no error, no warning, no version mismatch. The session comes up with a smaller
tool list and everything that is present works, so nothing looks wrong.

That is worse than it sounds, because **the tool list is fixed at handshake
time and never re-announced**. "Am I running the current server?" is a question
a client cannot ask from inside the protocol — so a capability that landed on
`main` an hour ago is not merely missing, it is unfindable, and the work routes
around it using whatever older tool is still there. Unattended, that run
finishes and reports success; it was only ever slower and dumber than the repo
it was running from.

## The tools

Every tool takes `project`: the path of the `*.scor` directory to work on — or,
for `project_new` alone, the one to create.

**The table is generated from the tools themselves.** Each row is a tool's name,
the first sentence of the description a client is shown, and what calling it
spends — read out of the registry rather than typed here. `make mcp-table`
rewrites it and `make test` fails when the page has drifted, so a row is never a
claim about the server that the server would not make itself.

Everything *under* the table is written by hand, and has to be: it is about how
the tools relate to each other, which is knowledge no single tool has.

<!-- BEGIN TOOLS. Generated from the registry by `make mcp-table`; edit the tool's description, not this table. -->

| Tool | What it does | Costs |
| --- | --- | --- |
| `project_new` | Create a *.scor project directory: project.json, and the assets/, generated/, recipes/ and cache/ folders beside it. | nothing |
| `project_read` | Read a project's project.json exactly as it is on disk. | nothing |
| `project_describe` | Say what the cut contains, shot by shot and sound by sound. | nothing |
| `project_check` | Report everything wrong or questionable about a project — the document, the media it references, and the layers it draws over each other — without rendering. | nothing |
| `project_assets` | List the media pool: every asset, its kind, what state it is in, and how many clips use it. | nothing |
| `import` | Copy media into the project's assets/ and add it to the assets table, ready for a clip to reference. | ffprobe |
| `project_probe` | Ask ffprobe about every asset that has a file and no recorded metadata, and write down what it says. | ffprobe |
| `script_read` | Read the document this edit is being cut from — the brief, the outline, whatever the project's `script` field points at. | nothing |
| `script_write` | Write the project's script — the document the edit is cut from. | nothing |
| `project_write` | Replace a project's project.json with the document given. | nothing |
| `place_clip` | Put a clip on a track: which asset, which track, when it starts and how long it runs — all in seconds, rounded onto the project's frame grid for you. | nothing |
| `trim_clip` | Move a clip already on the timeline, or change how long it runs or where in its source it opens — in seconds, rounded onto the project's frame grid. | nothing |
| `dissolve` | Dissolve one shot into the next, by writing ordinary opacity keyframes on both clips — the same ones you would place by hand, and they stay editable afterwards. | nothing |
| `duck_music` | Lower a music track while narration plays over it, by writing ordinary volume keyframes on its clips. | nothing |
| `set_volume` | Set how loud one clip plays — a level, a mute, or a fade between two points — by writing the ordinary volume keyframes you would place by hand, which stay editable afterwards. | nothing |
| `scale_pacing` | Move some clips toward or away from one instant, all by the same factor — the operation for pacing. | nothing |
| `synth_new` | Start a new sound: writes a starter recipe into recipes/ and adds the synth_audio asset that points at it. | nothing |
| `synth_read` | Read a recipe file as it is on disk. | nothing |
| `synth_write` | Replace a recipe file with the document given. | nothing |
| `synth_set` | Change one number in a recipe and leave the rest of the document alone: a track's gain, or the recipe's own bpm, seed, swing, duration or velocity. | nothing |
| `synth_check` | Parse a recipe and say what it is, without rendering it. | nothing |
| `synth_bake` | Render every synth_audio recipe whose sound is not already on disk, into generated/. | nothing |
| `synth_survey` | Say what every song recipe in the project is made of, and count the same facts across the whole set. | nothing |
| `audio_level` | Say how a finished sound file came out. | ffmpeg |
| `icons` | Find an icon by a word, and answer with names — each one a string to write as an `icon` asset's `name`. | nothing |
| `voices` | List the ElevenLabs voices a narration can be read in, or check that one still exists. | a key and a network, but no money |
| `voice_design` | Design a new ElevenLabs voice from a description, for when no voice in either list is the one the video needs. | money, at a provider |
| `rebrief` | Change what a generated asset is to be made from, and mark it stale in the same write. | nothing |
| `generate` | Realise the sketched briefs — the one tool here that costs money. | money, at a provider |
| `render` | Render the timeline to a video file. | ffmpeg, and real time |
| `still` | Look at the edit. | ffmpeg, and seconds |
| `look` | Look at the footage itself, not the edit. | ffmpeg |
| `hear` | See what a sound file looks like: its waveform, drawn as one picture, with the level and the length written on it. | ffmpeg |
<!-- END TOOLS -->

**A project is a directory, and `project_new` is what makes one.**
`project.json` plus `assets/`, `generated/`, `recipes/` and `cache/` — the same
thing `scorsese new` lays out, so an assistant pointed at a machine with no
project on it can start rather than ask for a terminal. The name defaults to the
directory's own and the grid to 30 fps, which makes the usual call one argument.

```
project_new  { "project": "teaser.scor" }
             → "Created project \"teaser\" at 30 fps in teaser.scor"
```

It refuses a directory that already holds anything, and writes nothing at all
when it refuses: half a project laid over what was already there is worse than
an error, and there is no way afterwards to tell which half is whose.

**The edit is the document.** `project_read` and `project_write` are the pair
that makes everything else possible: the whole cut is one JSON file, so any
change at all is read it, change it, write it back. The format is
[`project-format.md`](project-format.md).

`project_write` **validates before writing**. A document that would not load is
refused with every problem listed and the file on disk is left exactly as it
was — so a half-formed edit cannot destroy a working one.

**The loop carries a fingerprint.** `project_read` answers in two blocks: the
document, exactly as it is on disk, and then a line reporting a `fingerprint`
for it. `project_write` takes that fingerprint alongside the document, and it
is not optional — it is what says *which* version of the file this edit is a
change to.

```
project_read   { "project": "teaser.scor" }
               → the document
               → "fingerprint: 9f2c…"
project_write  { "project": "teaser.scor", "document": "…", "fingerprint": "9f2c…" }
```

If something else replaced the document between the read and the write, the
write is **refused and nothing is written**; the message says so, and says to
read the project again. Redo the change on what is there now and write it back
— a cheap round trip, and the correct outcome. Every other tool reads the
document for itself inside the one call, so this concerns the read/write pair
alone.

Without it, two callers editing one project both succeed and the later one
silently takes the earlier one's work with it. That needs no misbehaviour at
all to happen: `generate` holds a document for the minutes a provider takes, so
a caller that read before any of that landed and wrote after it would erase a
shot somebody has already paid for.

**It is not a lock and it is not coordination.** Nothing is held between calls,
and nothing here schedules callers or partitions work. Keeping to disjoint
scopes — one caller per track, and nobody running `scale_pacing` while another
is mid-edit — is still the convention, and it works because clips carry
absolute `start` frames, so genuinely independent work genuinely is
independent. The fingerprint only makes it impossible to break that convention
quietly.

**Call `project_probe` after adding assets by writing the document.** Import
measures what it brings in; an asset that arrived by being written into
`project.json` carries a path and nothing recorded about the file behind it,
and every feature that needs a measured fact about it — the source's own
length, which is the ceiling on a right trim; whether its picture carries
transparency, which is what keeps a scaled logo's edges from going dark — has
no choice but to skip it. This asks ffprobe about each such asset and writes
down what it says. Safe to call after every edit: one
already probed is left alone unless `all` is set.

## Getting media into the project

`import` is how media that is not already in the project gets in, and it is the
only way over the protocol: writing an asset into `project.json` names a path
the project already has to contain.

```
import  { "project": "teaser.scor", "path": "~/Desktop/footage" }
```

The file is **copied** into `assets/` and the document records the relative
path it landed at. The path in the call is used once to find the media and is
never written down — which is the whole reason a project survives `scp -r`.
`kind` overrides what the extension says, exactly as on the command line.

**A single file whose id is already taken is suffixed, and the reply says so.**
An id comes from the file's name, so two *different* shots both called
`intro.mp4` ask for the same one; the second lands as `intro-2`, and the reply
names the id it asked for beside the id it got. Refusing instead would fail the
common case over a name, and saying nothing is how somebody writes `intro` on a
clip and gets the wrong shot. **Take the id from the reply, not from the file
name.** Bytes already in the pool are a different thing and not a collision —
see below.

**`path` may be a directory, and a directory imports its contents, never
itself.** A folder has no duration, no pixels and no samples, so no clip could
ever point at one; assets are the things a clip references and the compositor
draws or hears, and the assets table is that list rather than a file browser.

What a folder import does is fixed so that it can be relied on:

- the media **directly inside it**, one asset each, and **no recursion** —
  walking a tree invents structure nobody asked for;
- **sorted by file name**, so the same folder imports to the same ids in the
  same order every time;
- files that are not media — a font, a licence, a `.DS_Store` — are **skipped
  and named in the reply**, because silently ignoring those and silently
  ignoring a mistyped video look identical from the outside;
- an id an asset already answers to is **refused, changing nothing at all**.
  Everything that can reject a file is found before the first byte is copied,
  so a refusal leaves the project exactly as it was rather than half a folder
  in `assets/`.
  Media whose bytes are already in the pool is *not* a collision: it comes back
  as the asset that holds them, so an import loop stays safe to re-run.

The reply names what came in, what each was measured to be, and what was
passed over — so nothing needs a `project_probe` after it.

## Finding the symbol you meant: `icons`

An `icon` asset is a name and nothing else — `clapperboard`, `triangle-alert`,
`volume-2` — and that name is the one thing about it nobody can work out from
the document. This build ships the whole Lucide set, **seventeen hundred
symbols**, which is far too many to list at a client: a word is how you reach
them.

```
icons  { "project": "teaser.scor", "query": "film" }
       → "15 icons match `film`: film, cctv, cctv-off, clapperboard, file-play,
          file-video-camera, monitor-pause, monitor-play, …"
```

**What comes back is the string to write as an `icon` asset's `name`** — every
one of them, verbatim. There is no second step and nothing to translate.

**It searches what an icon is *about*, not only what it is called.** The query
is matched against each icon's name and then against the words upstream files it
under, which is the half that earns the tool: `clapperboard` answers to *movie,
film, video, camera, cinema, cut, action, television, tv, show, entertainment*,
and none of those is in its name. A search over names alone would miss it for
every word anybody would actually type.

**A plain substring, case-insensitive, never fuzzy.** No scoring function, so
there is nothing to predict and nothing to be surprised by: a fragment matches
(`clap` finds the clapperboard), and a word that finds nothing means the
catalogue has nothing filed under it. Ordering is the one thing that is not
alphabetical — an exact name first, then names containing the word, then
everything a tag matched, then anything only a retired name matched — because a
caller confirming a name it already has should not read past nine others to do
so.

**It also matches the name an icon used to have, and says when it did.**
Upstream renames things, and a former name is the likeliest wrong guess there
is: `unlock` is what the rest of the world calls what Lucide now files as
`lock-open`, and `file-json` and `text-select` are the same story.

```
icons  { "project": "teaser.scor", "query": "unlock" }
       → "1 icon matches `unlock`: lock-open (formerly unlock). A name in
          brackets is one upstream retired — write the name before it."
```

A retired name sorts **after** every current one, because it is the weakest
evidence in the record and somebody who typed a current name must not be pushed
down the list by one. It is **marked** because an unexplained hit on a word that
is nowhere in the icon reads exactly like the fuzziness this search refuses. And
it is never writable: `"name": "unlock"` is still refused with the near-match
message, and the name outside the brackets is the only one the catalogue answers
to.

**The answer is capped at forty and says both numbers.** A one-letter query
matches most of the set; the reply states how many matched as well as how many
are shown, and a capped one asks for a narrower word. There is no page two:
narrowing is the way to the rest. A silent truncation would read as *that is all
there is*, which for this tool is a wrong answer rather than a short one.

**Nothing matching is an answer, not an error.** The reply says so, and says
what was searched, so the next move is another word rather than a bug report.

It costs nothing and needs nothing — no bake, no ffmpeg, no network — the same
standing `synth_survey` has, and for the same reason: the catalogue is compiled
into the binary. That also means the set is **the same for every project**; the
`project` argument is the uniform shape every tool here has rather than a
filter, and no project carries symbols of its own. `scorsese icons <query>` is
the same lookup on the command line, over the same function.

A name that is *nearly* right is a different question and has a different
answer: `project_check` refuses an unknown icon with the closest names beside it.
Use this when you do not know the name, and read that when you thought you did.

## Why the edit was made this way

A project carries its own reasoning, and an assistant is the thing best placed
to record it — while it edits, unprompted, because it is the one that knows why.

There are two mechanisms and they are not interchangeable. A **note** is one
sentence about one element, and it lives in `project.json` on the asset, track
or clip it is about, so `project_read` and `project_write` already carry it —
there is no separate note tool and there should not be one. A **script** is the
document the whole edit is cut from, and it is a file beside `project.json`
rather than a field inside it, so it needs `script_read` and `script_write`.

**Read the script before touching the edit.** It is where the reasons live that
no timeline can show — what the film has to be, and often what it must never
claim on camera. A cold start without it is a guess. `project_describe` prints
the script's path and every note ahead of the cut, so one call says whether
there is anything to read.

`script_write` replaces the file whole, and points the document at it if the
project had no script yet. That is the reason `scorsese new` leaves no stub:
starting a script is one call either way, and an empty `script.md` with the
document pointing at it would have every project claiming to carry one.

**Neither ever renders**, in any version of this tool. Text meant to be seen is
a `text` asset.

## Putting a clip on the timeline, and adjusting it afterwards

`place_clip` and `trim_clip` are the pair that assembles a cut, and everything
about them follows from one fact: **a clip is described in seconds and stored in
frames.** "Two seconds into the take, run it to sixteen, start it at
forty-eight" becomes `start: 1152, duration: 336, source_in: 48` on a 24fps
grid, and doing that conversion by hand is the one editing mistake that reaches
the finished video in silence — a window half a second off validates perfectly,
and a J-cut one frame off is only ever caught by watching.

```
place_clip  { "project": "teaser.scor", "asset": "rooftop", "track": "v1",
              "start_seconds": 48.0, "source_in_seconds": 2.0,
              "duration_seconds": 14.0 }
            → "`rooftop` placed on `v1`: starts at 48.00s (frame 1152), runs
               14.00s (336 frames) to 62.00s (frame 1488), opening 2.00s
               (frame 48) into `rooftop`."
```

**The reply says it in both units, always.** Seconds because that is what was
asked for, frames because that is what the document now holds — and a caller
that cannot read the frame back has no way to tell which side of a rounding its
cut landed on.

**`source_in_seconds` is a time in the source, and the framerates are not your
problem.** It is written down as `source_in` in *timeline* frames, so "skip the
first two seconds" means the same thing whether the take was shot at 25fps and
the timeline runs at 24 or the other way round; the conform rule in
[`project-format.md`](project-format.md) is what turns it into a source frame at
render time.

**Leave `duration_seconds` out and the clip runs the rest of the source.** "Put
this shot in" is a whole request on its own and the answer is written down
already — the asset's measured length, less wherever the clip opens. An asset
with no measured length has no rest to take: a title, a still, a colour, a brief
nobody has generated, and a file nobody has probed. There the duration is
required, and the refusal says so rather than guessing a length.

**`trim_clip` sets fields, not edges.** Each argument changes the field of the
same name and nothing else, so a `start_seconds` on its own *moves* the clip, a
`duration_seconds` on its own holds the start and moves the end, and a
`source_in_seconds` on its own shows a later part of the media in the same slot.
An editor's drag handles would make one argument mean different things depending
on which others came with it; this does not.

```
trim_clip  { "project": "teaser.scor", "clip": "vo-open", "start_seconds": 37.7 }
           → "`vo-open` now starts at 37.70s (frame 1131), runs 11.90s (357
              frames) to 49.60s (frame 1488), from the head of `vo-open`."
```

**Both refuse whole, and a refusal writes nothing at all.** A clip landing on
one already on that track, a window reaching past the end of the media, a clip
that would cover no frame, a negative time — each comes back with the reason and
leaves `project.json` byte-for-byte as it was, so a client may keep asking until
it asks for something possible.

Two things they deliberately do not do. **A track is never created**: a track
invented from a mistyped id would take the clip with it, and a clip on a track
nobody meant to have is invisible in every way except the render — so a missing
track is refused, and the refusal names the tracks there are. **A clip never
changes track**: which track a clip sits on decides what is drawn over what,
which is a different edit with a different consequence, and it stays a
`project_write`.

The clip's id is optional. Without one it comes from the asset's, suffixed until
it is free — `rooftop`, then `rooftop-2` — exactly as an imported asset gets its
own, and the reply names what it wrote. **Take the id from the reply**, the same
rule `import` asks for.

## The three operations that write keyframes for you

`dissolve`, `duck_music` and `set_volume` are sugar, and all three keep the same
bargain: what they write is **ordinary keyframes** — the ones you would have
placed by hand — which stay visible, editable and deletable afterwards. None of
them adds a stage to the renderer, which is why a dissolve composes with a move
or a turn for free.

`dissolve` has one behaviour worth knowing before calling it: **it moves the
incoming clip to a track above.** Two clips on one track may not overlap, and a
crossover needs them to, so the shot arriving is pulled back over the outgoing
one and put on a later track — later meaning drawn on top. The reply says which
track and how far it moved, because that is an edit you did not ask for in so
many words.

It refuses, changing nothing at all, when the two clips do not currently meet
at a cut, when either is shorter than the crossover, or when the crossover
would round to no frames. A dissolve with no defined crossover has no shape,
and guessing at one is worse than saying so.

`set_volume` is the fader. A `level` on its own is flat for the whole clip —
`1.0` is the source as recorded, `0.0` is silence, and **muting a clip is a
level of `0.0`** rather than a flag. Add `from_level` and `seconds` and the
level arrives as a fade instead, starting at `at_seconds` into the clip:

```
set_volume  { "project": "teaser.scor", "clip": "m1", "level": 0.4 }
set_volume  { "project": "teaser.scor", "clip": "m1", "level": 0.0,
              "from_level": 0.4, "at_seconds": 18.0, "seconds": 2.0 }
```

**A clip animates volume from one track, so `set_volume` takes that lane over.**
Whatever was animating the clip's volume is replaced — a fade you wrote by hand,
or a dip `duck_music` signed — and the reply names each one it displaced, how
many points it had, and who wrote it. Two tracks on one property would be a
document where the second silently never plays, which is a worse answer than
saying what went. When the dip was among them the reply says to run `duck_music`
again if it is still wanted; `duck_music` itself is unchanged, and still replaces
only what it signed.

It refuses, changing nothing at all, a negative level (volume is a multiplier,
so below zero inverts the phase rather than muting), a fade missing one of its
two halves, and a fade that would finish after the clip ends.

## Pacing: the same clips, spread differently

`scale_pacing` is the operation for **pacing**, which is most of what editing
actually is. A montage cut to a song that turned out to be 8% slower, a run of
titles that all need a beat more air, a sequence that drags in the middle — all
of them are the same clips spread differently, and every other way of doing it
is rewriting every `start` in the document and getting the arithmetic slightly
wrong.

```
scale_pacing  { "project": "teaser.scor", "clips": ["c-title", "c-black"],
                "factor": 0.83, "about_seconds": 8.0 }
```

`about_seconds` is the one instant that does not move. Everything before it
draws in and everything after it pushes out, which is what lets you keep the
moment you care about fixed and let the rest breathe around it.

**Durations scale with the positions, but only where that is free.** For a
title, a still, a colour, or a brief nobody has generated yet, `duration` says
nothing except how long the thing is on screen — so scaling it is exact, and
nothing plays faster. That is the difference between a faster cut and the same
cut with its gaps squeezed out.

For a clip with a real file behind it the same instruction has two entirely
different readings — show less of the source, or play the whole of it faster —
and picking one silently is the failure that *looks* successful either way. So
those clips move, keep their length, and **the reply names them.**

Positions are rounded in one place, which buys a property worth relying on: a
run of cuts that touched exactly still touches exactly, at every factor. It is
only a mixed selection — some clips scaling, some keeping a length their media
owns — that can open a gap or a collision, and a collision is refused.

Refusals change nothing at all: a factor that is not positive, a clip that
would land before the start of the timeline, a clip that would round away to
less than a frame, and any result the document would not load.

## Where things land on the frame: `project_describe` with `at`

`project_describe` says what is on screen. **`at` says where it is.**

```
project_describe  { "project": "teaser.scor", "at": "12s" }
                  → the cut, and then:

where things land at 12.00s (frame 288), on a 1920x1080 frame — as fractions of it
  bg/panel        shape    x 0.150–0.850  y 0.350–0.650  (0.700 × 0.300)
  titles/caption  text     x 0.180–0.820  y 0.402–0.598  (0.640 × 0.196)
  not on screen here: open, rooftop, end-card
```

Every number is a **fraction of the frame**, which is the unit the document is
written in: `transform.position`, a text `size`, a shape's `width`. So a figure
read here can be written straight back into `project.json` — a panel made to sit
behind that caption is `height: 0.196` plus whatever padding you want, rather
than a guess refined by looking at three renders.

**It is the compositor's own rectangle, not a second calculation of it.** The
same matrix that draws the layer places these corners, so the answer cannot
drift from the picture. That is the whole reason this exists: predicting a
wrapped block's height as `lines × size × 1.45` is arithmetic the renderer has
already done exactly, and being close enough is a bug waiting for a font to
change.

**A layer that is turned reports the smallest upright rectangle containing it.**
A rotated block has no upright rectangle of its own, and the box around it is
the answer a caller can use.

**`at` takes a list**, and should, whenever a layout has to hold at more than one
point: `{ "at": ["4s", "12s", "48s"] }`. A caption that fits at 0:12 may not at
0:48, where a longer one has taken its place — which is exactly the mistake
nobody catches until the film is watched.

**Clips with no rectangle are named, with the reason**, because the reasons are
different things to do about it:

- *not on screen here* — it is somewhere else on the timeline. Ask at an instant
  it plays.
- *in the mix, not on the picture* — it is a sound. A narration **sketch** is not
  this: an ungenerated prompt draws a slug card, and a card has a rectangle like
  anything else.
- *no rectangle for …* — it is on screen and could not be measured. An arrow (a
  line between two points has no box of its own), a media file that has gone
  missing, or a source whose pixel size the document never recorded, which
  `project_probe` writes down.

`resolution` sets the frame it is measured against, default `1920x1080`. The
answer is in fractions either way, but the frame's *shape* still decides it: a
`fit` picture letterboxes against that aspect and a title wraps against that
width. Nothing is composited, no ffmpeg runs, and it costs nothing.

**It reports; it never moves anything.** There is no tool that centres, packs or
un-collides a layout, and there will not be one — knowing where a thing is does
not make the editor the one to move it.

**Asking about an instant is not asking about the film.** `at` answers where
things are *here*, and a caption lying across a diagram three minutes later is
found only if somebody thought to ask there too. `project_check` asks at every
instant where the visible set changes, and warns about the pairs of layers drawn
across each other — with the span, because a pair overlapping for two frames
during a dissolve is noise and a pair overlapping for nine seconds is the bug.
The deliberate stacks are filtered out on purpose: a full-frame background, a
label inside its box, anything much smaller than what it sits on. Text is
filtered hardest, because a text layer's rectangle is its wrap box and not its
glyphs — two captions can share a quarter of their boxes with clear air between
the words. It under-reports rather than crying wolf, so a quiet `project_check`
is not a proof that every frame is right — it is one fewer reason to go and look
at one.

## Looking at the frames

Two tools answer with something other than words, and the split between them is
worth getting right: **`still` shows the edit, `look` shows the footage.** One
composites your timeline at an instant; the other decodes a video file. Neither
replaces the other — a cut that composites perfectly can still be assembled out
of shots nobody has looked at.

### `still` — what the edit looks like

Everything else here *describes*: what the document says,
what the cut contains, what is wrong with it. An assistant that writes a title
and reads back "CHAPTER ONE, centred, 0.14 of the frame" still has no idea
whether it is readable, whether the shot under it leaves the words legible, or
whether it is on screen at all.

```
still  { "project": "teaser.scor", "at": "9.1s" }
       → "frame 273 (9.10s) of Teaser at 1280x720", and the picture
```

The reply carries two content blocks: the sentence, and the frame as a PNG
image. A client that can see images sees it. `at` takes either unit — `9.1s` or
the timeline frame `273` — and a bare decimal is refused rather than guessed at.

**`at` also takes a list**, and that is how a whole cut gets checked:

```
still  { "project": "teaser.scor", "at": ["0s", "9.1s", "400", "22.5s"] }
       → four sentences and four pictures, in that order
```

One sentence and one picture per instant, **in the order asked** — never sorted,
never deduplicated, so the reply lines up with the question. *"Does every
section look right?"* is one question, and answering it one frame at a time
turns it into a round trip per section. A picture is the most expensive reply
this server sends, so what looking costs is what decides how often anything gets
verified, and an assistant that checks one section of six and reports on all six
is wrong without erroring.

**It is the frame a render would deliver**, because it is the render pipeline
with the encoder taken out: the same plan, the same decoders, the same
compositor. Nothing is encoded and no video file is produced, so it costs
seconds rather than a render. Sketch and stale generated assets appear as slug
cards, exactly as they would in a preview cut.

The default raster is 1280x720 rather than a delivery size, because layout is a
fraction of the frame — the same picture with a fraction of the wire cost. Pass
`resolution` for delivery size. Pass `out` to keep the PNG on disk as well;
without it, nothing is left behind.

**`out` is for one instant, and a list with `out` is refused.** A path names a
file and several frames do not fit in one. Reading it as a directory instead
would make the tool's secondary use a second, weaker `render --stills` — which
already writes a numbered set of PNGs — so the refusal names the path and the
count and points there.

### `look` — what is actually in the footage

Every other tool answers about a video file in words: how long it is, how big,
what codec, what it is called. None of that is what a shot *is*. An assistant
handed twenty clips and asked for a cut is arranging file names and hoping.

```
look  { "project": "teaser.scor", "file": "assets/03-rooftop.mp4" }
      → "5 frames of …/03-rooftop.mp4 (48.0s long), at 0:00, 0:05, 0:10, 0:15,
         0:20 — 25.0s to 48.0s not seen yet; call again with from: 25",
        and the sheet
```

**A path, not an asset id** — relative to the project like every other path
here, or absolute for footage that is somewhere else entirely. The file does not
have to be in the assets table, so material gets looked at *before* the decision
to import it.

**One picture, not five.** Five separate image blocks cost five times as much
and say less: what tells you what a shot does is the change *between* frames,
and a change is only visible when the frames are side by side. At most five
frames, ever, and the cap is enforced in the library rather than trusted from
the caller.

**A long file is walked.** Without `to`, the frames are five seconds apart and
the reply says where the next call starts — 0–20s, then 25–45s. That is the
intended way to cover a file, not a shortfall: a tool that squeezed an hour into
five frames would show five unrelated pictures.

```
look  { "project": "teaser.scor", "file": "assets/03-rooftop.mp4",
        "from": 12, "to": 16 }
      → four seconds, in five frames spread across it
```

**With `to` it is a different question**, so it behaves differently: the caller
has named a stretch they want to see, and the frames spread evenly across it,
first on `from` and last on `to`. Covering a file and studying a moment are not
the same request.

Nothing is left on disk — a sheet is something to look at, not part of the
project. `scorsese look` is the command that keeps one.

**It closes the loop on generated video.** Veo returns a clip that may or may
not be what the brief asked for, and without this nobody can tell: the asset
flips to `generated` and the next tool call proceeds having got a sunset where
it asked for a sunrise. Generate, look, decide whether to re-prompt — and
`generate` below is the first half of that loop.

### `grid` — reading a coordinate off the picture

Both tools answer *what is there*, and neither, on its own, answers *where*.
Every number an assistant writes into a document is a coordinate: a `crop`
rectangle, a title's `transform.position.y`. Found without a ruler, each one
costs a round of guess, render, look, guess again — an ffmpeg run and a whole
image through the client, to learn a number a ruler shows immediately.

`grid: true` rules the picture: a line every `0.1`, heavier at `0.5`, labelled
along the top and left edges, origin at the top-left corner.

```
still  { "project": "teaser.scor", "at": "9.1s", "grid": true }
       → "frame 273 (9.10s) of Teaser at 1280x720, ruled 0.0 to 1.0",
         and the picture with the ruler on it
```

**Fractions, never pixels.** Pixels change with the raster and with `still`'s
`resolution`; fractions are what the document takes, so what is read off the
picture is what gets typed into it. The two tools rule different things, and
that difference is the useful part:

| | what `0.0`–`1.0` spans | the unit of |
| --- | --- | --- |
| `still` | the render raster | `transform.position.x`, `transform.position.y` |
| `look` | the **source's** own width and height | a clip's `crop` |

`look` rules **each frame of the sheet** rather than the sheet, because a cell
is one whole source frame — so a rectangle read off a cell is a rectangle
`crop` will take. The tiling is never a coordinate.

One thing the ruler does not do for you: `transform.position` is an **offset**
from where a layer already rests, not the place it lands. On a `still` the grid
gives you where the layer is and where you want it; the number to write is the
distance between them.

**Off by default, and drawn on the frame itself** — including a PNG kept with
`out`, and the one `scorsese still --out` writes. It is what you pass while
measuring; leave it off for a picture you mean to keep, because there is no
taking the lines off afterwards.

## Changing a brief: `rebrief`

A generated asset is a **brief** and a **state**, and the state is the half
that gets forgotten. `sketch → queued → generated`, and back to `stale` the
moment the brief is edited — so an asset still marked `generated` after its
prompt changed is a lie the rest of the system believes: the cut renders the
previous take rather than a slug card, and nothing anywhere says the take is
not what the project asks for.

`rebrief` writes both in one call.

```
rebrief  { "project": "teaser.scor", "asset": "hero",
           "prompt": "a lone figure on a wet platform, 35mm" }
         → "`hero`: prompt changed, generated → **stale**. The file on disk is
            the previous brief's, so the cut shows a slug card until the next
            generate redoes it."
```

**The reply states the resulting state, always** — that is the whole point of
having a tool rather than two edits. It is also why the answer is not always
*stale*: an asset that has **never** been generated stays `sketch`, because
there is nothing stale about a brief nobody has realised. Both are states
`generate` acts on, so nothing is lost by being accurate about which one it is.

**Two kinds of brief, and the tool keeps them two.** `prompt` is for
`generated_video` and `generated_audio` — a sentence a provider is paid to
read. `recipe` is for `synth_audio`, and it repoints the asset at a *different*
recipe file; changing what is **inside** a recipe is `synth_write` or
`synth_set`, and needs no `rebrief` at all, because a bake is named for the
hash of its recipe. Passing the brief an asset's kind does not take is refused
rather than resolved: the call has misunderstood the asset, and guessing which
half was meant would write the half nobody checked.

**It refuses atomically.** The whole document is validated before anything
reaches disk, so a refusal — an unknown asset, the wrong kind of brief, a
recipe path that is not there — leaves `project.json` exactly as it was.

**A queued asset is refused until it is collected.** A ticket is money already
committed, and what comes back is named after the brief that was sent. Editing
the brief while a shot is in flight would file the arriving video under a name
it is not, so the way through is `generate` with `collect` first — it is paid
for either way.

**An unchanged brief writes nothing** and leaves the state alone. Marking an
asset stale for an edit that changed nothing would cost a slug card in every
preview until somebody regenerated it.

**Generating is not triggered.** `rebrief` costs nothing and spends nothing;
`generate` is still what realises the result, and its cost gate stays where it
is. The rest of a brief — a shot's `video` block, a line's `speech` block — is
still a `project_write`, and those fields are hashed into the brief too, so
changing a voice deserves the same `"state": "stale"` by hand.

## Generating the shots and lines that do not exist yet

`generate` is **the one tool here that costs money**, and everything about its
shape follows from that. It drives both providers: shots go to Veo, narration
to ElevenLabs.

```
generate  { "project": "teaser.scor", "dry_run": true }
          → "hero: $0.96 — 8s of fast at 1080p
             wide: $0.20 — 4s of lite at 720p
             vo-open: $0.01 — 58 characters in fast
             About $1.17 for the whole run — our arithmetic over published
             rates, never a bill."
```

**The two are quoted on separate lines and never averaged.** Eight seconds of
video is ninety-six cents and a sentence of narration is one — sixty to a
hundred times less. A per-item average across them would describe nothing that
exists.

**Quote before you spend.** `dry_run` needs no key and sends nothing. Every
figure it prints comes from the rate table in
[`prices.md`](prices.md) — and *no provider reports what a
generation actually cost*, so these are calculations, not receipts. That is why
the field on the asset is called `estimated_cost_cents`.

**A brief already generated is never sent again.** A generation lands at
`generated/<asset-id>-<hash of the brief>.mp4` — or `.mp3` for a line — and the
hash covers every field of the request: for a shot, that includes *the bytes of
every still it names*; for a line, the voice, the model, the language and the
seed as well as the words. So calling this twice by mistake — or after a dropped
connection, which is the likelier case — costs nothing the second time, swapping
one `face.png` for a different picture of the same name *does* count as a new
brief, and so does changing which voice reads a line.

**A key is asked for only by the half that has work.** A project of nothing but
narration never needs a Veo key, and one of nothing but shots never needs an
ElevenLabs key.

**It waits, then detaches — for video only.** A shot takes minutes. This waits
five of them by default (`wait_seconds`) and then returns, which loses nothing:
a submitted shot's ticket is written into `project.json` before anything else
can go wrong. **Narration is never in flight**: a line comes back on the same
call, so there is no ticket, nothing to poll, and nothing `collect` could pick
up.

```
generate  { "project": "teaser.scor", "collect": true }
          → "hero: generated — generated/hero-4f2a….mp4 (2855524 bytes)"
```

**`collect` can never spend anything.** It polls what is already in flight and
submits nothing, which is what makes it safe to call on opening a project — a
sweep that could start a generation is one that eventually starts twenty. It
works from another machine too, because a `*.scor` directory carries the
tickets with it.

**One shot failing leaves the rest generated.** A refused brief is reported
against its own asset and put back to `sketch` so the prompt can be edited; the
other nineteen shots are still generated and still say so. Only things that make
the whole run impossible — no key, over the budget ceiling, a still that will
not read — stop it.

**A line with no voice chosen yet is reported and skipped, not failed.** There
is no default voice and there cannot be one: every one of ElevenLabs' Default
voices expires on 2026-12-31. So a narration nobody has chosen a voice for is
the ordinary state of a cut being written, and it comes back as *not yet* while
the rest of the run goes ahead. Stopping there would mean re-speaking nineteen
paid-for lines in order to add the twentieth.

**What is generated is measured once it lands.** A brief is not a measurement:
how long a line takes depends on the words, and whether Veo put an engine note
under a shot is not in the prompt either — it routinely does. So `generate`
probes what it just wrote, because the mix reads `duration_seconds` and
`audio_channels`, and a generation nobody measured is a line the mix skips and
a shot that plays silent.

**A queued shot has two days.** Past that the provider deletes the finished
video and the money is gone, so a wait that old is reported as its own outcome
rather than as a longer wait: the two call for opposite things.

## Choosing a voice

`voices` answers the question a narration cannot be generated without: **what
goes in `voice_id`**. It costs no credits, and the answer is cached inside the
project, so browsing it is free in both senses.

```
voices  { "project": "teaser.scor" }
        → "21m00Tcm4TlvDq8ikWAM  Rachel  (female, young, american, narration)
           …
           19 built-in voices, read from ElevenLabs just now."
```

**There is no default voice, and there never will be.** Every ElevenLabs
default voice **expires on 2026-12-31**, and the default set is being replaced
before then. A voice id written into scorsese would therefore not be a shortcut
with a small cost — it would be a guaranteed outage with a date already on it.
So the list is resolved at runtime, and a voice is *picked* rather than
remembered.

**Two lists, and the second is where a language lives.** Without `library` you
get the small built-in set an account already has. With it you get the Voice
Library — thousands of voices other people published — narrowed by `language`,
`locale`, `gender`, `age` and `accent`. `language` is the filter worth reaching
for first: it is what surfaces people who actually *speak* pt-BR rather than an
English voice reading it.

```
voices  { "project": "teaser.scor", "library": true,
          "language": "pt", "gender": "female", "page_size": 50 }
```

**A search that was cut short says so.** `page_size` caps the reply at a
hundred; it does not cap what matched, and a search for Portuguese matches
thousands. So the listing closes with how many there were — a truncated search
and a complete one that printed identically would have somebody concluding
their filter found everything and stopping there.

```
voices  { "project": "teaser.scor", "library": true, "language": "pt" }
        → "…
           100 the Voice Library voices, read from ElevenLabs just now.
           That is the first 100 of 4,274 matching voices — narrow with
           language, accent, age or gender to search a smaller set."
```

There is no cursor and no second page to ask for: narrowing is the way to a
different hundred. The count is the vendor's own and is never estimated — where
it sends none, the listing says there are more without saying how many.

The Voice Library is **not available through the API on a free plan**. When it
is refused, the reply says exactly that and points at the built-in voices,
which generate the same way. No subscription tier is detected to produce that
message — it is only ever what the vendor answered when asked.

**Ask about one id with `check`, and believe the answer.** A voice that can no
longer be generated with is reported as such, with the reason: it has gone, or
this account may not use it. Nothing is ever silently substituted — the
alternative is narration produced, billed and read aloud in a voice nobody
chose, which would sound perfectly fine and be wrong.

```
voices  { "project": "teaser.scor", "check": "21m00Tcm4TlvDq8ikWAM" }
        → "21m00Tcm4TlvDq8ikWAM cannot be used at ElevenLabs: nothing there
           answers to it. …  Run `scorsese voices` and pick another — nothing
           has been changed."
```

**A listing says how old it is, every time.** The lists live under the
project's `cache/`, which is rebuildable and gitignored, so a second call
touches no network and an offline call still answers from what it has. An entry
older than a week re-reads itself, and `refresh` forces it sooner. The
provenance line is never omitted, because *these are the voices* and *these
were the voices last time anyone could ask* are different claims — and only
`check` is guaranteed to have asked just now.

**A `401` from this vendor is two different problems.** A key that is absent or
wrong is fixed in `.env` or the settings file; a key that is perfectly valid and
was never granted the `voices_read` permission is fixed in the ElevenLabs
dashboard. The refusal body names which it is, so the two get different advice —
sending somebody to check a credential that is already correct is the most
expensive kind of wrong error message.

## Designing the voice, when neither list has it

`voice_design` is the escape hatch for the case `voices` cannot answer: the
voice a person had in mind before they heard anything, which neither the
built-in set nor the Voice Library happens to contain. It is especially the
pt-BR case — the Library's coverage there is thin, and a free account cannot
reach it at all.

It is **the second tool here that spends money**, and it spends it differently.

```
voice_design  { "project": "teaser.scor",
                "prompt": "a Brazilian woman in her forties, unhurried, warm,
                           the voice of someone telling you something true",
                "text": "<a hundred characters or more of the line the video
                         actually needs>",
                "seed": 42 }
              → "1. hCTB9k2W…
                    generated/voice-design/9f3c…/1-hCTB9k2W….mp3
                 2. …
                 $0.02 for the design — 118 characters of preview text at 10¢
                 per 1000, charged once for all three candidates."
```

**One call, three candidates, one charge.** The billing is the thing to get
right here, because the obvious guess is wrong: three samples come back and the
*preview text* is billed once. `dry_run` quotes it without a key and without
sending anything, exactly as `generate`'s does.

**Voice design spend is its own line item.** It counts against the same
spending ceiling — it is real money at the same vendor — but it is never added
to what the shots cost. A generation total that moved while nobody was
generating would be a counter nobody could trust, and *"voice design"* beside
the number is the whole fix.

**The samples are files, not sound in a reply.** Three MP3s land in
`generated/`, named for the hash of the brief, and the reply says where. So
asking for the same design twice — after a dropped connection, most likely —
costs nothing the second time, the same guarantee a generated shot has. Playing
them is a window's job; a client that cannot hear gets the paths, and a person
gets the audition.

**Then keep one, which costs nothing more.**

```
voice_design  { "project": "teaser.scor", "keep": "hCTB9k2W…",
                "name": "Narrator (pt-BR)" }
```

That call turns a candidate into a real `voice_id`, usable exactly like one out
of a listing. The two that were passed over need no cleanup — they were never
persisted as voices — and they are reported to the vendor as auditioned and not
chosen, which is free and is what the endpoint asks for.

**What it creates is not inside the project, and that is worth knowing.** The
voice lives in the user's ElevenLabs account. The id travels with an `scp -r`
and the voice does not: open the project on another machine, under another
account, or after somebody tidied their voice list, and the id names nothing.

So the **description and the seed** are written into `designed-voices.json`
beside `project.json`, and a voice that is lost can be asked for again from
what the project already holds. It is a file rather than a field in
`project.json` because a record of things created in somebody's account
elsewhere is not what that document describes. `list` reads it back.

```
voice_design  { "project": "teaser.scor", "list": true }
```

**Voice cloning is not here, in any form.** No user-supplied audio, no
likeness, no consent surface. That is a decision rather than an omission, and
it is not a smaller version of this tool.

## Rendering part of the timeline

`render` takes `range`, the same `start:end` frame syntax as `scorsese render
--range`: `30:120` covers frames 30 up to but not including 120, `30:` runs to
the end, `:120` from the start. Without it the whole timeline is rendered, and
that is still the right call for a delivery.

```
render  { "project": "trilhas.scor", "out": "cue-3.mp4", "range": "450:750" }
```

It is there because checking one ten-second cue in a sixty-second cut should
cost ten seconds of encoding rather than sixty. The parser is the CLI's own, so
a range either client refuses is refused by both, with the same words.

## Making sound

`synth_read` and `synth_write` are the pair that has no command-line
counterpart, and that is deliberate. Over the CLI you would edit a recipe with
an editor; an assistant that has to round-trip through the filesystem to change
a note is doing bookkeeping instead of composing.

The loop is **write, bake, listen, adjust**, and every turn of it is free:

```
synth_new    → a starter recipe that makes a sound as written
synth_read   → what it says now
synth_write  → change it
synth_bake   → hear it
```

Nothing has to mark an asset stale. A bake is named for the hash of its recipe,
so changing the recipe changes which file the asset wants, and the next
`synth_bake` redoes it. Re-baking an unchanged recipe renders nothing.

What to write in a recipe is [`recipes.md`](recipes.md).

### Tuning, which is not writing

**Reach for `synth_set` when the music is already right and a number is not.**
Writing a score is a whole document; *adjusting* one is a float at a time, many
times, with a bake and a measurement between each turn — and `synth_write` prices
that at the entire piece, so moving a track's `gain` from `0.71` to `0.64`
re-sends every note in it to say nothing about any of them.

```
synth_set  { "project": "trilhas.scor", "recipe": "recipes/05.json",
             "field": "gain", "track": "arp", "value": 0.64 }
```

It sets **one** number, named the way the recipe names it: a song's `bpm`,
`seed` or `swing`, a patch's `duration`, `velocity` or `seed`, and a track's
`gain` — the track by **its name**, the one the song's notes already use. Those
are the values re-tuned after listening, and none of them rewrites a note.
Anything else, a note or an arrangement entry included, is a
`synth_write`: notes and entries have no names, and addressing them by position
would mean something different the moment one is inserted above them.

It refuses, **changing nothing**, on a field that recipe's shape does not have,
a track that is not in the song, or a value the field cannot hold — and the
refusal says what the recipe does take, so learning that costs no extra call.
What it writes is an ordinary recipe in the format's own canonical form, still
readable and still editable by hand, and its asset goes stale by exactly the
arithmetic a full write does.

## Seeing a sound: `hear`

`look` samples a video into a contact sheet so a client can see what a shot is.
`hear` is the same trick one medium over: a sound file becomes **one picture of
its waveform**, amplitude over time, with the level and the length written along
the top and a time axis under it.

```
hear  { "project": "teaser.scor", "file": "generated/vo-01.mp3" }
```

**Why it exists is narrower than it looks, and worth stating.** Everything
audible in scorsese used to be either imported — somebody heard it before
importing it — or synthesised, which is deterministic and reproducible from the
document. Generated narration is neither: it costs money, it cannot be
reproduced from the project alone, and it is the first output **nobody has ever
heard**. Three lines were once verified by file size, byte hash and a probed
duration of 3.67 seconds, every one of which 3.67 seconds of silence passes
identically.

**What the picture answers**: is it silent, is it clipped, does it start late or
end early, is it the length it should be, where are the pauses. Every one of
those is a shape on the line, and a client that cannot see the image gets all of
them in the reply's words as well.

**What it does not answer**: the wrong voice, the wrong language, a
mispronunciation, a flat read. Those need hearing, and nothing here pretends
otherwise — a picture that claimed to answer them would be worse than no picture
at all. Carrying the audio itself back as an MCP audio block would answer them
and is a separate question, because it depends on the client handling audio and
this server may not assume which client is on the other end.

`hear` and `audio_level` are not two ways to do one thing. A shape says *where*;
a number says *how much*. "The pause after the second line is a beat too long"
is only visible, and "the sub is 96% below 250 Hz" is only measurable.

## Listening, for a client that cannot hear

`synth_bake` reports how what it just made came out, and `audio_level` reports
the same about any finished file — mean, peak and crest over the whole thing,
the same again per section, and the share of the energy that is low, mid and
high. Give `audio_level` an `against` and the two files are compared field by
field.

**That is the loop closing.** For every other part of this project a client can
check its own work; audio is the one place it currently terminates at a human.
Most of that gap is not real and is not papered over here — for a `synth_audio`
asset the client *wrote the document*, so which instruments play and where the
sections are is already in the recipe, exactly and for free. The gap that is
real is everything that only exists **after** the render: level, spectral
balance, and whether section C is actually bigger than section A or merely has
more notes in it.

The comparison is the part with teeth, and the reason is that an absolute
number is hard to judge and a difference is not. Rewrite a score, re-bake,
compare against the previous file, and the question "did the change land?" has
an answer without anyone being asked to listen a second time.

`synth_bake` adds one thing `audio_level` cannot: **a row per track of a song**,
under the section rows, carrying the same figures post-gain. "The mix is muddy"
is a diagnosis with no address when five instruments are playing; "the sub is
96% below 250 Hz" is one fader. The rows are always in the reply — a report
that has to be asked for is one an unattended client never sees — and only a
song of more than one track has them. `audio_level` measures a finished file,
which no longer has tracks in it, so it reports the sum alone. What the rows
mean, in full, is in [`recipes.md`](recipes.md#which-layer-is-taking-up-the-room).

It is a **signal and never a gate** — there is no correct loudness — and it is
not a critic. It finds defects: too quiet, clipping, muddy, a section flat
where the arrangement said climax. It does not find taste, and a metric treated
as an ear produces music that optimises the number and gets worse.

### The one question that is about a set

Everything above asks *how did this one come out*. `synth_survey` asks *what
are all of these*, and it is the only tool that reads more than one recipe at
a time. Six cues can each be baked, levelled and corrected, all pass, and still
be one instrument playing in all six — which is the first thing a listener
notices and the last thing any per-bake number can see.

It costs nothing and needs nothing: no bake, no ffmpeg, no network. Everything
it reports is already written down in `recipes/`, so this is parsing documents
that were going to be parsed anyway. Per song it gives the tempo, the register
and the pitch classes, then a row per track; under them, the same facts counted
across the project.

A **track row has two halves**, and the second is the one that earns the call.
What the instrument *is* — source kind, gain, filter cutoff — does not predict
what anyone hears: three cues written on `karplus`, `fm2` and `osc_stack` can be
one plucked guitar to a listener, and changing the source kind does not move
that complaint. So the row also says what the track *does*: the share of the
arrangement it sounds over, its envelope's sustain, its notes per second, and
the median pitch it sits at. What each column means, and why `sustain` is the
envelope's rather than the source's, is in
[`recipes.md`](recipes.md#what-the-whole-set-is-made-of).

The rollup's **`loudest` is `gain × duty`**, not the highest written gain.
Percussion is written loud precisely because it is short, so ranking on gain
alone crowns a hi-hat — and the line that results reports a *more varied* set
than exists, which is the one failure this report cannot afford. It stays a
proxy: a plucked harp well down in the mix can still be the instrument you hear.

**It counts and stops.** There is no score, no grade, no recommendation and no
diversity number — a set of six variations on one instrument is a legitimate
thing to write on purpose, and a metric of variety is precisely the one that
would get optimised. A project of fewer than two songs has no set to report on.

## Stateless, on purpose

There is no server-side "open project". Every call names the project it works
on, so a client may crash, reconnect, or run two conversations against one
project without anything getting out of step.

The fingerprint the read/write pair passes is not an exception to that. It
travels with the call rather than being remembered here, and it is derived from
the bytes on disk — so a client that disappears mid-edit leaves nothing behind
to time out, and a fingerprint from an hour ago means exactly what one from a
second ago does.

## Every tool describes itself, and that is a gate

A tool's description is the entire interface a client has to it. An undescribed
tool is a capability that exists and cannot be found — nothing fails, the
assistant simply never calls it, and nobody ever learns why.

So `crates/mcp/tests/described.rs` walks the registry and fails on a tool or an
argument that says nothing useful about itself. The same gate already covers
`scorsese --help` and the animatable-property table in `project-format.md`.

**A tool missing from this page was the same failure by another door**, and
nothing caught it: the row's absence was invisible to every test in the repo, so
an agent reading the page cold learned about every capability except the newest
one. `crates/mcp/tests/table.rs` closes that by holding the page to the
registry — which is why a description's **first sentence** now has a second job.
It is the cell, so it has to say what the tool is for on its own, without the
rest of the paragraph standing behind it.

## What it is not

No editing logic of its own. Every tool is a thin wrapper over the same library
the CLI calls, and a tool that needs code the CLI cannot reach means that code
is in the wrong crate.
