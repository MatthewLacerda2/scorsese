# Three fonts, for the tests only

Neither is shipped. Neither is compiled into the crate, neither is a face
`style.font` can name, and nothing under `src/` reads either. They are test
data, and they are here because the rules they test cannot be tested without
real files.

| File | Face | Axes |
| --- | --- | --- |
| `Manrope[wght].ttf` | Manrope | `wght` 200 – 800, **defaulting to 200** |
| `SpaceMono-Regular.ttf` | Space Mono | none — **static** |
| `SourceSerif4-Regular.otf` | Source Serif 4 | none — **static**, **CFF outlines** |

The first two from **google/fonts**, unmodified:
<https://github.com/google/fonts/tree/main/ofl/manrope> and
<https://github.com/google/fonts/tree/main/ofl/spacemono>

```
sha256  d0639be45d0af36e798172419d7bd173c4bd4f29e2b76cbb69db1d11bf8b0a40  Manrope[wght].ttf
sha256  95837e182baeeada83368f7748db28357f0a1b75c6b84ff7065b5edf933c8e18  SpaceMono-Regular.ttf
```

## Why this one

Because its default instance is 200 — ExtraLight — and that is the whole bug.
A build that reads a variable file and draws at `LocationRef::default()` sets a
title card in something close to hairline while the document that produced it
looks entirely correct, and no error is ever raised. Manrope is the shortest
proof of that available: point at it, say nothing about weight, and the old
behaviour was silently wrong.

A font whose default happened to be 400 would test the mechanism and miss the
point, because every failure mode would still look approximately right.

The 200–800 range is the second reason. It stops short of both ends of
OpenType's own 1–1000, so a weight of 900 is a number the *format* allows and
this *file* does not — which is exactly the split between what validation can
check from the document and what only opening the file can answer.

## Why the static one

Because "a weight beside a file that has only one" is a rule too, and since
#267 **both shipped faces are variable** — so neither can stand in for a static
file any more. Space Mono is the smallest thing that can: no `fvar` at all, 97
KB, and a family nothing else here uses, so a test failing on it cannot be
confused with a test failing on a face scorsese draws with.

## Why the CFF one

Because every other font in the repository — both above, all the shipped
faces, the golden fixture's — is TrueType, whose outlines are quadratic only,
so the pen's cubic segment (`curve_to` in `src/text/draw.rs`) was never drawn by
any test (#671). It is not dead code: `Font::from_bytes` takes a user's own
font, and a great many of those are `.otf` files with CFF outlines. Without one
here, a drawing that dropped every cubic would leave the suite green while such
a font set letters with no curves in them.

Source Serif 4 because it is already a shipped face (as variable TrueType), so
its licence is one this repository already carries, and Adobe publishes it as a
CFF build. One static weight is all a test needs. From **adobe-fonts/source-serif**,
the `release` branch at `80d3f8894c09c937bebfa9011247d2e1c79fd6f4`, unmodified:
<https://github.com/adobe-fonts/source-serif/tree/release/OTF>

```
sha256  edf160d0d584deee8a3bb2c3371b2a7624ca63580fbe02c57c1f4c91e84d8787  SourceSerif4-Regular.otf
```

## Why it is committed rather than fetched

Same reason the shipped faces are: a test that needs a network is a bug,
and a font resolved from the system is a different file on every machine.

Manrope is a second copy of the file `crates/golden/fixtures/weight/assets/` also
carries, and that is deliberate rather than an oversight. A golden fixture is a
**real project directory**, so the font in it is a project's own font — the
thing under test there is that a document can carry a face and be rendered from
it. This copy is the compositor's test data and belongs to the compositor.
Reaching across into the golden crate's fixtures for it would invert a stated
boundary: nothing depends on `crates/golden`.

Source Serif 4 is not a second copy of the shipped serif: that one is a
variable TrueType build and this one a static CFF build, and the difference in
outline format is the entire reason this file is here.

## Licence

SIL Open Font License 1.1 for all three — the full texts are in
`Manrope-OFL.txt`, `SpaceMono-OFL.txt` and `SourceSerif4-OFL.txt` (Adobe's
`LICENSE.md`), exactly as they ship with the fonts. Every file is redistributed
**unmodified** and under its own name; `Manrope` and `Source` are Reserved
Font Names, so a subset copy — which is what "modified" would mean here — would
have to be renamed. Keeping them whole is cheaper than being careful about that,
and it is the same call the shipped faces made.
