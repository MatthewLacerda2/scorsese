# The fonts scorsese ships

Eight families, **upright and italic**, compiled into `scorsese-compositor`
with `include_bytes!`. These are the names a `text` asset's `style` can write; a
project may name a font file of its own instead.

A ninth face is compiled in beside them and is **not** in that list, because
nothing names it: Noto Color Emoji, the **fallback**. *The face nobody names*
below is the whole of it.

**The list itself lives in `src/text/font/shipped.rs`**, beside the
`include_bytes!` that make each one real. This file is the provenance: where
each came from, what it weighs, and what its licence asks for.

| name | family | weights | shape | italic |
| --- | --- | --- | --- | --- |
| `inter` — alias `sans` | Inter | 100 – 900 | variable | drawn, separate file |
| `source-serif` — alias `serif` | Source Serif 4 | 200 – 900 | variable | drawn, separate file |
| `liberation-sans` | Liberation Sans | 400, 700 | **drawn** | Italic + BoldItalic |
| `liberation-serif` | Liberation Serif | 400, 700 | **drawn** | Italic + BoldItalic |
| `montserrat` | Montserrat | 100 – 900 | variable | drawn, separate file |
| `lora` | Lora | 400 – 700 | variable | drawn, separate file |
| `playfair-display` | Playfair Display | 400 – 900 | variable | drawn, separate file |
| `jetbrains-mono` | JetBrains Mono | 100 – 800 | variable | drawn, separate file |

**Every family has a real italic, and none of them is an oblique.** An italic is
a different drawing — a redrawn `a`, `f` and `g` rather than the upright leaned
over — so it is a second set of files keyed by weight exactly like the first.
Inter is the case that proves the point: its `Inter-V.ttf` carries a `slnt` axis
that would produce a perfectly good oblique, and `italic: true` ignores it in
favour of `Inter-Italic.ttf`, where the letters are actually different.

## The face nobody names

**Noto Color Emoji, and it is reached by coverage rather than by name.** A
document that writes `Ship it 🔥` names `sans` like any other caption; the fire
is drawn because Inter has no glyph for `U+1F525` and the next face in the chain
does. There is no `style` field that selects it, none that turns it off, and it
never appears in the list above — a face a document could name is a face a
document would have to know about, and the point of a fallback is that nobody
does.

| file | family | covers | shape |
| --- | --- | --- | --- |
| `Noto-COLRv1.ttf` | Noto Color Emoji | 1,499 codepoints, and the sequences built from them | **COLRv1**, layered vector paints |

**The vector build, not the bitmap one, and that is a decision rather than a
preference.** Upstream ships the same emoji twice: `NotoColorEmoji.ttf` carries
CBDT bitmap strikes at 136 px, and `Noto-COLRv1.ttf` carries COLRv1 paint
graphs — outlines, gradients and compositing modes, resolution-free like every
other glyph in this directory. A title card is where an emoji is *large*, so a
136-pixel strike blown up to fill a quarter of a 4K frame is exactly the failure
this build avoids. It is the smaller file as well: 4,875 KB against 10,423 KB.

The flags are in it. `Noto-COLRv1-noflags.ttf` is 2,922 KB and would have saved
1,953 KB by dropping every regional-indicator pair, which is to say by silently
not drawing 🇧🇷 — the same silent drop the fallback exists to end, moved one step
along. Paying two megabytes not to reintroduce it is the whole trade.

**No skin tone or joined sequence is a special case here.** 👍🏽 is `U+1F44D`
followed by a modifier and 👨‍👩‍👧 is three people joined by zero-width joiners;
both are ligatures the font's own `GSUB` resolves, so they arrive as one glyph
for the same reason `fi` does. What makes that work is not the file, it is that
the whole sequence is shaped against one face — see
`src/text/runs.rs`.

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

Noto Color Emoji from **noto-emoji v2.051**, as `fonts/Noto-COLRv1.ttf`:
<https://github.com/googlefonts/noto-emoji>

Italics from the same releases: Inter's from `Inter Variable/Single axis/
Inter-italic.ttf`, Source Serif's from `VAR/SourceSerif4Variable-Italic.ttf`,
Liberation's Italic and BoldItalic from the same tarball, and the four Google
families' from `<family>-Italic[wght].ttf` beside their uprights.

All unmodified. **13,857 KB of faces in total**, across 21 files — italic
roughly doubles the eight nameable families, and it buys the third thing anybody
does to text; the emoji face is 4,875 KB of that on its own, which is what a
complete colour set costs.

```
sha256  ae7f78865f4e4c77c50f1ee0fbe603665e48539c3da815c04d99ffec8afc3c6d  Inter-Italic.ttf
sha256  69b1af837d101ab90b003d61d4ccc5e5320a6dcaefeb69906fa31c01a06e5837  Inter-V.ttf
sha256  85ae2a5cd3f56baf1ce1c21a851322c58e3d8fbe8e8ad4a4d090a820dd7fe558  JetBrainsMono-Italic[wght].ttf
sha256  48715a42ec242c21e9f02692891e147d022299a52e48d5e413e1a942193ffeda  JetBrainsMono[wght].ttf
sha256  698da70fc191cc5f33ad4d6d3fe830fe4624b898ea2e3169955928b7c491f1ee  LiberationSans-BoldItalic.ttf
sha256  788abee4c806d660e8aee46689dd8540cd4bb98da03dcc9d171ce3efd99a9173  LiberationSans-Bold.ttf
sha256  e5bae5c4cde31f22142753855f4f8fb86da6ff39955ed3c0a11248b0d16948b0  LiberationSans-Italic.ttf
sha256  76d04c18ea243f426b7de1f3ad208e927008f961dc5945e5aad352d0dfde8ee8  LiberationSans-Regular.ttf
sha256  f17db8af71e24d2066b587546021d4f0b296be389512b658dec3c09affeb11a7  LiberationSerif-BoldItalic.ttf
sha256  d754ba427cfe0bca54ae052384baa8f842da5bd6550ad4da024ac441e7a7d5ce  LiberationSerif-Bold.ttf
sha256  0e3dea9f8d613e006ccfa62201f33e265d19167bd0907725c3e145368b04fc2e  LiberationSerif-Italic.ttf
sha256  058ea80864aef09a23f45cbec2bb5400bc3dfbdea01c3f10538a21fcb497fb74  LiberationSerif-Regular.ttf
sha256  22d8d8854b53807aa664ca34f2031a9ed57a1d0dea296b8b96cdd3aad937a2b3  Lora-Italic[wght].ttf
sha256  822a6621ccbe8d97d20ac88c1c41f5615c9c2c202eaa75f272cd452aac6475a7  Lora[wght].ttf
sha256  51607f316bc020e59f03cbf51543eecffbea501c0b31d73e5b82927c5cca442c  Montserrat-Italic[wght].ttf
sha256  0f7b311b2f3279e4eef9b2f968bcdbab6e28f4daeb1f049f4f278a902bcd82f7  Montserrat[wght].ttf
sha256  0ae57fe58645638523ba35f388d93739d292539a9acb84df5700c81b1e1a28d2  Noto-COLRv1.ttf
sha256  a5e26dc5e2e77fb2803a0bf02fd4f81ee136ec8dea863ccdb0c59a263b21378b  PlayfairDisplay-Italic[wght].ttf
sha256  c40f2293766a503bc70cce9e512ef844a4ccb7cbcde792fe2ea31d191917d8d6  PlayfairDisplay[wght].ttf
sha256  6a059a64838978d54e8fab71ed86b0d82e948c0e12b2664d0c15166326dcff82  SourceSerif4Variable-Italic.ttf
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
| `Noto-COLRv1.ttf` | none — no `fvar` at all | — |
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

SIL Open Font License 1.1 for all nine — full texts beside the files, exactly
as each shipped: `Inter-OFL.txt`, `SourceSerif4-OFL.txt`,
`Liberation-LICENSE.txt`, `Montserrat-OFL.txt`, `Lora-OFL.txt`,
`PlayfairDisplay-OFL.txt`, `JetBrainsMono-OFL.txt`,
`NotoColorEmoji-OFL.txt`.

The two conditions that matter: every file is redistributed **unmodified** and
under its own name, and most of these families carry Reserved Font Names —
`Inter`, `Source`, `Liberation`, `Montserrat`, `Lora`, `Playfair`,
`JetBrains` — so a modified copy would have to be renamed. Noto Color Emoji
reserves none, which changes nothing here: it is redistributed whole and
unmodified like the rest. Neither is a
constraint on scorsese's own licence; the OFL covers the font files and nothing
else. **Subsetting counts as modifying**, which is why they are committed whole.

**Arial and Times New Roman themselves can never ship.** They are Monotype's,
licensed through Microsoft, and cannot be committed to a public repository. That
is the entire reason Liberation exists: metric-compatible open substitutes, the
same advance widths, the same look to anyone who is not a typographer. If you
want the Arial look, `liberation-sans` is it.

What is still missing is a **condensed** or a **wide**, which would be a `wdth`
axis and a third field. Nobody has asked, and the same rule would apply: a real
width is a drawing, not a horizontal scale.
