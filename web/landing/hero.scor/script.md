# The landing page's film (#904)

The film at the top of scorsese's landing page (`web/src/landing/`). It is the
proof the page makes: made only with what scorsese makes videos with — four
pages, one native text, one synthesised score — and so it cost $0 and can be
re-rendered by anyone with the CLI (`make landing-hero` from the repo root).

The story is the pitch, in three steps and a name: **your files** (cards fanning
in), **your words** (a brief typing itself), **your video** (a timeline
assembling while a little promo plays above it), then **scorsese**.

Rules it keeps:

- No provider generation: no narration, no generated shots or stills.
- Every word on screen is the customer's language, never JSON, MCP or a
  provider's name.
- The score is 100 bpm, two bars a scene: each scene is 144 frames at 30 fps,
  and the crash lands on the cut to the name. Retiming a scene means changing
  the recipe's arrangement with it.
- The landing page's *No surprises* section quotes this film's length (21.6 s)
  and its scene count (4) in `web/src/landing/Sections.tsx`: change them there
  when the film changes.
