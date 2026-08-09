# The fonts scorsese ships

Eight families, compiled into `scorsese-compositor` with `include_bytes!`.
These are the names a `text` asset's `style` can write; a project may name a
font file of its own instead.

**The list itself lives in `src/text/font/shipped.rs`**, beside the
`include_bytes!` that make each one real. This file is the provenance: where
each came from, what it weighs, and what its licence asks for.

| name | family | weights | shape |
| --- | --- | --- | --- |
| `inter` — alias `sans` | Inter | 100 – 900 | variable |
| `source-serif` — alias `serif` | Source Serif 4 | 200 – 900 | variable |
| `liberation-sans` | Liberation Sans | 400, 700 | **drawn** |
| `liberation-serif` | Liberation Serif | 400, 700 | **drawn** |
| `montserrat` | Montserrat | 100 – 900 | variable |
| `lora` | Lora | 400 – 700 | variable |
| `playfair-display` | Playfair Display | 400 – 900 | variable |
| `jetbrains-mono` | JetBrains Mono | 100 – 800 | variable |

## Variable and drawn, and why the difference is in the code

A **variable** family is one file whose `wght` axis covers a range, so any
weight inside it is a real position on that axis — including ones the designer
never drew, which is what `500` between Regular and Medium means.

A **drawn** family is several files the designer actually drew, and the only
weights it has are the ones in the table. **Liberation is the reason this
distinction exists.** There is no variable build of it anywhere — upstream ships
Regular, Bold, Italic and BoldItalic as four separate files — so a model where a
face is one file could not express it at all. That is why the Arial and Times
look-alikes were unshippable the moment `weight` arrived, and it is what #278
fixed.

**A drawn family refuses a weight it was not drawn at**, naming the ones it has.
`liberation-sans` at `600` is an error, not a quiet 700. Snapping would be the
same silent substitution the variable rules already refuse when a weight falls
off the end of an axis.

## Provenance

Inter from **3.19**, as `Inter Desktop/Inter-V.ttf`:
<https://github.com/rsms/inter>

Source Serif 4 from **4.005**, as `VAR/SourceSerif4Variable-Roman.ttf` on the
`release` branch: <https://github.com/adobe-fonts/source-serif>

Liberation from **liberation-fonts 2.1.5**:
<https://github.com/liberationfonts/liberation-fonts>

Montserrat, Lora, Playfair Display and JetBrains Mono from **google/fonts**,
each the `[wght]` variable file under `ofl/<family>/`:
<https://github.com/google/fonts>

All unmodified. 4,926 KB of faces in total.

```
sha256  69b1af837d101ab90b003d61d4ccc5e5320a6dcaefeb69906fa31c01a06e5837  Inter-V.ttf
sha256  48715a42ec242c21e9f02692891e147d022299a52e48d5e413e1a942193ffeda  JetBrainsMono[wght].ttf
sha256  788abee4c806d660e8aee46689dd8540cd4bb98da03dcc9d171ce3efd99a9173  LiberationSans-Bold.ttf
sha256  76d04c18ea243f426b7de1f3ad208e927008f961dc5945e5aad352d0dfde8ee8  LiberationSans-Regular.ttf
sha256  d754ba427cfe0bca54ae052384baa8f842da5bd6550ad4da024ac441e7a7d5ce  LiberationSerif-Bold.ttf
sha256  058ea80864aef09a23f45cbec2bb5400bc3dfbdea01c3f10538a21fcb497fb74  LiberationSerif-Regular.ttf
sha256  822a6621ccbe8d97d20ac88c1c41f5615c9c2c202eaa75f272cd452aac6475a7  Lora[wght].ttf
sha256  0f7b311b2f3279e4eef9b2f968bcdbab6e28f4daeb1f049f4f278a902bcd82f7  Montserrat[wght].ttf
sha256  c40f2293766a503bc70cce9e512ef844a4ccb7cbcde792fe2ea31d191917d8d6  PlayfairDisplay[wght].ttf
sha256  14d360ee1b76655da9276628b229e11671bc1f5d1083636144db6677d452cf55  SourceSerif4Variable-Roman.ttf
```

The two Liberation hashes for Regular are the same bytes this repository
shipped before #267 swapped them out, which is worth noting: they came back
rather than being fetched anew.

## Which axes each file carries, and what is left alone

Only `wght` is read. Every other axis stays where the file's own `fvar` puts it,
so which axes a file has is something to check **before** choosing it.

| file | other axes | left at |
| --- | --- | --- |
| `Inter-V.ttf` | `slnt` −10–0 | `0`, upright |
| `SourceSerif4Variable-Roman.ttf` | `opsz` 8–60 | `20`, the text design |
| everything else | none | — |

**Inter 3.19 rather than 4.x, deliberately.** Inter 4's `InterVariable.ttf`
carries an `opsz` axis running 14–32 defaulting to **14** — a design tuned for
small text — so every title would be set at the caption design with nothing
saying so. 3.19 has no `opsz` at all.

**Source Serif 4's `opsz` sits at 20 and there is no build without it.** A title
at display size is therefore drawn slightly off-design. Recorded here rather
than discovered later. Optical sizing is a real feature with a real rule and is
not this.

One trap worth naming: **Montserrat's own `fvar` default is 100** — Thin. It
does no harm, because a shipped family with no weight named is set at 400 rather
than at the file's default, which is exactly the rule that exists for it.

## Why they are committed

A system-font lookup resolves to a different file on Linux, macOS and Windows.
Text drawn from one would not match text drawn from another, so a golden
reference blessed on one machine would fail on every other and the pixel gate
would become noise. Deterministic text means a font we ship.

That makes these files part of the pixel gate rather than a detail beneath it,
and [`docs/golden-renders.md`](../../../docs/golden-renders.md) says so on its
own page — the faces sit upstream of the gate the way the decoder does, and
neither may be let go of quietly. The rule that follows lives there with the
other re-blessing rules: **swapping, subsetting or system-resolving a face
re-blesses every fixture that draws text**, and is legitimate only as a
deliberate visual change explained in the pull request. The desktop app's own
reference images are in the same position, because its preview draws a
compositor frame.

Adding a *new* name moves nothing, which is what makes the list cheap to grow:
every existing fixture names `sans`, `serif`, or a font its own project carries.

## Licence

SIL Open Font License 1.1 for all eight — full texts beside the files, exactly
as each shipped: `Inter-OFL.txt`, `SourceSerif4-OFL.txt`,
`Liberation-LICENSE.txt`, `Montserrat-OFL.txt`, `Lora-OFL.txt`,
`PlayfairDisplay-OFL.txt`, `JetBrainsMono-OFL.txt`.

The two conditions that matter: every file is redistributed **unmodified** and
under its own name, and most of these families carry Reserved Font Names —
`Inter`, `Source`, `Liberation`, `Montserrat`, `Lora`, `Playfair`,
`JetBrains` — so a modified copy would have to be renamed. Neither is a
constraint on scorsese's own licence; the OFL covers the font files and nothing
else. **Subsetting counts as modifying**, which is why they are committed whole.

**Arial and Times New Roman themselves can never ship.** They are Monotype's,
licensed through Microsoft, and cannot be committed to a public repository. That
is the entire reason Liberation exists: metric-compatible open substitutes, the
same advance widths, the same look to anyone who is not a typographer. If you
want the Arial look, `liberation-sans` is it.

Italic is the half still missing: there is no `slant` field, so an italic file
would be a face nothing could ask for. Liberation ships Italic and BoldItalic
upstream and Inter and Source Serif both have separate italic files, so the
files are there whenever the field is —
[#279](https://github.com/MatthewLacerda2/scorsese/issues/279).
