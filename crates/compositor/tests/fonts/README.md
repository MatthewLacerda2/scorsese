# A variable font, for the tests only

`Manrope[wght].ttf` is **not** shipped. It is not compiled into the crate, it
is not one of the faces `style.font` can name, and nothing under `src/` reads
it. It is test data, and it is here because the rule it tests cannot be tested
without a real one.

| File | Face | Axis |
| --- | --- | --- |
| `Manrope[wght].ttf` | Manrope | `wght` 200 – 800, **defaulting to 200** |

From **google/fonts**, unmodified:
<https://github.com/google/fonts/tree/main/ofl/manrope>

```
sha256  d0639be45d0af36e798172419d7bd173c4bd4f29e2b76cbb69db1d11bf8b0a40  Manrope[wght].ttf
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

## Why it is committed rather than fetched

Same reason the two shipped faces are: a test that needs a network is a bug,
and a font resolved from the system is a different file on every machine.

It is a second copy of the file `crates/golden/fixtures/weight/assets/` also
carries, and that is deliberate rather than an oversight. A golden fixture is a
**real project directory**, so the font in it is a project's own font — the
thing under test there is that a document can carry a face and be rendered from
it. This copy is the compositor's test data and belongs to the compositor.
Reaching across into the golden crate's fixtures for it would invert a stated
boundary: nothing depends on `crates/golden`.

## Licence

SIL Open Font License 1.1 — the full text is in `OFL.txt`, exactly as it ships
with the font. The file is redistributed **unmodified** and under its own name;
`Manrope` is a Reserved Font Name, so a subset copy — which is what "modified"
would mean here — would have to be renamed. Keeping it whole is cheaper than
being careful about that, and it is the same call the shipped faces made.
