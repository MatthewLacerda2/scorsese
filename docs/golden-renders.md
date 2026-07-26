# Golden renders

Every change from here on is a change to *pixels*, and pixels regress silently.
A transform half a pixel out, an easing curve flipped, a cut one frame early —
all of these compile, pass clippy, and produce a video file that looks finished
and is wrong. Nobody's eyes are in the loop for an agent-driven PR, so the
golden renders are what stands between "CI is green" and "the video is actually
right".

They are a **hard gate**. They run inside `cargo test --workspace` and they are
green-to-merge, no exceptions.

**Pixels only.** Sound is guarded a different way, in
`crates/render/tests/audio/`: those tests render real projects and ask how loud
the result is over a window of time. There is no committed reference waveform
and there should not be — a soundtrack that has been resampled and through a
lossy encoder is not reproducible sample for sample, and a binary reference
nobody can read by looking at it is a reference nobody can check. "Is there
sound at this second, and is it getting louder" is a question with an obvious
right answer, which is the property that makes a gate worth having.

## What a fixture is

A directory under `crates/golden/fixtures/`:

```
cuts/
  project.json        # a real project — the format is part of what is tested
  fixture.json        # how to conjure the media, how to render, what to compare
  expected/
    frame-0000.png    # reference frames, a few hundred bytes each
    frame-0029.png
```

Media is **generated at test time** from ffmpeg's synthetic sources — solid
colours, test patterns — so the repository never carries sample footage. The
recipe lives in `fixture.json`, keyed by asset id; the file name comes from the
asset's `path` in `project.json`, so it is written down once.

`fixture.json` names the frames to compare. Choose the boundaries: the last
frame of a clip and the first frame of the next, either side of a gap, the first
and last frame of a held still. That is where an off-by-one hides, and a fixture
that samples the middle of a long clip asserts almost nothing.

`description` is not decoration. It is what a future reader — human or agent —
uses to decide whether a change to this fixture's references is legitimate, so
say what would break if the fixture were wrong.

## How comparison works

Never byte-equality of encoded output. Two ffmpeg builds encoding the same
frames produce different bitstreams; the bytes of an `.mp4` are not ours to
assert on. **Frames are.**

Each compared frame is measured two ways, both with tolerance:

- **SSIM** on the worst 8×8 block, on luma — catches *where* pixels are. A shape
  half a pixel out, a soft edge, a wrong patch in one corner.
- **Mean absolute error** per colour channel — catches *what* pixels are. A wrong
  colour, a frame a few levels bright, a cut one frame early.

Neither alone is enough and each catches what the other waves through;
`crates/golden/tests/comparing.rs` pins that to specific cases. Defaults are
`min_ssim` 0.95 and `max_mean_error` 2.0, overridable per fixture under
`tolerance`. They are deliberately loose enough to survive a different
platform's x264 and far tighter than any real regression — red against blue
scores 0.70 and 169.

## Re-blessing

```sh
UPDATE_GOLDENS=1 cargo test -p scorsese-golden
```

This rewrites every reference frame of every fixture. Because references are
PNGs, the change arrives in review as an **image diff** — which is the only form
in which a human can judge whether it was legitimate.

**Re-blessing is legitimate when** the render changed on purpose and the new
frames are what was intended: a compositor capability that alters output by
design, a fixture deliberately re-aimed at something else, an intentional change
to the conform or letterbox rules.

**Re-blessing is not legitimate** as a way to make CI green. If you cannot say,
in the PR description, what visual change you intended and why the new frames
are right, the failure is a bug and the reference was already correct. A golden
gate re-blessed reflexively is worse than no gate, because it still reads as
coverage.

Two rules keep that honest:

- A missing reference **fails** rather than being created silently. A fixture
  with nothing to compare against would otherwise pass by asserting nothing.
- Re-blessed PNGs must be in the diff. A reviewer seeing `expected/*.png` change
  should expect the PR description to explain it.

## When a golden fails

The failure names every frame that fell outside tolerance, its scores, and two
paths: the committed reference and the frame that was actually produced. The
render itself is kept on disk rather than cleaned up, because watching it is
usually the fastest diagnosis. In CI those frames are uploaded as the
`golden-failures` artifact.

## Adding a fixture

1. Make the directory with `project.json` and `fixture.json`.
2. Add its name to the `goldens!` list in `crates/golden/tests/goldens.rs`. If
   you forget, `every_fixture_is_covered` fails — an untested fixture sitting in
   the repository looking like coverage is exactly the failure mode this whole
   file exists to prevent.
3. Create the references with `UPDATE_GOLDENS=1`, then **look at them** before
   committing. A blessed reference is an assertion about what is correct; if you
   did not check it, you have asserted that whatever the code did was right.
