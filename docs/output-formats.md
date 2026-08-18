# Output formats

What a render *imports* is not what it *delivers*. Decoding is
format-agnostic and costs nothing — the decode pipe says `-i <file>` and asks
for raw RGBA back, so wmv, avi, mov, mkv and mp4 all import and sit on a
timeline with no code that knows what any of them are.

One exception, and it is about *stills* rather than containers: holding one
picture for a clip's length is an ffmpeg option, and the cheap way to say it —
`-loop` — belongs to the demuxer that reads png and jpeg rather than to ffmpeg
itself. So a format with frames of its own, a gif or an avif, is held by
looping the container instead. That is the only place the decode side knows
what a file is, and [project-format.md](project-format.md) says what it means
for an animation.

Encoding is the opposite: every file we write is a choice, and it used to be
made by accident. The container came from whatever extension the output path
happened to have, and the codecs were two hardcoded strings in the encoder. So
`--out cut.wmv` produced an ASF file carrying H.264 — a Windows Media file in
name only — and said nothing, after spending the whole encode. For an agent
rendering unattended a plausible-looking wrong file is far more expensive than
a refusal.

So the shape of the delivered file is a **render setting**, next to resolution,
fps and bitrate. Like those, it is chosen per render and never stored in the
project: `project.json` describes the edit, not the deliverable.

## What scorsese writes

The first codec in each row is that container's **default** — what you get when
you name a file and nothing else.

| container | extension | video | audio |
| --- | --- | --- | --- |
| `mp4` | `.mp4` | `h264` | `aac` |
| `mkv` | `.mkv` | `h264` | `aac` |
| `avi` | `.avi` | `mpeg4`, `h264` | `pcm_s16le` |
| `wmv` | `.wmv` | `wmv2` | `wmav2` |

Anything not in that table is refused. `crates/render/tests/formats/` holds this
page to the code, so a row here that the code does not write — or a codec the
code writes that this page never mentions — fails the build.

### Why that list

- **mp4/H.264/AAC** is what delivering a video means. It is the default, and
  nothing about this setting changed what a render did before it existed.
- **mkv** is the same pair in the container that carries anything, for when
  mp4's constraints are the problem.
- **avi** gets MPEG-4 Part 2 and PCM, which is what an AVI is expected to hold.
  H.264 in an AVI is legal and occasionally wanted, so it is available — but by
  asking, never by default. That one row is the whole point of the setting:
  the container and the codec are separate decisions.
- **wmv** is ASF carrying WMV 8 and WMA 2. Anything else in a `.wmv` is not a
  Windows Media file in a sense a viewer of one would accept, so H.264 there is
  refused rather than delivered.

Every encoder named is either already required by the default path (`libx264`)
or built into ffmpeg itself with no external library behind it (`mpeg4`,
`wmv2`, `aac`, `pcm_s16le`, `wmav2`). That is deliberate. "Whatever ffmpeg has"
is not an answer we can stand behind: *which* ffmpeg is a shipping decision — a
distro build in dev and CI, a bundled Tauri sidecar in a shipped build — and a
licensing one. A short list we test beats a long list we assume.

Exporting to wmv or avi is rare, and this is not an argument that it is common.
The next likely ask is webm, not wmv; avi and wmv are here because they are the
cases that prove the mechanism generalises.

## Choosing one

```sh
scorsese render --out cut.mp4                       # mp4, h264 + aac
scorsese render --out cut.avi                       # avi, mpeg4 + pcm_s16le
scorsese render --out cut.avi --video-codec h264    # avi, h264 + pcm_s16le
scorsese render --out cut.mp4 --container mkv       # matroska, whatever the file is called
scorsese render --out cut.wmv --video-codec h264    # refused, before anything is encoded
```

`--container` defaults to what `--out`'s extension asks for, so naming the file
is usually the whole decision and every invocation written before this existed
still means what it meant. `--video-codec` and `--audio-codec` default to what
the container is written with.

The extension is a **default**, not the answer: `--container` wins over it, and
the muxer is pinned with ffmpeg's `-f` so the file is what the setting says
whatever it is called.

## When a combination is refused

The refusal happens where the setting is built — before the project is opened,
before ffmpeg is even located, and minutes before anything would have been
encoded:

```
$ scorsese render --out cut.wmv --video-codec h264
error: scorsese does not write wmv with h264; it writes wmv with: wmv2
```

That is structural rather than a check somebody has to remember to run. A
`RenderSettings` holds an `OutputFormat`, and an `OutputFormat` cannot be
constructed around a combination we do not write — so a combination that would
produce a file nobody wants has no way to reach an encoder.
