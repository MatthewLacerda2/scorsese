# Humaaans, as downloaded

The SVG files of Pablo Stanley's **Humaaans** "Flat Assets" pack, exactly as the
official download ships them (folder structure and file names unchanged; the
pack's PNGs, `.DS_Store` and `__MACOSX` entries left out).

- **Source:** the free ($0) Gumroad listing, https://pablostanley.gumroad.com/l/humaaans
  (linked from https://www.humaaans.com/), downloaded by the maintainer on
  2026-10-10 as `humaaans-flat-assets.zip`,
  sha256 `cd38c5f1cf893a73bf8cddbe3728f244c485514a44a0ccd9c67cf7f7d50a8087`.
- **Licence:** CC0 Public Domain, "Free for commercial or personal use", as
  humaaans.com states it (read 2026-10-10, #1004).

#1004 turns these into the parts pages assemble; this folder is its input.

## What ships, and how

All 79 files are compiled into scorsese unchanged (614 KiB of SVG text), and
`crates/render/src/page/humaaans/` rewrites one each time a page asks for it,
at `https://lib.scorsese/humaaans/<kind>/<name>.svg`. Nothing in this folder
is edited, so it can always be checked against the zip.

| served as | from |
| --- | --- |
| `head/<name>` | `Single Pieces/Head/Front/` |
| `body/<name>` | `Single Pieces/Body/` |
| `standing/<name>`, `sitting/<name>` | `Single Pieces/Bottom/Standing/`, `…/Sitting/` |
| `seat/<name>`, `scene/<name>` | `Single Pieces/Objects/Seat/`, `Single Pieces/Scene/` |
| `person/<name>` | `Humaaans/` (the pack's 32 assembled people) |

A name is the file's, lowercased and hyphenated (`Skinny Jeans Walk.svg` →
`skinny-jeans-walk`, `Hijab2.svg` → `hijab-2`), except `Skinny Jeans 1.svg`,
the only sitting pair of jeans, served as `sitting/skinny-jeans`.
`humaaans/index.json` lists them all.

What the rewrite changes, and nothing else:

- Sketch's XML declaration, comment, `<title>`, `<desc>`, its layer-name ids
  and its `stroke="none"` / `stroke-width="1"` are dropped, so a page can give
  a part an outline and never finds the wrong element by id.
- A head, a body or a lower half is placed in one 300 × 426 frame, at the
  offsets every assembled person in the pack uses (head 82,0; body 22,82;
  lower half 0,187), so the three stack into a person.
- The ids a mask or a `<use>` points at begin with the part's name, and
  `xlink:href` is written `href`.
- A fill that is skin, hair, headwear or clothing gains
  `style="fill:var(--person-<region>,<its own colour>)"`, so it is recoloured
  by setting the variable and draws in the artist's colour when nothing is set.
  The pack's black and white shading and its scenes keep their colours.
