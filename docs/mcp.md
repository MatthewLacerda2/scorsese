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

```
cargo build -p scorsese-mcp
```

Then in the client's MCP configuration, a server whose command is the built
binary — `target/debug/scorsese-mcp`, or wherever a release build put it. For
Claude Code:

```
claude mcp add scorsese -- /path/to/scorsese-mcp
```

## The tools

Every tool takes `project`: the path of the `*.scor` directory to work on — or,
for `project_new` alone, the one to create.

| Tool | What it does | Costs |
| --- | --- | --- |
| `project_new` | lay out a new `*.scor` directory to edit in | nothing |
| `project_read` | the `project.json` exactly as it is on disk | nothing |
| `project_write` | replace it, **validated first** | nothing |
| `project_describe` | what is on screen when, and what is audible under it | nothing |
| `project_check` | every problem with the document and its media, at once | nothing |
| `project_assets` | the media pool and the state of everything in it | nothing |
| `import` | copy media in from anywhere on disk — a file, or a folder's worth | ffprobe |
| `project_probe` | measure the assets nobody has probed, and record it | ffprobe |
| `script_read` | the document this edit is being cut from | nothing |
| `script_write` | replace it, starting one if there is none | nothing |
| `dissolve` | cross one shot into the next, as opacity keyframes | nothing |
| `duck_music` | lower a music track under narration, as volume keyframes | nothing |
| `scale_pacing` | spread a run of clips out, or close it up, about one instant | nothing |
| `synth_new` | start a recipe, and the asset that points at it | nothing |
| `synth_read` | a recipe file as it is on disk | nothing |
| `synth_write` | replace one, **parsed first** | nothing |
| `synth_set` | change one number in one, by name | nothing |
| `synth_check` | parse a recipe without rendering it | nothing |
| `synth_bake` | render the recipes not already baked, and say how each came out — whole, per section, per track | nothing |
| `synth_survey` | what every song recipe is made of, counted across all of them | nothing |
| `audio_level` | how a finished sound file came out, and how it differs from another | ffmpeg |
| `still` | **look at** the edit, returned as pictures | ffmpeg, and seconds |
| `render` | encode the timeline, or a `range` of it, to a file | ffmpeg, and real time |

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

**Call `project_probe` after adding assets by writing the document.** Import
measures what it brings in; an asset that arrived by being written into
`project.json` carries a path and nothing recorded about the file behind it,
and every feature that needs the source's own length — the ceiling on a right
trim among them — has no choice but to skip it. This asks ffprobe about each
such asset and writes down what it says. Safe to call after every edit: one
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

## The two operations that write keyframes for you

`dissolve` and `duck_music` are sugar, and both keep the same bargain: what
they write is **ordinary keyframes** — the ones you would have placed by hand —
which stay visible, editable and deletable afterwards. Neither adds a stage to
the renderer, which is why a dissolve composes with a move or a turn for free.

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

## Looking at the frames

`still` is the only tool that answers with something other than words, and that
is the point of it. Everything else here *describes*: what the document says,
what the cut contains, what is wrong with it. An assistant that writes a title
and reads back "CHAPTER ONE, centred, 0.14 of the frame" still has no idea
whether it is readable, whether it collides with the shot under it, or whether
it is on screen at all.

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
and the pitch classes, then a row per track naming its source kind, its gain
and its filter cutoff; under them, the same facts counted across the project.
What each column means is in
[`recipes.md`](recipes.md#what-the-whole-set-is-made-of).

**It counts and stops.** There is no score, no grade, no recommendation and no
diversity number — a set of six variations on one instrument is a legitimate
thing to write on purpose, and a metric of variety is precisely the one that
would get optimised. A project of fewer than two songs has no set to report on.

## Stateless, on purpose

There is no server-side "open project". Every call names the project it works
on, so a client may crash, reconnect, or run two conversations against one
project without anything getting out of step.

## Every tool describes itself, and that is a gate

A tool's description is the entire interface a client has to it. An undescribed
tool is a capability that exists and cannot be found — nothing fails, the
assistant simply never calls it, and nobody ever learns why.

So `crates/mcp/tests/described.rs` walks the registry and fails on a tool or an
argument that says nothing useful about itself. The same gate already covers
`scorsese --help` and the animatable-property table in `project-format.md`.

## What it is not

No editing logic of its own. Every tool is a thin wrapper over the same library
the CLI calls, and a tool that needs code the CLI cannot reach means that code
is in the wrong crate.
