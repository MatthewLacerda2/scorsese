# Pages — motion graphics from a web page

A **page** is an HTML file under `pages/` that an `html` asset plays. On the
timeline it is a picture with alpha: a title card, a lower third, a counter
ticking up, a diagram drawing itself in. Placed by a clip on a video track like
footage, and everything a clip does (transform, opacity, mattes, groups,
keyframes) applies to it unchanged.

Reach for one when a `text`, `shape` or `icon` asset would need a dozen layers
and a page of keyframes to say what twenty lines of CSS say. Layout, type,
gradients, SVG and animation are what a page is good at. A single word on
screen is still a `text` asset.

**Pages cost nothing.** No key, no network, no money: a page is drawn on this
machine by a headless browser and cached in `cache/pages/`, so writing one,
looking at it and writing it again is the loop to be in.

```
page_write  project: …, page: "lower-third", html: "<!doctype html>…"
place_clip  … asset: "lower-third" on a video track
still       at: ["0.5s", "2s", "3.8s"]      # look; read the notes under each frame
page_read   page: "lower-third"             # before changing it
```

From a shell it is `scorsese page lower-third lower-third.html` to write one
(`-` reads standard input) and `scorsese page lower-third` to print it.
`scorsese import page.html` copies an existing page in.

## The contract

A page is drawn frame by frame at the render's raster and frame rate, on a clock
that belongs to the clip. What it can count on:

- **`window.scorsese`**, set before any of its scripts run, and frozen:

  | field | what |
  | --- | --- |
  | `width`, `height` | the viewport, in CSS pixels |
  | `fps` | the frame rate it is drawn at |
  | `duration` | seconds, where its clock is at the clip's last frame |
  | `clips` | where every clip beside it sits, by id: `{start, end}` in its own seconds |
  | `words` | when each word of a generated narration beside it is said, as `"<clip id>/<word>"`: `{start, end}` in its own seconds |

- **The shorter side of the viewport is 1080 CSS pixels**, at every raster. A
  landscape render is a 1920 × 1080 page, a vertical one 1080 × 1920, a square
  one 1080 × 1080. Write sizes in `px` against that and the page lays out the
  same in a quarter-size preview and in a 4K delivery, only softer or sharper.
  `vmin` is 10.8 px everywhere, which is the unit for a page meant to survive
  every aspect.
- **A clock that only moves when a frame is drawn.** `performance.now()`,
  `Date`, `setTimeout`, `setInterval`, `requestAnimationFrame`, and every CSS
  animation, transition and Web Animation follow the clip. A frame is a
  function of its time and of nothing else, so the same page draws the same
  frames on every machine.
- **The clock starts at the clip's `source_in`, and runs at its `speed`.** A
  clip trimmed into a page starts the animation part-way, as trimming footage
  would. `duration` already accounts for both.
- **The clock reads 100 ms at the page's time zero, not 0.** `performance.now()`,
  `Date` and the `requestAnimationFrame` timestamp all start 0.1 s in, because
  anime.js reads a timestamp of 0 as "not started yet" (#606). CSS animations,
  transitions, Web Animations and anime.js are unaffected. A script that reads
  the time itself must subtract it, or it runs **three frames early at 30 fps**,
  and every exit timed back from `duration` ends before the cut (#917). The
  kit's `kit.frame` (*Sharing code between pages*) hands a callback the page's
  seconds with it already taken off. Without the kit, read them as
  `(performance.now() - start) / 1000`, where `start` is `performance.now()`
  read when the script first runs, as *A Lottie animation* below does. That is
  the page's time zero, and a trimmed clip still starts part-way.
- **A transparent background.** Where the page draws nothing, the tracks below
  it show through. A page that should cover the frame paints its own
  background, on `body`.

### Time the exit to `duration`

A page has no length of its own: the clip decides it. An entrance is timed from
zero; an exit is timed back from `duration`, so the page leaves on the clip's
last frame however long the clip is cut. One line hands it to CSS:

```html
<script>
  document.documentElement.style.setProperty("--out", scorsese.duration - 0.6 + "s");
</script>
```

and the exit is then `animation: leave 600ms var(--out) ease-in forwards`.
`forwards` and not `both`: an exit that fills backwards would hold the page at
its last keyframe from the first frame.

### Time it to the edit with `scorsese.clips`

A page that runs beside the edit — a diagram lit while the narration is on each
part of it, a title landing on the music's downbeat — times itself to the clips,
never to seconds copied from where they sit. `scorsese.clips` holds every clip
on the timeline the page's clip sits on, by clip id, as `{start, end}` in
**the page's own seconds**: the clock `duration` is on, already corrected for
where the page's clip starts, its `source_in` and its `speed`. A clip before the
page's starts at a negative time; one after it, past `duration`. The page's own
clip is there too.

```js
const line = scorsese.clips["nar-9-scale"];   // {start: 81.0, end: 86.4}
const drop = scorsese.clips["music"].start + 32.5;  // a beat inside the song
```

Move or re-record the narration and the page follows it with nothing edited:
it is drawn again, because **what a page read of `clips` is part of its
capture**. Only that: moving a clip the page never looked up draws nothing
again. Looking up an id no clip has gives `undefined`, and a clip given that id
later redraws the page too. Listing them all (`Object.keys`, `for … in`) makes
every clip part of it, so look up the ids the page needs.

A page inside a group is told the clips beside it in the group, on the group's
clock: the same however often the group is placed.

### Start on a spoken word with `scorsese.words`

A narration generated by scorsese comes back with **when each word is said**,
so a page can start on the word *gradient* rather than at a second somebody
measured. `scorsese.words` holds them, one entry per word, as `{start, end}` in
the page's own seconds, named by the narration's clip id and the word:

```js
const word = scorsese.words["nar-9/gradient"];  // {start: 82.15, end: 82.46}
```

A word is named by what it says, lowercased and without the punctuation around
it: `Gradient,` is `gradient`, `don't` keeps its apostrophe. The second time a
line says the same word it is `gradient@2`, the third `gradient@3`, counted over
the whole line, so trimming the clip renames nothing. The times go through the
narration's clip — where it starts, its `source_in` and its `speed` — and a word
its trim leaves unheard is not there at all.

Re-record or move the line and the page follows, exactly as with `clips`: what
a page read of `words` is part of its capture, a word it never looked up moving
draws nothing again, and listing them all makes every word part of it.

**Only generated narration has words.** Imported audio has none, and neither
does a line generated before scorsese kept them; looking one up gives
`undefined`, so a page should fall back rather than throw. `project_describe`
with `at` names the word each narration is saying at that instant, and says so
of a line with no timings. Nothing is regenerated to get them: a line keeps its
audio, and the next time its text changes it comes back timed.

## What is there offline

**Nothing on the internet.** A page is served from `https://page.scorsese/`,
whose paths are the project's, and every other request is refused. A refused
request is a warning, and the page is drawn without what it asked for.

- **The shipped faces, by family name**, with no `@font-face` needed: `Inter`,
  `Montserrat`, `Liberation Sans`, `Source Serif 4`, `Lora`,
  `Playfair Display`, `Liberation Serif`, `JetBrains Mono`. Each in every weight
  it ships and in italic. A family the page names and was not given falls back
  to a shipped face, the same one everywhere.
- **scorsese's motion kit**, at `https://lib.scorsese/kit.js`: the helpers a
  timed page needs, on the page's own seconds. See *Sharing code between
  pages*.
- **anime.js 3.2.2**, at `https://lib.scorsese/anime.min.js`.
- **opentype.js 2.0.0**, at `https://lib.scorsese/opentype.min.js`, which
  reads a glyph's outline out of a face, a thing the browser never hands a
  page. Load it for `kit.font` and `kit.write` (*The kit*), which read the
  shipped faces with it, variable ones at any weight. The faces themselves are
  at `https://lib.scorsese/fonts/<file>.ttf`, listed with their family, weight
  and style at `https://lib.scorsese/fonts/index.json`.
- **lottie-web 5.13.0** (the full build: every renderer, and expressions), at
  `https://lib.scorsese/lottie.min.js`, to play a Lottie animation
  `stock_import` brought in. See *A Lottie animation* below for the one way
  to drive it.
- **The icon set**, every [Lucide](https://lucide.dev) icon the `icon` asset
  draws, at `https://lib.scorsese/icons/<name>.svg`. The `icons` tool finds a
  name from a word (`film` → `clapperboard`, `film`, `video`…). Each is a
  24-unit square stroked in `currentColor`, so the page decides colour and
  size; see *Icons* in the worked pages for the two ways. A plain `<img>` of
  one draws it black. A name the set lacks is a note naming the nearest ones.
- **The project's own files, by relative path.** From `pages/`, the project's
  `assets/photo.png` is `../assets/photo.png`, and a font file the project
  carries loads with an ordinary `@font-face` rule. Nothing outside the project
  folder, so the project still survives being copied to another machine.

**A shipped file is part of the page's capture.** A page that loads the kit,
anime.js, lottie-web, opentype.js, a face's file or an icon is drawn again when a build ships a different one, and
a page that does not load it is not.

**Inside Chromium's sandbox.** The browser that draws a page runs with its own
sandbox on, everywhere, so a page from a template, a shared project or an
agent's mistake cannot reach the machine's processes or files. It costs nothing
measurable and draws the same pixels. A machine that cannot start it — one
running as root, or a kernel that refuses unprivileged user namespaces, like
GitHub's ubuntu-24.04 runners — gets a note naming the opt-out, and nothing is
drawn without the sandbox until `SCORSESE_CHROME_NO_SANDBOX=1` is set. The web
app's captures have no opt-out.

**Not available:** `<video>` and `<audio>` inside a page (they run on their own
clock, and a warning says so: put footage and sound on the timeline instead),
workers' clocks, and `requestIdleCallback`.

## Seekable, not integrated

Write the page so that **what is on screen is a function of the time**: a CSS
`@keyframes` animation, a transition, a Web Animation, an anime.js timeline, or
a `requestAnimationFrame` callback that reads the timestamp it is handed and
computes the frame from it (less the page's time zero, which reads 100 ms: see
*The contract*).

Avoid the loop that **accumulates**: `x += speed * dt` each frame, a particle
system stepping its state, anything whose frame 90 is only reachable by running
frames 0 to 89 first. It draws correctly — a still or a piece of a render that
starts part-way still runs every frame before it, only without drawing them
(#809) — but every one of those frames costs its script and its layout again on
every look, and it is the shape that goes wrong when a frame is dropped. If a
page needs randomness, seed it: `Math.random` is the browser's, and a page that
should look the same twice should not use it.

**A library with its own playback is driven, never played.** lottie-web, like
any player, would run its animation on its own loop; loaded with
`autoplay: false`, it draws exactly the frame `goToAndStop` names, so the page
names the frame for its time on every `requestAnimationFrame` (*A Lottie
animation*, below).

**Only the frames on screen are drawn.** A `still` 85 s into a 90 s page draws
the few frames around that instant: the 2,550 before it are run, which is cheap
for a page whose frame is a function of the time, and never drawn, which is
nearly all a frame costs. A render draws each page clip's stretch from where it
enters the page, a long one in pieces at once.

## Sharing code between pages

Pages in one project that share a look or a set of helpers load them from **one
file in the project**, by relative path, rather than each carrying a copy:

```html
<script src="https://lib.scorsese/kit.js"></script>
<script src="lib.js"></script>          <!-- the project's pages/lib.js -->
<link rel="stylesheet" href="look.css"> <!-- the project's pages/look.css -->
```

**Every file a page loads is part of its capture**, by its contents. Edit
`pages/lib.js` and every page that loaded it is drawn again on the next look;
a page that never loaded it is not. Nothing else needs to be told. A shared
file is an ordinary file of the project, under `pages/` beside the pages that
load it, and never an asset: nothing places it. `page_write` writes one when
named with `file` instead of `page` (`"file": "lib.js"`, its text in `html`),
and `page_read` reads it back the same way — on the web that is the only way to
write one. A shared file is a `.js`, `.css`, `.json` or `.svg` of at most 1 MB;
a `.html` is refused, since an html file no asset points at is a page nothing
plays.

A classic script's top-level names are shared with the page that loads it and
with every other script on it, so two `const`s of one name are an error. Keep a
shared file to `function` declarations, and set anything else up inside one.
*Two pages sharing a file*, in the worked pages, is the whole pattern.

### The kit

`https://lib.scorsese/kit.js` defines one global, `kit`: the helpers
every timed page was writing for itself. Each takes the time and answers a
value, or sets an element's style, for that instant alone. Nothing is kept from
one frame to the next, so a page written on the kit is seekable by construction.
Times are the page's seconds.

| helper | what |
| --- | --- |
| `kit.frame(draw)` | calls `draw(t)` on every frame with the page's seconds: 0 on its first frame, `scorsese.duration` on the clip's last. The clock's 100 ms origin is already taken off |
| `kit.clamp(x, lo = 0, hi = 1)` | `x`, held inside `[lo, hi]` |
| `kit.lerp(a, b, p)` | from `a` to `b` as `p` goes from 0 to 1 |
| `kit.remap(x, a, b, c = 0, d = 1, ease)` | `x` from `[a, b]` onto `[c, d]`, held at the ends, eased on the way (linear by default) |
| `kit.ease.linear`, `.in`, `.out`, `.inOut`, `.back` | easings, progress 0–1 to eased progress: cubic, and `back`, which overshoots and settles |
| `kit.enter(t, at = 0, length = 0.6, ease = out)` | 0 before `at`, 1 once `length` seconds have passed |
| `kit.exit(t, length = 0.3, ease = in)` | 1 until the clip's last `length` seconds, then down to 0 on its last frame, however long the clip is cut |
| `kit.rise(element, t, at = 0, {length = 0.6, distance = 40, out = 0.3})` | fades `element` in as it rises `distance` px into place from `at`, and out with the clip's last `out` seconds (`out: 0` keeps it to the end) |
| `kit.stagger(index, step = 0.15, first = 0)` | when the `index`th of a row enters: `first + index * step` |
| `kit.count(t, at, length, to, {from = 0, decimals = 0, locale = "en-US"})` | the figure a count-up shows at `t`, grouped (`1,250,000`), eased out. Show it in `tabular-nums` |
| `kit.random(seed)` | a generator answering the same numbers in `[0, 1)` for the same seed, on every frame and every machine. Draw what a page needs once, up front |
| `kit.svg(tag, attributes, parent)` | an SVG element with its attributes set, appended to `parent` when one is given |
| `kit.draw(what, t, at = 0, {speed = 1500, within, fill = 0.4})` | draws the strokes of `what` (an SVG element, or a list of them) on in document order from `at`, at a constant `speed` in the drawing's own units a second, or all of them `within` that many seconds. A closed shape's fill fades in over `fill` seconds once its outline is done. Answers when the last mark is finished, so the next can start after it |
| `kit.font(family, {weight = 400, italic = false})` | a promise of a shipped face (`"Montserrat"`, or its scorsese name `montserrat`) as opentype.js reads it, at the nearest weight it ships. Needs `opentype.min.js` loaded first. A family it does not ship is an error naming the ones it does |
| `kit.write(parent, face, text, {x, y, size = 96, width, leading = 1.2, align = "start", fill, stroke, strokeWidth})` | `text` as glyph outlines on `parent`, baseline at `y`, broken at `width` or a `\n`, each line set from `x` by `align` (`start`, `middle`, `end`). Answers the group, which holds a `<g data-word>` per word named as `scorsese.words` names a spoken one (`free`, then `free@2`). Draw it, or one word of it, with `kit.draw` |
| `kit.camera(canvas, t, views)` | moves `canvas`, an element at the page's top left larger than the frame, under a fixed camera: each view is `{at, x, y, zoom = 1, length = 1}`, the canvas point the frame centres on and how close, reached `length` seconds after `at`, eased in and out. The first view is where it starts |

Anything else is the page's own, or a shared file's. A helper joins the kit
when pages keep writing it, and each one in it is used by a worked page below.

## What works

These are observations about what reads well on a video, not rules the renderer
holds. Every one is a choice the page makes.

- **Keep inside the safe margins.** Nothing important within 5% of the
  frame's edge (96 px at the sides of a landscape page, 54 px at the top and
  foot): players crop, and captions sit at the bottom. Text that does not is
  a note (below).
- **Type is bigger on video than on a web page.** A title is 100–160 px, body
  text no smaller than 36 px. Two families at most, usually one sans and one
  serif or display face.
- **Text over footage needs help.** A translucent panel behind it, or a soft
  `text-shadow`, because the shot under a lower third changes and the text
  must stay readable over all of it.
- **Entrances ease out, exits ease in.** Things arrive fast and settle
  (`cubic-bezier(.2, .7, .2, 1)`, 400–900 ms), and leave quickly. Stagger
  related elements by 100–200 ms rather than moving everything at once.
- **Hold still long enough to be read.** A rule of thumb is a second plus a
  second for every five words, between the end of the entrance and the start
  of the exit.
- **Counting numbers keep their width**: `font-variant-numeric: tabular-nums`,
  so the figure does not jitter as its digits change.
- **Lines draw with `stroke-dashoffset`.** Give an SVG path `pathLength="1"`,
  set `stroke-dasharray: 1` and animate `stroke-dashoffset` from 1 to 0. For a
  drawing of many strokes that should look drawn by one hand, `kit.draw`
  times them by their length instead, one after another.

## Warnings

Whatever a page does that its author should hear about comes back in words:
`still` writes a `note:` under the frame, and `render` puts one in its reply
(and the CLI prints one) for

- a request refused because pages render offline, or a WebSocket or WebRTC
  connection the page opened, which reaches nothing,
- a file the page asked for that is not in the project,
- a script that threw, with its message,
- a `<video>` or `<audio>` element,
- text laid out wrong and **held** there — running off the frame, inside the
  safe margin, overflowing the box painted behind it (a card's background,
  border or shadow), or overlapping other text. Each note quotes the text,
  names the side and the amount in CSS pixels, and says from when on the
  page's own clock. Only text is measured, by its lines rather than its
  element, and only what stays put for a quarter of a second: an entrance
  sliding in from off the frame, text that is hidden or faint, and anything
  positioned out of its box on purpose say nothing. Text is measured as much
  of it as shows: a word an `overflow: hidden` box or an `inset()` clip-path
  has hidden (a line-mask reveal, a word rotator) says nothing, and one half
  in is measured by its visible half — and is itself a note, naming the box
  that slices it, unless that box draws an ellipsis or scrolls. Masks and
  other clip shapes are not
  read. A box full of shapes is not checked; look at it,
- and a page that could not be captured at all. That clip shows the page's slug
  card (`PAGE · NOT CAPTURED`) instead, and the render still finishes. The usual
  cause is that there is no browser on the machine: `SCORSESE_CHROME`, then
  `chrome-headless-shell` on `PATH`, are where one is looked for. The next is
  a sandbox the machine cannot start, and that note says so.

A note is never an error, so read them. A page that threw on its first line
draws as an empty frame.

## Worked pages

Each of these is captured by the test suite (`crates/render/tests/pipeline/guide.rs`),
which fails if one draws nothing, warns or cannot be captured, so they stay
true as the renderer changes. The word after `html` names it.

### A title card

Full frame, its own background, a title rising in and the whole card fading
on the clip's last 600 ms.

```html page title-card
<!doctype html>
<html>
<head>
<style>
  html, body { margin: 0; height: 100%; }
  body {
    display: grid; place-content: center; text-align: center;
    background: radial-gradient(circle at 50% 40%, #1f2d52, #090d1a 70%);
    color: #f4efe6;
  }
  .card { animation: leave 600ms var(--out) ease-in forwards; }
  h1 {
    margin: 0; font: 700 150px/1 "Playfair Display"; letter-spacing: -2px;
    animation: rise 900ms cubic-bezier(.2, .7, .2, 1) both;
  }
  p {
    margin: 32px 0 0; font: 500 36px Inter; letter-spacing: 10px;
    text-transform: uppercase; opacity: .75;
    animation: rise 900ms 250ms cubic-bezier(.2, .7, .2, 1) both;
  }
  @keyframes rise { from { opacity: 0; transform: translateY(40px); } }
  @keyframes leave { to { opacity: 0; } }
</style>
<script>
  document.documentElement.style.setProperty("--out", scorsese.duration - 0.6 + "s");
</script>
</head>
<body>
  <div class="card">
    <h1>The Long Way Home</h1>
    <p>A film in three parts</p>
  </div>
</body>
</html>
```

### A lower third

No background at all, so the shot shows everywhere but the band. The bar wipes
in, the name and role follow it, and all of it slides out at the end.

```html page lower-third
<!doctype html>
<html>
<head>
<style>
  html, body { margin: 0; height: 100%; }
  .third {
    position: absolute; left: 96px; bottom: 120px;
    animation: out 500ms var(--out) cubic-bezier(.5, 0, .8, .3) forwards;
  }
  .bar {
    position: absolute; inset: 0; background: rgba(12, 14, 20, .78);
    border-left: 8px solid #f2b134; transform-origin: left;
    animation: wipe 600ms cubic-bezier(.2, .7, .2, 1) both;
  }
  .text { position: relative; padding: 22px 40px 24px 36px; color: #fff; }
  .name {
    font: 700 56px/1.1 Montserrat;
    animation: fade 500ms 250ms ease-out both;
  }
  .role {
    font: 400 36px/1.3 Inter; opacity: .8;
    animation: fade 500ms 400ms ease-out both;
  }
  @keyframes wipe { from { transform: scaleX(0); } }
  @keyframes fade { from { opacity: 0; transform: translateX(-16px); } }
  @keyframes out { to { opacity: 0; transform: translateX(-40px); } }
</style>
<script>
  document.documentElement.style.setProperty("--out", scorsese.duration - 0.5 + "s");
</script>
</head>
<body>
  <div class="third">
    <div class="bar"></div>
    <div class="text">
      <div class="name">Ana Ribeiro</div>
      <div class="role">Marine biologist, Azores</div>
    </div>
  </div>
</body>
</html>
```

### A stat counter

A figure counting up, in tabular figures so it never jitters, and its label
rising in under it. `kit.count` answers the figure for the time it is handed,
never adding to the one before, so any frame stands on its own.

```html page stat-counter
<!doctype html>
<html>
<head>
<script src="https://lib.scorsese/kit.js"></script>
<style>
  html, body { margin: 0; height: 100%; }
  body { display: grid; place-content: center; text-align: center; color: #fff; }
  .figure {
    font: 800 220px/1 Montserrat; font-variant-numeric: tabular-nums;
    text-shadow: 0 6px 30px rgba(0, 0, 0, .45);
  }
  .label {
    margin-top: 12px; font: 500 40px Inter; letter-spacing: 4px;
    text-transform: uppercase; opacity: 0;
  }
</style>
</head>
<body>
  <div class="figure" id="figure">0</div>
  <div class="label" id="label">trees planted this year</div>
  <script>
    const figure = document.getElementById("figure");
    const label = document.getElementById("label");
    kit.frame((t) => {
      figure.textContent = kit.count(t, 0, 2.2, 1250000);
      kit.rise(label, t, 0.6, { distance: 20 });
    });
  </script>
</body>
</html>
```

### A flowchart drawing in

SVG boxes appearing one after another, each connector drawing itself from the
box before. The `pathLength="1"` on every path is what lets one `@keyframes`
draw lines of any length.

```html page flowchart
<!doctype html>
<html>
<head>
<style>
  html, body { margin: 0; height: 100%; }
  body { display: grid; place-content: center; }
  svg { width: 1500px; overflow: visible; }
  .box rect { fill: rgba(255, 255, 255, .94); }
  .box text { font: 600 40px Inter; fill: #14213d; text-anchor: middle; dominant-baseline: middle; }
  .box { animation: pop 500ms cubic-bezier(.2, .9, .3, 1.2) both; transform-box: fill-box; transform-origin: center; }
  .line {
    fill: none; stroke: #f2b134; stroke-width: 6; stroke-linecap: round;
    stroke-dasharray: 1; stroke-dashoffset: 1;
    animation: draw 600ms ease-in-out forwards;
  }
  @keyframes pop { from { opacity: 0; transform: scale(.8); } }
  @keyframes draw { to { stroke-dashoffset: 0; } }
</style>
</head>
<body>
  <svg viewBox="0 0 1500 300">
    <g class="box" style="animation-delay: 0s">
      <rect x="0" y="100" width="340" height="110" rx="18" />
      <text x="170" y="155">Idea</text>
    </g>
    <path class="line" pathLength="1" d="M 350 155 H 570" style="animation-delay: .4s" />
    <g class="box" style="animation-delay: .9s">
      <rect x="580" y="100" width="340" height="110" rx="18" />
      <text x="750" y="155">Prototype</text>
    </g>
    <path class="line" pathLength="1" d="M 930 155 H 1150" style="animation-delay: 1.3s" />
    <g class="box" style="animation-delay: 1.8s">
      <rect x="1160" y="100" width="340" height="110" rx="18" />
      <text x="1330" y="155">Launch</text>
    </g>
  </svg>
</body>
</html>
```

### Icons

A row of three features, each with its icon. Two ways to colour a shipped icon,
and this page uses both:

- **As a mask**, for an icon that only needs a colour and a size: a box whose
  `background` is the colour and whose `mask` is the icon. The box's width and
  height are the icon's.
- **Inlined**, for an icon whose strokes should move: `fetch` it and set it as
  the element's `innerHTML`, and it takes `color` from the element like text
  does. Its paths are then the page's, so `pathLength="1"` and the
  `stroke-dashoffset` draw above work on them. Fetch it before starting the
  animation, as below.

```html page icons
<!doctype html>
<html>
<head>
<style>
  html, body { margin: 0; height: 100%; }
  body { display: grid; place-content: center; background: #0f1b2d; color: #f4efe6; }
  .row { display: flex; gap: 140px; }
  .feature { text-align: center; animation: rise 700ms cubic-bezier(.2, .7, .2, 1) both; }
  .feature p { margin: 28px 0 0; font: 600 44px Inter; }
  .icon {
    width: 180px; height: 180px; margin: 0 auto; background: #f2b134;
    mask: var(--icon) center / contain no-repeat;
  }
  .drawn { width: 180px; height: 180px; margin: 0 auto; color: #7bdff2; }
  .drawn svg { width: 100%; height: 100%; }
  .drawn path { stroke-dasharray: 1; stroke-dashoffset: 1; animation: draw 1.2s ease-in-out forwards; }
  @keyframes rise { from { opacity: 0; transform: translateY(40px); } }
  @keyframes draw { to { stroke-dashoffset: 0; } }
</style>
</head>
<body>
  <div class="row">
    <div class="feature">
      <div class="icon" style="--icon: url(https://lib.scorsese/icons/clapperboard.svg)"></div>
      <p>Shoot</p>
    </div>
    <div class="feature" style="animation-delay: .15s">
      <div class="icon" style="--icon: url(https://lib.scorsese/icons/scissors.svg)"></div>
      <p>Cut</p>
    </div>
    <div class="feature" style="animation-delay: .3s">
      <div class="drawn" id="share"></div>
      <p>Share</p>
    </div>
  </div>
  <script>
    fetch("https://lib.scorsese/icons/send.svg").then((r) => r.text()).then((svg) => {
      const box = document.getElementById("share");
      box.innerHTML = svg;
      box.querySelectorAll("path").forEach((path) => path.setAttribute("pathLength", "1"));
    });
  </script>
</body>
</html>
```

### In step with the narration

A stack of three blocks, each lit for exactly as long as the narration line
about it plays. Each block names its line's clip id in `data-clip`; the script
turns that clip's `{start, end}` into an animation of that length starting at
that time, which holds the lit look only while it runs. Nothing in the page is a
number of seconds, so re-cutting the narration re-times the page on its own. A
block whose id is not on the timeline stays dim.

```html page in-step
<!doctype html>
<html>
<head>
<style>
  html, body { margin: 0; height: 100%; }
  body { display: grid; place-content: center; background: #0f1b2d; }
  .stack { display: flex; flex-direction: column; gap: 28px; width: 900px; }
  .block {
    padding: 34px 48px; border-radius: 18px; font: 600 52px Inter;
    color: #f4efe6; background: rgba(255, 255, 255, .08); opacity: .4;
  }
  @keyframes lit {
    from, to { opacity: 1; background: #f2b134; color: #14213d; }
  }
</style>
</head>
<body>
  <div class="stack">
    <div class="block" data-clip="vo-encoder">Encoder</div>
    <div class="block" data-clip="vo-memory">Memory</div>
    <div class="block" data-clip="vo-decoder">Decoder</div>
  </div>
  <script>
    for (const block of document.querySelectorAll("[data-clip]")) {
      const line = scorsese.clips[block.dataset.clip];
      if (!line) continue;
      block.style.animation = `lit ${line.end - line.start}s linear ${line.start}s`;
    }
  </script>
</body>
</html>
```

### On the word

A title whose words come up as the narration says them. Each word of the title
names the spoken word it waits for in `data-word`, as `<clip id>/<word>`; the
script starts its entrance at that word's `start`. A word with no timing to
wait for — the line imported, or not on the timeline — is simply shown, so the
title never goes missing.

```html page on-the-word
<!doctype html>
<html>
<head>
<style>
  html, body { margin: 0; height: 100%; }
  body { display: grid; place-content: center; }
  h1 { margin: 0; font: 800 120px Montserrat; color: #fff;
       text-shadow: 0 6px 24px rgba(0, 0, 0, .45); }
  h1 span { display: inline-block; margin: 0 .12em; }
  @keyframes up { from { opacity: 0; transform: translateY(40px); } }
</style>
</head>
<body>
  <h1>
    <span data-word="vo/the">The</span>
    <span data-word="vo/gradient">gradient</span>
    <span data-word="vo/flows">flows</span>
    <span data-word="vo/back">back</span>
  </h1>
  <script>
    for (const span of document.querySelectorAll("[data-word]")) {
      const word = scorsese.words[span.dataset.word];
      if (!word) continue;
      span.style.animation = `up 400ms cubic-bezier(.2, .8, .2, 1) ${word.start}s both`;
    }
  </script>
</body>
</html>
```

### Two pages sharing a file

An explainer's usual shape: a **diagram** that stays on screen the whole time,
building up and lighting the part the narration is on, and short **close-ups**
on the track above it, each drawing only the right half of the frame so the
diagram shows through on the left. Both load the kit and the project's own
`pages/lib.js`, which holds what they share: the palette, and how a labelled
box is drawn.

```js file pages/lib.js
// What this project's pages share. Loaded after the kit.
function palette() {
  const root = document.documentElement.style;
  root.setProperty("--paper", "#0f1b2d");
  root.setProperty("--ink", "#f4efe6");
  root.setProperty("--accent", "#f2b134");
}

// A labelled box on `svg`, as a group the page animates.
function box(svg, x, y, width, height, label) {
  const group = kit.svg("g", { class: "box" }, svg);
  kit.svg("rect", { x, y, width, height, rx: 18 }, group);
  kit.svg("text", { x: x + width / 2, y: y + height / 2 }, group).textContent = label;
  return group;
}

palette();
```

The diagram's boxes pop in one after another, outlined, and each is filled
while its narration line plays (`scorsese.clips`, as *In step with the narration*
above), fading over 0.3 s at either end. A seeded drift of dust behind them
moves with the time, never by steps.

```html page diagram
<!doctype html>
<html>
<head>
<script src="https://lib.scorsese/kit.js"></script>
<script src="lib.js"></script>
<style>
  html, body { margin: 0; height: 100%; }
  body { background: var(--paper); }
  svg { position: absolute; inset: 0; width: 100%; height: 100%; }
  .box { transform-box: fill-box; transform-origin: center; }
  .box rect { fill: var(--accent); stroke: var(--ink); stroke-width: 3; }
  .box text { font: 600 44px Inter; fill: var(--ink); text-anchor: middle; dominant-baseline: middle; }
  circle { fill: var(--ink); opacity: .3; }
</style>
</head>
<body>
  <svg id="stage" viewBox="0 0 1920 1080"></svg>
  <script>
    const stage = document.getElementById("stage");
    const dust = kit.random(7);
    const motes = Array.from({ length: 40 }, () => ({
      dot: kit.svg("circle", { cx: dust() * 1920, r: 2 + dust() * 3 }, stage),
      y: dust() * 1080,
    }));
    const parts = ["Encoder", "Memory", "Decoder"].map((label, i) => ({
      group: box(stage, 160, 230 + i * 230, 560, 160, label),
      line: scorsese.clips["vo-" + label.toLowerCase()],
    }));
    kit.frame((t) => {
      for (const { dot, y } of motes) {
        dot.setAttribute("cy", kit.lerp(y, y - 80, t / scorsese.duration));
      }
      parts.forEach(({ group, line }, i) => {
        const at = kit.stagger(i, 0.25);
        const lit = line ? kit.enter(t, line.start, 0.3) * kit.clamp((line.end - t) / 0.3) : 0;
        group.style.opacity = kit.enter(t, at, 0.3) * kit.exit(t, 0.6);
        group.style.transform = `scale(${kit.lerp(0.8, 1, kit.enter(t, at, 0.6, kit.ease.back))})`;
        group.querySelector("rect").style.fillOpacity = lit;
      });
    });
  </script>
</body>
</html>
```

The close-up names nothing the diagram did not: the same palette, the same
boxes, each filled to its share as it rises in.

```html page close-up
<!doctype html>
<html>
<head>
<script src="https://lib.scorsese/kit.js"></script>
<script src="lib.js"></script>
<style>
  html, body { margin: 0; height: 100%; }
  .half { position: absolute; inset: 0 0 0 50%; background: var(--paper); }
  h2 { margin: 120px 0 0 110px; font: 700 72px Montserrat; color: var(--ink); opacity: 0; }
  svg { position: absolute; left: 0; top: 280px; width: 960px; height: 720px; }
  .box rect { fill: none; stroke: var(--ink); stroke-width: 3; }
  .box text {
    font: 600 40px Inter; fill: var(--ink); text-anchor: middle; dominant-baseline: middle;
    stroke: var(--paper); stroke-width: 8px; paint-order: stroke;
  }
  .fill { fill: var(--accent); }
</style>
</head>
<body>
  <div class="half" id="half">
    <h2 id="title">Inside the memory</h2>
    <svg id="slots" viewBox="0 0 960 720"></svg>
  </div>
  <script>
    const half = document.getElementById("half");
    const title = document.getElementById("title");
    const slots = document.getElementById("slots");
    const shares = [["Keys", 0.9], ["Values", 0.6], ["Gates", 0.35]].map(([label, share], i) => ({
      fill: kit.svg("rect", { class: "fill", x: 110, y: 40 + i * 200, height: 140, rx: 18 }, slots),
      share,
      group: box(slots, 110, 40 + i * 200, 740, 140, label),
    }));
    kit.frame((t) => {
      half.style.opacity = kit.exit(t);
      kit.rise(title, t, 0, { out: 0 });
      shares.forEach(({ fill, share, group }, i) => {
        const at = kit.stagger(i, 0.2, 0.4);
        kit.rise(group, t, at, { out: 0 });
        fill.setAttribute("width", kit.remap(t, at + 0.3, at + 1.3, 0, 740 * share, kit.ease.inOut));
      });
    });
  </script>
</body>
</html>
```

Put the diagram on the lower video track for the whole stretch, and each
close-up above it where its part is explained. Editing `pages/lib.js`, say the
accent colour, draws both again.

### A Lottie animation

A character, a mascot, an animated icon or illustration: a Lottie from
LottieFiles is free, transparent and sharp at any size, and is usually the
right call over drawing one in code or paying for a generation. Find one with
`stock_search` (`kind: lottie`) and bring it in with `stock_import`, which
writes its JSON **beside the pages**, as `pages/lottie-<id>.json`, and says the
name to load. It is not an asset and is never placed by itself: a page plays it,
and the page is what goes on the timeline.

The page loads the file, hands it to lottie-web with **`autoplay: false`**, and
on every `requestAnimationFrame` turns the time it is handed into the
animation's frame and draws exactly that one with `goToAndStop(frame, true)`.
`performance.now()` when the script first runs is the page's time zero (it
reads 100 ms there, not 0: see *The contract*), so `(now - start) / 1000` is the
page's seconds and any frame stands on its own. A
fractional frame is drawn in between, so the animation is smooth at any render
rate. `% anim.totalFrames` loops it; `Math.min(at, anim.totalFrames - 1)` plays
it once and holds the last frame.

The file can be changed before it is played, since it is only JSON. This page
swaps one of its colours for a brand colour: every flat fill or stroke within
a whisker of `from` becomes `to`, as `[r, g, b]` from 0 to 1. Here the file is
`wave.json`, a hand-made stand-in for the one `stock_import` writes; use the
name the import gave.

```html page lottie
<!doctype html>
<html>
<head>
<script src="https://lib.scorsese/lottie.min.js"></script>
<style>
  html, body { margin: 0; height: 100%; }
  body { display: grid; place-content: center; }
  #wave { width: 720px; height: 720px; }
</style>
</head>
<body>
  <div id="wave"></div>
  <script>
    const FILE = "wave.json";                  // the name stock_import gave
    const start = performance.now();           // the page's time zero

    function recolour(node, from, to) {
      if (Array.isArray(node)) return node.forEach((one) => recolour(one, from, to));
      if (!node || typeof node !== "object") return;
      const fill = (node.ty === "fl" || node.ty === "st") && node.c && node.c.a === 0;
      if (fill && from.every((v, i) => Math.abs(node.c.k[i] - v) < 0.02)) {
        node.c.k = [...to, node.c.k[3] ?? 1];
      }
      Object.values(node).forEach((one) => recolour(one, from, to));
    }

    fetch(FILE).then((r) => r.json()).then((data) => {
      recolour(data, [0.2, 0.5, 0.9], [0.95, 0.69, 0.2]);
      const anim = lottie.loadAnimation({
        container: document.getElementById("wave"),
        renderer: "svg", loop: false, autoplay: false, animationData: data,
      });
      const draw = (now) => {
        const at = (now - start) / 1000 * anim.frameRate;
        anim.goToAndStop(at % anim.totalFrames, true);
        requestAnimationFrame(draw);
      };
      requestAnimationFrame(draw);
    });
  </script>
</body>
</html>
```

Its size is the container's: lottie-web fits the animation inside the box,
keeping its shape, so size and place the `div` as anything else on the page.
`still` shows where a frame of it lands. A file over 1 MB cannot be kept by a
project on the web, which keeps the files beside its pages up to that size;
choose a lighter one there.

### A board that draws itself

The whiteboard explainer: a drawing and the words beside it appear stroke by
stroke, in step with the narration, on one canvas the camera moves across. The
canvas is an SVG twice the frame each way, and the camera is `kit.camera`
moving it. Here it frames the figure, pans to the verse once the figure is
drawn, and pulls back to show the whole board for the clip's last second.

The figure is ordinary SVG, drawn by `kit.draw` in the order it is written,
each stroke taking as long as it is long. The verse is `kit.write`: each
letter's outline drawn, then filled. Each word starts on the word the
narration (clip `vo`) says, from `scorsese.words`, and without one it follows
the word before. `free` is coloured by its `data-word` and pops once it is
written. Every helper answers when it is finished, so the camera and the pop
are timed from what came before rather than from numbers of seconds.

```html page board
<!doctype html>
<html>
<head>
<script src="https://lib.scorsese/opentype.min.js"></script>
<script src="https://lib.scorsese/kit.js"></script>
<style>
  html, body { margin: 0; height: 100%; overflow: hidden; }
  body { background: #f7f3ea; }
  #board { position: absolute; left: 0; top: 0; width: 3840px; height: 2160px; color: #1d2433; }
  #figure > * { fill: none; stroke: currentColor; stroke-width: 10;
    stroke-linecap: round; stroke-linejoin: round; }
  #figure .sun { fill: #f2b134; }
  [data-word="free"] { color: #c8452c; transform-box: fill-box; transform-origin: center; }
</style>
</head>
<body>
  <svg id="board" viewBox="0 0 3840 2160">
    <g id="figure">
      <circle class="sun" cx="960" cy="860" r="150" />
      <line x1="960" y1="660" x2="960" y2="590" />
      <line x1="1160" y1="860" x2="1230" y2="860" />
      <line x1="760" y1="860" x2="690" y2="860" />
      <line x1="1105" y1="715" x2="1155" y2="665" />
      <line x1="815" y1="715" x2="765" y2="665" />
      <path d="M 300 1440 C 560 1160 820 1160 1080 1380 S 1520 1200 1640 1440" />
      <path d="M 960 1440 C 900 1500 1020 1560 960 1620" />
    </g>
  </svg>
  <script>
    const board = document.getElementById("board");
    const figure = document.getElementById("figure");
    // When the narration says `word`, or `otherwise` without a timed one.
    const cue = (word, otherwise) => scorsese.words["vo/" + word]?.start ?? otherwise;

    kit.font("Playfair Display", { weight: 700 }).then((face) => {
      const verse = kit.write(board, face, "The truth will set you free.",
        { x: 2880, y: 1060, size: 150, width: 1500, align: "middle", strokeWidth: 3 });
      const free = verse.querySelector('[data-word="free"]');
      kit.frame((t) => {
        const drawn = kit.draw(figure, t, 0, { within: 1.2 });
        let done = 0;
        verse.querySelectorAll("[data-word]").forEach((word, i) => {
          done = kit.draw(word, t, cue(word.dataset.word, drawn + 0.6 + i * 0.25), { speed: 4000 });
        });
        free.style.transform = `scale(${kit.lerp(1, 1.15, kit.enter(t, done, 0.4, kit.ease.back))})`;
        kit.camera(board, t, [
          { x: 960, y: 1080 },
          { at: drawn, x: 2880, y: 1080, length: 0.8 },
          { at: scorsese.duration - 1, x: 1920, y: 1080, zoom: 0.5 },
        ]);
      });
    });
  </script>
</body>
</html>
```

Draw any SVG this way: a figure written out by hand, as here, an icon fetched
from the set (*Icons*), or a drawing `page_write` carries inline. A shape with
only a fill, no stroke, appears at its turn rather than being drawn.

### A traced picture, drawn on

A figure drawn by hand in SVG is only as good as the agent drawing it. A
generated illustration is better, and `vectorize` traces it into
`pages/<name>.svg`, beside the pages, for a page like this one to draw. The
SVG's marks are already in the order a hand would make them: the dark
outlines first, each one a single pen stroke, then every colour sketched and
filled. So the page only loads the file, sizes it, and hands its drawing group
to `kit.draw`. The group's id is the name it was traced under, here `figure`.

The figure starts when the narration (clip `vo`) says *teacher*, or 0.2 s in
without a timed one, and is drawn within two and a half seconds whatever its
size. Pass `speed` instead to let a busier drawing take longer.

```html page traced
<!doctype html>
<html>
<head>
<script src="https://lib.scorsese/kit.js"></script>
<style>
  html, body { margin: 0; height: 100%; overflow: hidden; }
  body { background: #f7f3ea; display: grid; place-content: center; }
  #art svg { display: block; height: 900px; width: auto; }
</style>
</head>
<body>
  <div id="art"></div>
  <script>
    // When the narration says `word`, or `otherwise` without a timed one.
    const cue = (word, otherwise) => scorsese.words["vo/" + word]?.start ?? otherwise;

    fetch("figure.svg").then((r) => r.text()).then((svg) => {
      document.getElementById("art").innerHTML = svg;
      const figure = document.getElementById("figure");
      kit.frame((t) => kit.draw(figure, t, cue("teacher", 0.2), { within: 2.5 }));
    });
  </script>
</body>
</html>
```

Two traced drawings can share a page: every id in one begins with its name.
Placed on the board of *A board that draws itself*, a traced figure takes the
place of the hand-written one, and `kit.camera` moves across it the same way.
Trace again with other choices (`colours`, `detail`, `keep_background`) to
change it; the page does not change.
