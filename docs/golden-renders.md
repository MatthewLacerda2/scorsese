# Golden renders

Every change from here on is a change to *pixels*, and pixels regress silently.
A transform half a pixel out, an easing curve flipped, a cut one frame early —
all of these compile, pass clippy, and produce a video file that looks finished
and is wrong. Nobody's eyes are in the loop for an agent-driven PR, so the
golden renders are what stands between "CI is green" and "the video is actually
right".

They are a **hard gate**. They run inside `make test` — the workspace suite,
through `cargo nextest run` — and they are green-to-merge, no exceptions. They
run **on Linux**, and are skipped and reported as skipped anywhere else. That
is a statement about where this comparison is authoritative rather than a
softening of the one before it; *The platform* below is the whole of why.

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

## What this gate cannot hold: a picture made of noise

Both measures above assume a picture the encoder reproduces. **Grain breaks that
assumption**, and it breaks it in a way that cannot be tuned around, so a
committed reference of a grained frame is not a fixture anybody should add.

Measured, at 64×64 and `-crf 18`, encoding the same noisy frames at two very
different x264 presets — a smaller perturbation than two x264 *versions*:

| noise | worst-block SSIM | mean error |
| --- | --- | --- |
| σ ≈ 2 | 0.985 | 0.76 |
| σ ≈ 4 | 0.954 | 1.09 |
| σ ≈ 8 | 0.949 | 1.4 |

The SSIM column is the problem, and the reason is structural rather than
incidental: on a flat plate under noise the whole of a block's variance *is* the
noise, so the structure term collapses to how well two encoders happened to
agree about it. It sits at the 0.95 bar in the direction that must pass. It sits
there in the other direction too — a frame whose grain had vanished entirely
scores about the same — so widening the tolerance does not buy a working gate,
it buys one that accepts everything. And the light grain that would keep SSIM
comfortable is a grain x264 deletes outright: at σ ≈ 3 on a flat plate, a P
frame came back **bit-identical to the ungrained plate**.

So grain, and anything else whose output is fine noise, is pinned where the
claim can be exact instead:

- **`crates/compositor/tests/grading/grain/`** — the pixels themselves, before
  an encoder has seen them: the same seed twice, a frame drawn alone against the
  same frame drawn after five others, one value on all three channels. And
  `pinned.rs`, which is this gate's job done another way — one exact frame and
  one exact seed written down as literals, so "the same project renders the same
  picture on every machine" is asserted rather than assumed. Exact rather than
  within a tolerance, because no encoder stands between.
- **`crates/render/tests/pipeline/grain.rs`** — through a real render, in the
  two places a whole file is what is being claimed about: two renders of one
  project extract to identical frames, and a `--range` out of the middle of a
  timeline lands the same grain on the same timeline frame. The second compares
  *distances* rather than colours, because a ten-frame encode and a thirty-frame
  one legitimately differ.

This is not a hole in the pixel gate so much as its edge, written down. The
fixtures here are flat synthetic colour, which is what makes their tolerances
mean what they say.

## What it can hold, and where that line falls: a picture made of edges

The section above is the answer for one effect, not a rule about effects, and
the next one asked the same question and got the opposite answer. **Chromatic
aberration** (`aberration`) displaces red and blue against green, which sounds
like a small change and is not the same *kind* of small change: what it produces
is a band of solid colour a couple of pixels wide along every edge in the
picture. That is structure, and structure is what an encoder is built to keep.

Measured on the `aberration` fixture as it stands — the same two-preset
perturbation grain was measured under, `-crf 18` at `ultrafast` and `veryslow`,
alongside what the gate scores when the effect is removed and the reference is
left in place:

| the fixture's `aberration` | encoder noise, two presets | with the effect removed |
| --- | --- | --- |
| `0.008` — fringes under a pixel | 0.9597 / 0.77 | 0.9582 / 5.49 |
| `0.015` — the committed fixture | **0.9889 / 0.98** | **0.8877 / 9.48** |
| `0.025` | 0.9630 / 0.96 | 0.8128 / 14.68 |

Each cell is worst-block SSIM and mean error; the bars are 0.95 and 2.0.

At `0.015` the gate works and works with room: encoder noise leaves SSIM at
0.9889, and taking the effect away drops it to 0.8877 — the bar sits between the
two rather than on top of either, which is exactly what grain could not manage.
For scale, the `shapes` fixture already in this set scores **0.9708** under the
same two-preset perturbation, so an aberrated frame is *more* robust to an
encoder difference than one already gating merges.

**The bottom row of that table is the finding worth keeping.** At `0.008` the
fringes are under a pixel wide, and the two numbers collapse onto each other —
0.9597 against 0.9582, both within a whisker of the bar, on opposite sides of a
question the gate then cannot answer. A sub-pixel split is not structure, it is
texture, and it fails here for grain's reason. So the rule this fixture is built
on is: **the split has to be wider than the encoder's smallest honest detail**,
and a fixture demonstrating a *tasteful* aberration would be a fixture that
asserts nothing. `0.015` on a 192×108 raster is far past what anybody would put
in a project, and that is deliberate, the same way `blur_heavy` is.

None of this weakens the section above. Grain is still unfixturable and this is
still why: the question is never "is the effect subtle" but "is what it produces
a thing an encoder reproduces", and noise and edges answer it differently.

## And where it falls easily: a picture made of regions

The two sections above are the hard cases. A **chroma key** is the easy one, and
it is worth the paragraph because it shows which measure does the work.

What a key produces is a *region* — a boundary between two solid colours, in a
new place. That is the most reproducible thing an encoder handles, so the two
`chroma_key` fixtures discriminate by more than an order of magnitude. Measured
the same way as the two above, `-crf 18` at `ultrafast` against `veryslow`
alongside what the gate scores with the effect taken away and the reference left
in place:

| the claim | encoder noise, two presets | with it removed |
| --- | --- | --- |
| `chroma_key`, the matte | 0.9981 / 0.43 | 0.7763 / **52.96** |
| `chroma_key`, the despill alone | 0.9981 / 0.43 | 0.9917 / **7.17** |
| `chroma_key_lit`, the matte | 0.9707 / 0.18 | 0.6017 / **61.37** |
| `chroma_key_lit`, against an RGB keyer | 0.9707 / 0.18 | 0.6009 / **34.90** |

Worst-block SSIM and mean error again; the bars are 0.95 and 2.0. The last row
is not the effect removed but the effect *implemented wrongly* — a keyer
measuring distance in RGB rather than with the light divided out, which keys the
lit third of that fixture's screen and leaves the other two thirds standing.
That is the bug the fixture exists for, and it is caught as loudly as no key at
all.

**The second row is the one worth reading.** SSIM does not catch the despill —
0.9917, comfortably inside a 0.95 bar — because a despill changes what a region
*is* without moving where it is, and structure is exactly what SSIM measures.
The mean error catches it three and a half times over. That is the two-measure
rule paying for itself rather than being asserted: neither number alone is a
gate, and here it is the one grain and aberration were both judged on that says
nothing at all.

**It is also why that fixture's spilled block is half the frame.** At the width
a spill rim actually is — six pixels around a block — the same removal moved the
mean error by 0.83, inside the bar, and the fixture asserted the matte while
looking as though it asserted the suppression too. A mean is a mean over the
whole frame, so an effect confined to a thin border has to be given area before
this gate can see it, exactly as an aberration has to be given width. Both are
the same rule wearing different clothes: **make the fixture exaggerate whatever
the gate measures the effect by**, and say in the description that it does.

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

## The platform, which decides whether any of that runs

The section above is about a decoder difference these tolerances survive.
There is one they do not, and it is why this gate has a platform at all.

**The fixture tests run on Linux and are skipped everywhere else.** Not
compiled out — `#[ignore]`d, so they still have to build, so
`cargo nextest run -p scorsese-golden --run-ignored all` still runs them on
purpose, and so the runner *counts* the skip instead of reporting a pass over
it. `make test` and `make gates` then say it in a sentence, exactly as
`make gates` says the desktop app's gates did not run: never green over a check
that did not run.

What forced it: on macOS with homebrew's ffmpeg `8.1.2`, four grade fixtures —
`grade_brightness`, `grade_contrast`, `grade_ramp`, `grade_saturation` — fail on
an unmodified `main`. SSIM is 0.9992, so the pixels are in the right *places*;
the entire failure is a mean error of about three levels out of 255, uniform and
always positive. The arithmetic is not in doubt: `crates/compositor` asserts
exact grade output values in memory, with no tolerance and no ffmpeg anywhere,
and every one of those passes on that machine. The shift lives entirely in the
encode/decode round trip the golden path goes through. There is no bug to fix,
which is what makes this a question about the gate rather than about the code.

**Keyed on the platform, not on a decoder mismatch.** Those two look like the
same rule from the machine that prompted this, and they are opposites where it
matters. `alpha`'s reference was blessed under ffmpeg `8.1.2` and passes on CI's
`6.1.1-3ubuntu5` — two majors apart, no complaint. A rule that skipped whenever
the running ffmpeg differed from `decoder.txt` would therefore have skipped on
CI, which is the one place this gate must run. Linux is where references are
blessed and where a merge is gated, so the platform is the honest thing to key
on and the record stays what it already was: context for a failure, never a
switch.

**Not by widening a tolerance**, which was the first instinct and the wrong one.
`max_mean_error` is not per-platform: raising it to absorb three levels on macOS
raises it on Linux too, and a grade fixture that tolerates a three-level shift
cannot catch a three-level grade bug — anywhere, ever. That trades away
sensitivity on the platform where the gate works in order to accommodate the one
where it does not. **And not by re-blessing**, for the reason the section above
already gives.

What it costs, said out loud: someone working off Linux can break a fixture and
not find out until CI. That is accepted. It is already true of the desktop app's
gates for the same kind of reason, the gate still gates — it gates later — and
the alternative was in force for a while and was worse. It was every agent on
that machine being told that four named failures are fine to ignore: a standing
exception that has to be judged correctly every session, over a set that only
grows, in a repo whose whole merge story is that a green gate means something.

**The harness's own tests are not skipped**, and keeping that distinction
straight matters. `comparing.rs` compares images it builds itself; `failing.rs`
and `provenance.rs` vandalise a private copy of `letterbox` and insist the
harness notices, names the frame, and leaves the evidence. Those ask whether the
*comparison* works, which is a question with the same answer on every platform.
So does `every_fixture_is_covered`, which reads the fixture directory and no
frames at all. What is Linux-only is the claim that the shipped pixels are
right.

## The faces, which sit upstream of it just as much

The decoder is not the only input the environment would otherwise choose. **The
shipped faces are the other**, and they are held still for exactly the
reason ffmpeg is: a system-font lookup resolves to a different file on Linux,
macOS and Windows, so text drawn on one machine would not match text drawn on
another and a reference blessed anywhere would fail everywhere else. Faces
compiled in with `include_bytes!` are to text what a pinned runner image is to
decoding — refuse to let the environment pick, carry the input instead. The
provenance, the licence conditions and the full reasoning are in
[`crates/compositor/fonts/README.md`](../crates/compositor/fonts/README.md);
this section exists so the rulebook names both things the gate rests on rather
than one of them.

This is a different claim from the font mentioned under *What a fixture is*.
There a face is a fixture **asset** — the case of a project bringing its own,
which `weight` is the fixture for, and which a fixture directory records. Here
it is the default `sans` and `serif` every other text fixture draws with, which
nothing in a fixture directory records at all.

**The fallback face is in the same position and is easier to forget**, because
no document names it. A character the named face has no glyph for is drawn from
Noto Color Emoji, so that file is an input to `emoji`'s reference exactly as
Inter is to `title`'s — and it is reached by *coverage*, which means a face
whose coverage changed would move pixels in a fixture whose `project.json` says
nothing about it.

What follows from it is the part a rule is for. **The shipped faces may not be
swapped, subsetted, or resolved from the system without re-blessing every text
fixture.** Subsetting counts as modifying the file, and modifying the file moves
the pixels. `anchored`, `captioned`, `emoji`, `paragraph`, `serif`, `slugs`,
`title`, `title_moved` and `wash` are drawn with them — so this is not a corner
of the gate.

`slugs` is on that list because a slug card's text is set in `sans` like any
other, which is easy to miss when looking for fixtures with a `text` asset in
them. `weight` is **not**, and the reason is worth knowing: it sets its title in
a font the *project* carries, which is the whole of what that fixture tests. It
is the one that stays still when the shipped faces move, and that is a feature.

The **desktop app's reference images are in the same position**, and are not in
this directory. `app/tests/snapshots/` holds pictures of the window, and the
window's preview draws a compositor frame — so a change to the shipped faces
moves those too, under the same rules, even though nothing under `crates/golden`
noticed.

Such a re-blessing is legitimate only under the rules below: a deliberate visual
change, said out loud in the PR description, arriving as image diffs a human can
look at. It is never legitimate as a way to get a text fixture green again.

## Re-blessing

```sh
UPDATE_GOLDENS=1 cargo test -p scorsese-golden
```

This rewrites every reference frame of every fixture. Because references are
PNGs, the change arrives in review as an **image diff** — which is the only form
in which a human can judge whether it was legitimate.

**On Linux**, and that is not a detail. Off it the fixtures are ignored, so this
command blesses nothing at all until `-- --include-ignored` is added — a small
hurdle standing in front of exactly the act that should never be casual. A
reference blessed elsewhere holds that machine's decode of the frames and
records that machine's ffmpeg in `decoder.txt`, and the next thing to compare it
against is CI. A set of references nobody has checked on the platform that gates
is a whole fixture set going red for a reason no diff explains.

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
with that, and what not to. If the fixture renders text, the shipped faces are
the other upstream input to rule out before suspecting the code — the section
on them above names the seven that do. The render itself is kept on disk rather
than cleaned up, because watching it is usually the fastest diagnosis. In CI
those frames are uploaded as the `golden-failures` artifact.

## Adding a fixture

1. Make the directory with `project.json` and `fixture.json`.
2. Add its name to the `goldens!` list in `crates/golden/tests/goldens.rs`. If
   you forget, `every_fixture_is_covered` fails — an untested fixture sitting in
   the repository looking like coverage is exactly the failure mode this whole
   file exists to prevent.
3. Create the references with `UPDATE_GOLDENS=1`, on Linux — see *Re-blessing*
   for why the platform is part of the instruction. Then **look at them** before
   committing. A blessed reference is an assertion about what is correct; if you
   did not check it, you have asserted that whatever the code did was right.
   The same run writes `expected/decoder.txt` naming the ffmpeg that produced
   them; commit it with the PNGs, since on its own it would describe frames
   that are not there.
