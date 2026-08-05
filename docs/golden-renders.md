# Golden renders

Every change from here on is a change to *pixels*, and pixels regress silently.
A transform half a pixel out, an easing curve flipped, a cut one frame early —
all of these compile, pass clippy, and produce a video file that looks finished
and is wrong. Nobody's eyes are in the loop for an agent-driven PR, so the
golden renders are what stands between "CI is green" and "the video is actually
right".

They are a **hard gate**. They run inside `make test` — the workspace suite,
through `cargo nextest run` — and they are green-to-merge, no exceptions.

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
    decoder.txt       # an ffmpeg known to produce exactly those frames
```

Media is **generated at test time** from ffmpeg's synthetic sources — solid
colours, test patterns — so the repository never carries sample footage. The
recipe lives in `fixture.json`, keyed by asset id; the file name comes from the
asset's `path` in `project.json`, so it is written down once.

A fixture may also carry an `assets/` directory of its own, which the harness
copies into the scratch project verbatim. That is for the files ffmpeg cannot
conjure, and there is exactly one kind of them today: a **font**. A face is not
footage — it is the same bytes on every machine, and no `lavfi` source will
ever produce one — so a fixture testing that a project can bring its own font
has no other way to say so. `weight` is the fixture that does. The rule above
is unmoved: media is still generated, never committed.

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

Frames come out of the render through `scorsese_render::frames`, not through
anything this harness owns. Getting a frame in and out of a file is a shipped
capability — `scorsese render --stills` writes PNGs with the same code — and
nothing may depend on `crates/golden`, so the harness is a caller of it rather
than the owner. Every fixture passing unchanged across that move is what says
the two were the same code.

**`scorsese still` is not a shortcut for this, and must never become one.** It
composites a frame and writes it out without opening an encoder, which is what
makes it fast and what makes it useless here: this gate exists to catch what a
*delivered file* holds, and a picture that never went through an encoder cannot
answer that. The two are halves of one question — `still` asks what the
compositor produced, `--stills` asks what came back out of the file — and
swapping one for the other would quietly narrow what these fixtures prove to
something they already proved.

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

## The decoder, which sits upstream of all of that

"Frames are ours to assert on" is a claim about the **encoder** at the end of a
render. ffmpeg is also the **decoder** at the start of one: every pixel the
compositor works on arrived through it, so a different ffmpeg is a difference
the tolerances above never accounted for. They were sized for encoder noise and
say so.

CI and a development machine are not on the same one. CI runs Ubuntu 24.04's
`6.1.1-3ubuntu5`; Arch currently ships `n8.1.2`. Two majors apart, comparing
pixels.

**Measured, on the fixture set as it stands: they agree exactly.** Every
compared frame of every fixture, decoded by 6.1.1 and by 8.1.2, comes out
bit-identical — SSIM 1.0000 and mean error 0.000, not merely inside tolerance.
That is not luck. H.264 specifies its inverse transform exactly, so conformant
decoders are obliged to produce identical samples; it is *encoding* that is
free to differ, which is the asymmetry this whole gate is built on. The
synthetic flat-colour sources these fixtures use give swscale nothing to
disagree about either.

So this is a gap that has never bitten and, for the codecs in use, has no
mechanism to. Two things follow from it, and neither is a check that can fail:

- **CI pins its runner image** to `ubuntu-24.04` rather than `ubuntu-latest`.
  A moving label would change the decoder under the pixel gate by a major
  version the day GitHub rolls it, with nothing in any diff to say so — the
  same reasoning as `rust-toolchain.toml`'s. Every CI run also prints
  `ffmpeg -version`, so which decoder ran is a fact in the log rather than
  something to reconstruct later.
- **Each fixture records a decoder** in `expected/decoder.txt`: an ffmpeg known
  to produce exactly those frames. Blessing rewrites it in the same act that
  rewrites the references, so it can never describe an older set of frames than
  they are. The committed records name CI's `6.1.1-3ubuntu5`, from the
  measurement above.

The record **never fails anything, and never warns on a passing run**. It
speaks in one place: inside the report of a fixture whose frames already
disagree, where it names the ffmpeg that produced them and the one the
references were recorded under. A failure that would otherwise read as "the
pixels moved" reads as "the pixels moved, and you are on a different decoder —
rule that out first".

Failing on a mismatch instead would make the suite unrunnable for anyone whose
distribution ships something else, which is a gate going red for a cause its
author cannot see in their own diff. Warning on every green run would train
everyone to scroll past it. `crates/golden/tests/provenance.rs` holds both
halves to specific cases, including that a fixture whose record names a
different ffmpeg still passes when its frames match.

**It is context, not permission.** Re-blessing because the report mentioned a
decoder difference is the same illegitimate act as re-blessing for any other
reason, and it would bake one machine's decoder into the reference.

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
paths: the committed reference and the frame that was actually produced. It
also names the ffmpeg that decoded them, and whether that is the one the
references were recorded under — see the decoder section above for what to do
with that, and what not to. The render itself is kept on disk rather than
cleaned up, because watching it is usually the fastest diagnosis. In CI those
frames are uploaded as the `golden-failures` artifact.

## Adding a fixture

1. Make the directory with `project.json` and `fixture.json`.
2. Add its name to the `goldens!` list in `crates/golden/tests/goldens.rs`. If
   you forget, `every_fixture_is_covered` fails — an untested fixture sitting in
   the repository looking like coverage is exactly the failure mode this whole
   file exists to prevent.
3. Create the references with `UPDATE_GOLDENS=1`, then **look at them** before
   committing. A blessed reference is an assertion about what is correct; if you
   did not check it, you have asserted that whatever the code did was right.
   The same run writes `expected/decoder.txt` naming the ffmpeg that produced
   them; commit it with the PNGs, since on its own it would describe frames
   that are not there.
