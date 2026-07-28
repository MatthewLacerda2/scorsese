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
| `duck_music` | lower a music track under narration, as volume keyframes | nothing |
| `synth_new` | start a recipe, and the asset that points at it | nothing |
| `synth_read` | a recipe file as it is on disk | nothing |
| `synth_write` | replace one, **parsed first** | nothing |
| `synth_check` | parse a recipe without rendering it | nothing |
| `synth_bake` | render the recipes not already baked | nothing |
| `render` | encode the timeline to a file | ffmpeg, and real time |

**The edit is the document.** `project_read` and `project_write` are the pair
that makes everything else possible: the whole cut is one JSON file, so any
change at all is read it, change it, write it back. The format is
[`project-format.md`](project-format.md).

`project_write` **validates before writing**. A document that would not load is
refused with every problem listed and the file on disk is left exactly as it
was — so a half-formed edit cannot destroy a working one.

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
