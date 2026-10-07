# Reference tracks — what finished music measures

Five commercial recordings that a zimmer score is supposed to end up sounding
*like*, with the numbers `scorsese level` (`hear` over MCP) reads off them, plus one
anti-reference: scorsese's own earlier attempt at one of them. Read it
**before** writing a song recipe, to pick the reference for the cue and know
what its mix looks like, and **after** a bake, to read the bake report against
it.

**The numbers are a floor to clear, never a definition of success.** They say
how loud, how squeezed, where the energy sits and how wide. They say nothing
about melody, groove or timbre, which is where a score actually succeeds or
fails (see [what the numbers cannot capture](#what-the-numbers-cannot-capture)).
[`recipes.md`](recipes.md#how-a-bake-came-out) holds the general rule: *a metric
treated as an ear produces music that optimises the number and gets worse.*

The recordings are not in the repository and never will be: they are
commercial. Only their numbers are here. Where the files live on the
maintainer's machine is recorded on issue #411.

## Which reference for which cue

| reference | reach for it when the cue is… | simplify? |
| --- | --- | --- |
| **Billie Jean** (Michael Jackson) | a groove: steady, mid-tempo, carried by bass and drums, under talk or an explainer | no: the bassline, the hat pattern and the snare *are* the song |
| **Fuego** (Alok) | a build to a payoff: a quiet, wide build-up, then a loud, bass-heavy drop | no |
| **Techno Syndrome** (Mortal Kombat) | drive and relentless energy, four on the floor, at full tilt from start to finish | yes |
| **BFG Division** (DOOM, 2016) | heavy and aggressive: a wall of low end, deliberately compressed | yes |
| **Brutal Doom main theme** | the same family as BFG Division: the original DOOM theme as heavy metal, the steadiest wall in the set | yes |

**Simplify** is the maintainer's rule for the two dense references: aim at the
main instruments and the energy, and do not chase every detail. It is not a
licence to thin out the other two, where the few main parts are the whole song.

What Fuego is *for* was not stated by the maintainer; the build-and-drop
reading above comes from its numbers. Nobody here has listened to the file
either (its tags name an uploader, not the track), so treat it as provisional
until the maintainer confirms it is the right recording.

## The numbers

Measured with `scorsese level` on 2026-10-07. **Body** is the main stretch of
the track: the 8-second windows after the intro and before the fade. The ranges
are the spread across those windows, not a single average, because a real mix
moves.

| reference | length | whole-file mean | body mean | body crest | body low / mid / high % | body corr |
| --- | --- | --- | --- | --- | --- | --- |
| Billie Jean | 4:54 | −14.1 | −12.5 to −16.5 | 14–18.5 (mostly 15–16) | 56–86 / 11–40 / 2–5 | +0.6 to +0.97 |
| Fuego, drops | 4:05 | −11.3 | −8 to −10 | 9.5–11.5 | 74–85 / 14–25 / 1–2 | +0.75 to +0.95 |
| Fuego, breaks | | | −15.5 to −19 | 13–17.5 | 9–39 / 61–89 / 0–5 | +0.2 to +0.5 |
| Techno Syndrome | 4:46 | **−7.0** | −5.5 to −7.5, brief dips to −10 | **8–10** | 50–66 / 32–45 / 2–3 | mono (file) |
| BFG Division | 9:03 | −12.7 | −10.5 to −14.5 | 11.5–15 | 54–82 / 16–43 / 1–4, a few windows down to 28% low | +0.45 to +0.6 |
| Brutal Doom | 3:18 | −12.2 | −11 to −13 | 11.5–13.5 | 73–85 / 14–26 / 1–2 | +0.5 to +0.8 |

Means and peaks are dBFS, crest is dB. Every one of these files peaks over full
scale, and that is a property of the file rather than a target
(see [the files themselves](#the-files-themselves)).

**The mean a bake can reach is about −1 minus its crest.** zimmer's limiter
holds the true peak at −1 dBTP ([recipes.md](recipes.md#effects-recipe-patch)),
and crest is peak minus mean, so a bake with Billie Jean's crest of 15.5 lands
near −16.5, not −14. Chase the **crest** and the **bands**; the mean follows
from the crest, and how loud the music sits in the video is the clip's volume
and `duck_music`, not the bake. That makes the reachable body means:

| reference | crest to aim for | mean that comes with it |
| --- | --- | --- |
| Billie Jean | ~15.5 | ~−16.5 |
| Fuego drop | ~10 | ~−11 |
| Techno Syndrome | ~9 | ~−10 |
| BFG Division | ~12.5 | ~−13.5 |
| Brutal Doom | ~12 | ~−13 |

### What the shapes say

Read the per-section rows, not only the summary line: how the track *opens*,
and how much it moves, is half of what each reference is for.

- **Billie Jean starts at full strength.** Its first 8 seconds are −16.2 with a
  crest of 17.9 and 79% low, only about 2 dB under the body: the groove is
  there from the first bar, not built up to. The first minute and a half is
  bass and drums, nearly mono (corr +0.95) and 75–86% low. After about 1:30 the
  mid share rises to 30–40% and the width opens to +0.6–0.7. The level barely
  moves the whole way through: every section sits between −12.5 and −16.5,
  most within a dB of −14, until the fade.
- **Fuego is two states, about 8 dB apart.** The breaks are quiet, mid-heavy
  and wide (−16 to −19, low as little as 9%, corr down to +0.2). The drops are
  loud, bass-heavy and narrow (−8, low around 80%, corr +0.9, crest closing to
  10). The handover takes one 8-second window. The intro starts at −27 with a
  crest of 23 and climbs to the break level by 0:24. The contrast is the
  point: a build that is as loud as its drop has nothing to pay off.
- **Techno Syndrome is the loudest and most squeezed of the set.** It opens on
  a mid-only riff (−12.6, 85% mid), is at full level by 0:24 and stays within
  about 2 dB of −6 until the outro. Its brief breaks dip to −9 or −10 for one
  window and come straight back. Its mid share (around 40%) is the highest of
  the five, which is the riff.
- **BFG Division alternates two walls.** Its opening 32 seconds is almost
  nothing but low end (96–97% low, 3% mid) at −11 with a crest of 10.5. This
  is the loudest, flattest part of the track, so the intro is not a build. The
  full-band sections that follow are 55–80% low and a little more dynamic (crest
  12–14). Long breakdowns fall to −17…−24 and go very wide (corr to +0.1)
  before the low wall returns.
- **Brutal Doom barely moves.** After an intro of −15.6, it holds between −11
  and −13 with a crest of 12–13 and 75–85% low for three minutes. If a cue
  needs the same pressure all the way through, this is its shape.

## The anti-reference: scorsese's own Billie Jean

The bake of an earlier attempt at Billie Jean, measured 2026-08-27 and described
by the maintainer as *"loosely similar in the start and then got lost"*. It is
the most useful row on this page, because the target and the failure are the
same piece of music.

| | scorsese's attempt | Billie Jean |
| --- | --- | --- |
| whole-file mean | −20.8 | −14.1 |
| whole-file crest | 20.6 | 16.8 |
| body crest | 16–19 | 15–16 |
| low / mid / high, whole file | 52 / 40 / 8 | 70 / 26 / 4 |
| first 8 s | −25.3, about 5 dB under its body, **22% high** | −16.2, about 2 dB under its body, 79% low |
| true peak | 0.0, clipping | (file over full scale, see below) |
| width | not measured (predates `corr`) | +0.81 |

What it says, in order of size:

- **The balance is the biggest miss.** The low end is 18 points short and the
  mid 14 points over, so the attempt is lighter and boxier than a record whose
  identity is its bassline.
- **The opening is wrong in kind, not just level.** The record opens on bass and
  drums at nearly full level, while the attempt opened quiet and hat-heavy. The
  first 8 seconds are a fair test on their own.
- **Crest is closer than first thought.** Section by section the attempt is
  only 1–3 dB more dynamic than the record. Billie Jean is a dynamic mix too, so
  aiming its crest at Techno Syndrome's would be a mistake. Over the whole file
  the gap is about 4 dB, which glue on the sum
  ([recipes.md](recipes.md#where-an-effect-goes)) is for.
- **The loudness gap is mostly the crest and the ceiling.** At −1 dBTP and the
  record's crest, a bake lands near −17.8. The rest of the 6.7 dB is the master
  running over full scale, which zimmer will not copy.
- **It clipped on true peak.** That was #437, and the limiter now holds −1 dBTP.

A re-bake that is better by these numbers has its low share near 70%, its first
section within a few dB of its body and mostly low end, a body crest around 15–16
and no true-peak clip. Then it needs listening to, because that is still only the
floor.

## The files themselves

Four things about the recordings change how far a number can be trusted:

- **Four of the five are 32 kHz, 128 kbps MP3s** (Fuego is 48 kHz, 256 kbps). A
  32 kHz file has nothing above 16 kHz and a low-bitrate encoder trims more, so
  their **high band reads low**. Even Fuego, the clean file, reads 1–2% high in
  its drops, so a few percent really is what these masters carry. A body above
  about 5% high is brighter than every reference here. Below that, the high
  share cannot be compared.
- **The Techno Syndrome file is mono**: corr +1.00 in every window. Use its
  loudness, crest and band split, never its width.
- **Every file peaks over 0 dBFS** (true peaks +1 to +3.6). Decoding a loud MP3
  master to float keeps its inter-sample overs, so this is normal for the files
  and not a fault in the recordings. The *clipping* label `level` prints is
  about the file, not something zimmer should aim at. Those overs also add a
  fraction of a dB to each crest, which is well inside the ranges above.
- **The two DOOM tracks are game-soundtrack masters**, mixed loud and
  compressed on purpose. Their low crest is the point of them, not a defect.

## What the numbers cannot capture

`level` measures the **mix**: how loud, how squeezed, which of three bands the
energy is in, how wide. Everything else is outside it:

- **Melody, harmony and key.** Two scores can share every figure on this page
  and have nothing musical in common.
- **Groove and feel.** Billie Jean's identity is where its bass and hat notes
  land against the beat, and timing is not a level.
- **Timbre inside a band.** A distorted guitar and a detuned saw can both read
  `low 78% mid 20%`. The verdict that ended #251, *teclado* (it sounds like a
  home keyboard), is about timbre, and no row here would have caught it.
- **Arrangement.** Which instrument plays when shows up only as level and band
  share moving between sections. The bake report's per-track rows and grid say
  more, but only about zimmer's own bakes, never about a reference.
- **The voice.** Billie Jean's mid share and Techno Syndrome's shout include a
  vocal, and zimmer makes none. A score that reaches the reference's mid share
  has filled that space with instruments, which may or may not be the right
  call for a cue that sits under narration anyway.

So the order is: pick the reference for the cue, write for its main instruments
and its shape, bake, compare the rows here, and fix what the numbers show is
**wrong**. Then someone listens. Matching every figure is not the goal, and a
score tuned until it does will usually be worse for it.
