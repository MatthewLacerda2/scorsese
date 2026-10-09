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
for an animation. A decode that starts part-way through such a clip — a
preview, a partial render, a cut on another track — starts that far into the
animation, modulo its length, which the render measures from the file because
the assets table deliberately never records it.

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
you name a file and nothing else. A `—` means the container has no picture in
it at all.

| container | extension | video | audio |
| --- | --- | --- | --- |
| `mp4` | `.mp4` | `h264` | `aac` |
| `mkv` | `.mkv` | `h264` | `aac` |
| `avi` | `.avi` | `mpeg4`, `h264` | `pcm_s16le` |
| `wmv` | `.wmv` | `wmv2` | `wmav2` |
| `mp3` | `.mp3` | — | `mp3` |
| `wav` | `.wav` | — | `pcm_s16le` |
| `m4a` | `.m4a` | — | `aac` |

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
- **mp3** is what "an audio file" means to almost everyone, and the user asked
  for it by name (#505): a soundtrack for another editor, a jingle, a music bed
  to send a client. Before it existed, two synth scores were delivered as
  minutes of 1080p black frames with a caption, because a render could only
  ever be a video.
- **wav** and **m4a** came along because each cost one row: PCM for sound going
  into another tool, AAC in the container a phone or a music library expects.

Every encoder named is either already required by the default path (`libx264`)
or built into ffmpeg itself with no external library behind it (`mpeg4`,
`wmv2`, `aac`, `pcm_s16le`, `wmav2`) — with **one** exception, `libmp3lame`.
That is deliberate. "Whatever ffmpeg has" is not an answer we can stand behind:
*which* ffmpeg is a shipping decision — a distro build in dev and CI, a bundled
Tauri sidecar in a shipped build — and a licensing one. A short list we test
beats a long list we assume.

`libmp3lame` is the exception because mp3 is the format people mean by "an
audio file", and a sound-only delivery without it answers the request with
something else. It is in the distribution ffmpeg on the development machine
(Arch), in CI's (`ubuntu-24.04`) and in Homebrew's, so it is tested everywhere
we test; what it costs is that a **shipped** sidecar ffmpeg has to be built with
it. And because an ffmpeg without it is still a legitimate build, a render in
mp3 asks the ffmpeg on hand for the encoder before it mixes or encodes anything,
and refuses in words that name it when it is missing:

```
error: the ffmpeg on hand was built without libmp3lame, which mp3 is encoded with — …
```

That refusal is the rule for any future encoder that comes from a library: the
codec says so ([`AudioCodec::library`](../crates/render/src/format/codecs.rs)),
and the render asks before it spends.

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
scorsese render --out score.mp3                     # the soundtrack alone, no picture
scorsese render --out score.mp3 --resolution 1280x720   # refused: an mp3 has no picture
```

`--container` defaults to what `--out`'s extension asks for, so naming the file
is usually the whole decision and every invocation written before this existed
still means what it meant. `--video-codec` and `--audio-codec` default to what
the container is written with.

Over MCP the `render` tool takes the same three choices as `container`,
`video_codec` and `audio_codec`, built by the same constructor
(`OutputFormat::for_path`), so a refusal reads identically from either client.

The extension is a **default**, not the answer: `--container` wins over it, and
the muxer is pinned with ffmpeg's `-f` so the file is what the setting says
whatever it is called.

## Sound only

`mp3`, `wav` and `m4a` carry the timeline's mix and nothing else. A render into
one **never reaches the compositor** — no source is decoded for its picture, no
frame is drawn — because the frame loop is the expensive part of a render and
none of it is wanted. The mix is made exactly as it is for a video of the same
project: every clip volume, keyframe and duck, and the same length, since
picture decides how long an edit runs and a bed that outlasts the last shot is
trimmed there, with the same note saying so. A project with nothing on any
video track has no picture to decide, so there the audio tracks set the length
instead; a video render of that project is refused for having none.

Everything that only means something to a picture — `--resolution`, `--fps`,
`--bitrate`, `--video-codec`, `--threads`, `--stills` — is **refused** for a
sound-only format rather than ignored, because a flag that silently does
nothing is the worse of the two: whoever passed it thought it would matter. A
timeline on which nothing makes a sound is refused too, rather than delivered
as a file of silence.

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

## A lossy codec gets room to overshoot

`aac`, `wmav2` and `mp3` are **lossy**: they keep what they can afford of the
spectrum and rebuild a waveform from it, and the rebuilt peaks land above the
originals. How far depends on the material — about 1 dB on one synth score,
3.3 dB on a denser one, both measured on real projects (#503) — so a mix
sitting safely at −1 dBTP could come out of the encoder over full scale,
clipped in the file a viewer plays.

So before a lossy delivery is encoded, the finished mix is **rehearsed**:
encoded on its own with the same codec, bitrate and container, decoded, and
measured. If it comes back over **−1 dBTP** — the ceiling a synthesis bake is
already limited to — the whole mix is turned down by exactly the excess, and
rehearsed again. A sound-only delivery is encoded by the very call that
rehearses it, so for an mp3 or an m4a what was measured is what is written.
It is a uniform trim, never a limiter, so the mix's dynamics are the author's;
and it is paid only when the codec is lossy and only as far as that material
needs. `pcm_s16le` hands back what it was given and is never touched.

The render says what it did. Its report carries the delivered file's own level,
read back out of the file after encoding, beside the mix's — they differ
whenever the codec is lossy, and the file's is the one a clipping verdict is
about — and a line saying how far the soundtrack was turned down, when it was.
`scorsese render` and the MCP `render` tool both say those two things, in the
same words from the same function (#519).
`crates/render/tests/audio/loudness/headroom.rs` holds this to a square wave
that overshoots AAC by more than 3 dB.

## Delivering at a loudness target

Turning a mix down is all the section above ever does. Bringing a quiet one
**up** — so a short made for a social feed plays as loud as the feed around it,
since every feed normalises what it plays — is a separate request, and an
opt-in one: `scorsese render --loudness -14`, or the MCP `render` tool's
`loudness: -14`. Without it nothing in this section runs, and a render is what
it was before the option existed, bit for bit.

With it:

1. The finished mix is measured in **LUFS** — ITU-R BS.1770-4's gated,
   K-weighted loudness, the unit platforms publish their figures in
   (`scorsese_zimmer::level::Integrated`, held to EBU Tech 3341's reference
   signals and, in the render's tests, to ffmpeg's own `ebur128` meter).
2. It is raised or lowered by the difference, and every peak that gain pushes
   past **−1 dBTP** is held there by a **true-peak limiter**: linked across the
   two channels, with 2 ms of lookahead and a 60 ms release, judged on the
   waveform between samples the way the report's meter is. Limiting takes some
   loudness back, so the result is measured again and the gain corrected until
   it lands within 0.1 LU of the target.
3. The lossy rehearsal above runs as it always does. When a codec still needs
   room, the mix is raised again with the limiter's ceiling lowered by that
   room, so the codec's headroom comes out of the peaks rather than out of the
   target.

The limiter is the one place a render changes a mix's dynamics, and a target is
the author asking for exactly that. It stays deterministic — the same mix
limits the same way whatever size of chunk it is streamed through — and it has
a limit of its own: no more than **6 dB** of gain beyond the plain difference is
added to make up for what limiting takes. A mix whose loudness lives in its
peaks (a drum hit, a burst of distortion) would only be crushed past that, so
the render stops short instead.

The report says what it took, beside the file's level and any trim. A quiet
music bed with sharp clicks on it, delivered as an mp4 (ffmpeg's `ebur128`
reads the same −25.9 and −14.0 from the two files; the ceiling is −1.4 rather
than −1.0 because AAC needed that much room):

```
loudness -25.9 LUFS as mixed, -14.0 LUFS delivered (target -14.0 LUFS): raised 12.5 dB, the limiter taking up to 10.9 dB off peaks to hold them under -1.4 dBTP
```

A file that lands more than 0.5 LU short of its target says by how much, and a
silent soundtrack — or one shorter than the 0.4 s a measurement needs — says
there was no loudness to bring anywhere.

**A number, not a destination, for now.** Presets named for where a video is
going (`reels`, `tiktok`, `shorts`, `youtube`) are planned on top of the number,
each figure copied from that platform's own published guidance with its date.
Those come from the platform studies under #977; until they land, no platform's
figure is written here, or anywhere in the code, from memory. The range a target
may take is −40 to −5 LUFS: nothing is delivered quieter, and past −5 a mix is
nearly all limiter.
