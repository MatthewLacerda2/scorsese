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

Every tool takes `project`: the path of the `*.scor` directory to work on.

| Tool | What it does | Costs |
| --- | --- | --- |
| `project_read` | the `project.json` exactly as it is on disk | nothing |
| `project_write` | replace it, **validated first** | nothing |
| `project_describe` | what is on screen when, and what is audible under it | nothing |
| `project_check` | every problem with the document and its media, at once | nothing |
| `project_assets` | the media pool and the state of everything in it | nothing |
| `project_probe` | measure the assets nobody has probed, and record it | ffprobe |
| `dissolve` | cross one shot into the next, as opacity keyframes | nothing |
| `duck_music` | lower a music track under narration, as volume keyframes | nothing |
| `synth_new` | start a recipe, and the asset that points at it | nothing |
| `synth_read` | a recipe file as it is on disk | nothing |
| `synth_write` | replace one, **parsed first** | nothing |
| `synth_check` | parse a recipe without rendering it | nothing |
| `synth_bake` | render the recipes not already baked | nothing |
| `audio_level` | how a finished sound file came out, and how it differs from another | ffmpeg |
| `still` | **look at** one frame, returned as a picture | ffmpeg, and seconds |
| `render` | encode the timeline to a file | ffmpeg, and real time |

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

## Looking at a frame

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

**It is the frame a render would deliver**, because it is the render pipeline
with the encoder taken out: the same plan, the same decoders, the same
compositor. Nothing is encoded and no video file is produced, so it costs
seconds rather than a render. Sketch and stale generated assets appear as slug
cards, exactly as they would in a preview cut.

The default raster is 1280x720 rather than a delivery size, because layout is a
fraction of the frame — the same picture with a fraction of the wire cost. Pass
`resolution` for delivery size. Pass `out` to keep the PNG on disk as well;
without it, nothing is left behind.

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

It is a **signal and never a gate** — there is no correct loudness — and it is
not a critic. It finds defects: too quiet, clipping, muddy, a section flat
where the arrangement said climax. It does not find taste, and a metric treated
as an ear produces music that optimises the number and gets worse.

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
