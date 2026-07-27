# The fonts scorsese ships

Two faces, one weight each, compiled into `scorsese-compositor` with
`include_bytes!`. They are the `sans` and `serif` a `text` asset's `style` can
name; a project may name a font file of its own instead.

| File | Face | Metric-compatible with |
| --- | --- | --- |
| `LiberationSans-Regular.ttf` | Liberation Sans | Arial |
| `LiberationSerif-Regular.ttf` | Liberation Serif | Times New Roman |

Both from **liberation-fonts 2.1.5**, unmodified:
<https://github.com/liberationfonts/liberation-fonts>

```
sha256  76d04c18ea243f426b7de1f3ad208e927008f961dc5945e5aad352d0dfde8ee8  LiberationSans-Regular.ttf
sha256  058ea80864aef09a23f45cbec2bb5400bc3dfbdea01c3f10538a21fcb497fb74  LiberationSerif-Regular.ttf
```

## Why they are committed

A system-font lookup resolves to a different file on Linux, macOS and Windows.
Text drawn from one would not match text drawn from another, so a golden
reference blessed on one machine would fail on every other and the pixel gate
would become noise. Deterministic text means a font we ship.

Arial and Times New Roman themselves are proprietary — Monotype, licensed
through Microsoft — and cannot be committed to a public repository. Liberation
Sans and Liberation Serif are the well-trodden open substitutes: metric
compatible, so a line set in one breaks in the same places as the same line set
in the other.

## Licence

SIL Open Font License 1.1 — the full text is in `OFL.txt`, exactly as it
shipped with the fonts.

The two conditions that matter here: the files are redistributed **unmodified**
and under their own names, and `Liberation` is a Reserved Font Name, so a
modified copy would have to be renamed. Neither is a constraint on scorsese's
own licence — the OFL covers the font files and nothing else. Subsetting them
to save space would count as modifying them, which is why they are committed
whole.

One regular weight of each rather than a family: bold and italic are a real
feature with a real vocabulary (`weight`, `slant`) and shipping four more files
against a field nothing can select would be a megabyte pretending to be a
choice. Liberation Mono is deliberately left out until something asks for it.
