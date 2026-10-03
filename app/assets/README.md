# The logo

`logo.png` is the artwork, as drawn: 928x1129, opaque, white on black. It is
**authored and not rebuildable** — deleting it loses work, the same way a
recipe under `recipes/` does — so it is committed at full size even though
nothing loads it directly.

`icon.png` is what `main.rs` compiles into the binary and hands to the window:
512x512, square, RGBA. It is *derived* from `logo.png`, and this is how:

```
ffmpeg -i logo.png -vf \
  "crop=883:902:23:131,pad=982:982:(ow-iw)/2:(oh-ih)/2:black,scale=512:512:flags=lanczos" \
  -pix_fmt rgba icon.png
```

The crop is the artwork's own bounding box — the drawing sits low and slightly
left in its frame, with 131px of empty black above it and 96 below. Cropping to
the content and re-centring is what makes the figure legible at the ~24px a
taskbar actually draws; the original's margins would have rendered it as a small
smudge in a black square. The pad puts an even ~5% back so it does not touch the
edges.

## Why one size and not a set

A window icon is one image here. `eframe::icon_data::from_png_bytes` yields a
single `IconData`, and winit hands that to the platform, which scales it to
whatever the bar, the switcher or the dock asks for. 512 is above every size any
of them requests, so extra copies would be files nothing reads.

An installed desktop entry is a different question — that wants a set under
`hicolor/`, at fixed sizes, referenced by a `.desktop` file — and it belongs
with whatever packages the app rather than here. This directory is what the
binary itself carries.

That distinction is not only about tidiness. **X11 takes an icon from the
window**, which is what `with_icon` sets and what this file is for. **Wayland
does not** — a compositor matches the window's app id against an installed
desktop entry and takes the icon from there. `main.rs` sets the app id to
`scorsese` so such an entry has something to match, but until one is installed a
Wayland bar has nowhere to read a picture from. Shipping that entry is packaging
work and is not done here.

## Redoing it

Replace `logo.png`, re-run the command above with a crop matching the new
artwork's bounding box, and check the result at small sizes before committing.
There is no gate holding `icon.png` to `logo.png`: it is a derived *binary*, so
nothing here can compare them, and this file is the record instead.

# The window's typeface

`fonts/` holds the two faces the window is set in (#643), compiled into the
binary by `src/theme/fonts.rs`. **Geist** for everything proportional, because
it is the web app's face (`@fontsource-variable/geist` in `web/package.json`),
and **Geist Mono**, its sibling, for the timecodes and frame counts. Both are
SIL Open Font License 1.1 — `Geist-OFL.txt` is the licence as Vercel ships it,
and the OFL asks for nothing more than that it travel with the files.

They come from Vercel's `geist` npm package, version 1.7.2, unmodified:

| file | from | sha256 |
| --- | --- | --- |
| `Geist-Regular.ttf` | `dist/fonts/geist-sans/` | `5c8968eafb98a4c4f47033daf29e38e284a6f2a82eb017d171ab040fe7c4b615` |
| `GeistMono-Regular.ttf` | `dist/fonts/geist-mono/` | `42d8ad2e610238e64e8abfcde3037c63f7850a73928742b7ab7229d897bcb155` |

**Static Regular files, not the variable one the web loads**, because egui
draws a font at its default instance and has no weight axis to set: a variable
file would ship every weight to draw one. One weight is what egui can use —
a heading here is set apart by capitals, tracking and colour, as the web's
`text-xs uppercase tracking-wide` headings are, rather than by weight. egui's
own fonts stay behind both as the fallback, for the transport's symbols and
any glyph Geist does not have.
