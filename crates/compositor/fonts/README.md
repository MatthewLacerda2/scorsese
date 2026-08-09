# The fonts scorsese ships

Two faces, **one variable file each**, compiled into `scorsese-compositor` with
`include_bytes!`. They are the `sans` and `serif` a `text` asset's `style` can
name; a project may name a font file of its own instead.

| File | Face | `wght` axis | Bytes |
| --- | --- | --- | --- |
| `Inter-V.ttf` | Inter | 100 – 900, default 400 | 805,396 |
| `SourceSerif4Variable-Roman.ttf` | Source Serif 4 | 200 – 900, default 400 | 1,204,208 |

Inter from **Inter 3.19**, unmodified, as `Inter Desktop/Inter-V.ttf`:
<https://github.com/rsms/inter>

Source Serif 4 from **4.005**, unmodified, as `VAR/SourceSerif4Variable-Roman.ttf`
on the `release` branch: <https://github.com/adobe-fonts/source-serif>

```
sha256  69b1af837d101ab90b003d61d4ccc5e5320a6dcaefeb69906fa31c01a06e5837  Inter-V.ttf
sha256  14d360ee1b76655da9276628b229e11671bc1f5d1083636144db6677d452cf55  SourceSerif4Variable-Roman.ttf
```

## Which axes each file carries, and what is done with them

Only `wght` is read. Every other axis is left where the file's own `fvar` puts
it, so which axes a file has is a thing to check before choosing it rather than
after seeing a golden diff.

| File | Axes | Left at |
| --- | --- | --- |
| `Inter-V.ttf` | `wght` 100–900, `slnt` −10–0 | `slnt` 0, which is upright |
| `SourceSerif4Variable-Roman.ttf` | `wght` 200–900, `opsz` 8–60 | `opsz` 20 |

**Inter 3.19 rather than 4.x, and that is a deliberate choice.** Inter 4's
`InterVariable.ttf` carries an `opsz` axis running 14–32 whose default is **14**
— a design tuned for small text — so every title scorsese drew would be set at
the caption design and nothing would say so. 3.19's `Inter-V.ttf` has no `opsz`
at all, so there is no axis sitting at a value nobody chose. Its `slnt` default
of 0 is upright, which is the only slant this project has a field for.

**Source Serif 4's `opsz` sits at 20, and there is no build without it.** That
is the text-size design, and a title at display size is therefore drawn slightly
off-design — heavier in the stems and tighter in the spacing than the face's own
display master. It is recorded here rather than discovered later. Optical sizing
is a real feature with a real rule (which pixel size maps to which `opsz`), and
it is not this.

## Why they are committed

A system-font lookup resolves to a different file on Linux, macOS and Windows.
Text drawn from one would not match text drawn from another, so a golden
reference blessed on one machine would fail on every other and the pixel gate
would become noise. Deterministic text means a font we ship.

That makes these two files part of the pixel gate rather than a detail beneath
it, and [`docs/golden-renders.md`](../../../docs/golden-renders.md) says so on
its own page — the faces sit upstream of the gate the way the decoder does, and
neither may be let go of quietly. The rule that follows lives there with the
other re-blessing rules: **swapping, subsetting or system-resolving these faces
re-blesses every fixture that draws text**, and is legitimate only as a
deliberate visual change explained in the pull request. The desktop app's own
reference images are in the same position, for the same reason: its preview
draws a compositor frame.

## Licence

SIL Open Font License 1.1 for both — the full texts are in `Inter-OFL.txt` and
`SourceSerif4-OFL.txt`, exactly as they shipped with the fonts.

The two conditions that matter here: the files are redistributed **unmodified**
and under their own names, and both families carry Reserved Font Names —
`Inter` and `Source` — so a modified copy would have to be renamed. Neither is a
constraint on scorsese's own licence; the OFL covers the font files and nothing
else. Subsetting them to save space would count as modifying them, which is why
they are committed whole.

## One file each, rather than one weight each

The size argument for shipping a single weight per family died when `weight`
arrived at schema v11. A variable face is **one file covering the whole range**,
which is the trade `docs/project-format.md` already makes the case for on a
project's own fonts — so it is the trade the shipped ones make too.

It costs 1.2 MB: 1,963 KB of variable faces against the 785 KB of static ones
they replace, for every weight from 100 (or 200) to 900 instead of Regular. A
bold title used to require leaving the program to find a variable font, check
its licence and copy it into `assets/`, for the most ordinary thing anybody asks
of text.

`sans` and `serif` **default to weight 400 when none is given**, and that is a
rule rather than an exception — its reasoning is in `docs/project-format.md`
beside the general one it appears to contradict.

Italic is the half that still stands: there is no `slant` field, so an italic
file would be a face nothing could ask for. Neither family's monospace is
shipped, for the same reason Liberation Mono was not.

## What was here before

Liberation Sans and Liberation Serif, one static weight each, chosen because
they are metric-compatible with Arial and Times New Roman. **That
compatibility is retired rather than lost**: it matters for document
interchange, where a line has to break in the same place as it did in Word, and
nothing in a video editor depends on it. Seven golden fixtures were re-blessed
when the faces changed — the six that draw a `text` asset plus `slugs`, whose
cards are drawn in `sans` too. `weight` was not among them, because it sets its
title in a font the project itself carries, which is the point of that fixture.
