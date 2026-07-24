# `project.json` — schema v1

The contract between the CLI, the MCP server, the GUI, and every project
saved on someone's disk. It is meant to be hand-written: an agent should be
able to author a whole video in this file and render it without touching a
mouse.

Changing this format is `architecture` work — it needs a `schema_version`
bump and a migration note.

## The document

```json
{
  "schema_version": 1,
  "name": "Narrated teaser",
  "assets": [ ... ],
  "tracks": [ ... ]
}
```

`assets` and `tracks` may be omitted; both default to empty. Every other
field shown below as optional may be omitted, and omitted is written as
*absent*, never as `null`. Unknown fields are an error, not a warning — a
typo like `"trackz"` fails the load rather than being silently dropped.

## Assets

An asset is an entity. Clips point at assets **by id, never by path**, so
re-importing or regenerating a file is one edit in one place.

| Field | Required for | Meaning |
| --- | --- | --- |
| `id` | all | Unique within the project |
| `kind` | all | `video`, `image`, `audio`, `text`, `generated_video`, `generated_audio` |
| `path` | file-backed kinds | Relative to the project root |
| `sha256` | optional | 64 lowercase hex chars, of the file at `path` |
| `media` | optional | What ffprobe found: `duration_seconds`, `width`, `height`, `frame_rate`, `audio_channels`, `sample_rate` |
| `prompt` | `generated_*` | What to generate |
| `state` | `generated_*` | `sketch`, `queued`, `generated`, `stale` |
| `text` | `text` | The string to render; text assets carry content inline and have no `path` |

```json
{ "id": "shot-city", "kind": "generated_video", "state": "sketch",
  "prompt": "wide aerial of a city at dawn, slow push in" }
```

A `generated_*` asset in `sketch` or `stale` has no media yet and renders as
a slug card — the prompt on a gray card. That is what makes previewing a full
cut cost nothing. `GO` generates exactly the sketch and stale assets;
`generated` is a cache hit and is never re-billed.

## Tracks and clips

```json
{
  "id": "v1", "kind": "video", "name": "Main",
  "clips": [
    { "id": "c-shot", "asset": "shot-city", "start": 0.0, "duration": 8.0 }
  ]
}
```

A track is `video` or `audio`. Video tracks composite in array order, first
at the bottom; audio tracks all mix together. Visual assets go on video
tracks, audible ones on audio tracks.

A clip carries `start` and `duration` on the timeline, an optional
`source_in` offset into the media (default `0`), and optional `keyframes`.
Clips on one track may *touch* — one ending exactly where the next starts —
but never overlap.

**Times are seconds, not frames.** Frame rate is a render setting chosen per
render, so it is not known when the project is authored; a frame-based
timeline would silently bind a project to one fps.

## Keyframes

```json
"keyframes": [
  { "property": "opacity", "keyframes": [
      { "t": 0.0, "value": 0.0, "easing": "ease_in" },
      { "t": 0.5, "value": 1.0 }
  ]}
]
```

A keyframe track is `(property_path, [(t, value, easing)])` over any numeric
property. `t` is relative to the **start of the clip**, so moving a clip
never rewrites its keyframes. Times must ascend strictly. `easing` is
`linear` (default), `ease_in`, `ease_out`, `ease_in_out`, or `hold`.

`property` is a dotted string — `opacity`, `transform.position.x`, `volume`
— and core does **not** check that it names a property that exists. That is
the generality rule: core defines property types, never property values. The
compositor resolves paths; adding a new animatable property costs nothing
here.

## Paths

Every path is relative to the project root and uses forward slashes on every
platform. Absolute paths (`/media/x.mp4`, `C:/media/x.mp4`, `\\host\share`),
backslashes, and `..` components are all rejected. This is what lets a
project survive `scp -r` between machines.

## Validation

`Project::load` validates; `Project::save` does not, so an editor may save
work that is mid-edit and temporarily incoherent. Validation reports **every**
problem in one pass rather than stopping at the first, so an agent repairing
a project unattended sees the whole list at once.

What it checks: schema version, duplicate ids, path rules, hash shape, the
fields each asset kind requires, clip references resolving, asset kind
against track kind, times being finite and non-negative, non-zero durations,
clip overlap, and keyframe shape.

A complete worked example lives in
`crates/core/tests/fixtures/narrated_teaser.json`.
