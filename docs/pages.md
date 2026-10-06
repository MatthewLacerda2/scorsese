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

## What is there offline

**Nothing on the internet.** A page is served from `https://page.scorsese/`,
whose paths are the project's, and every other request is refused. A refused
request is a warning, and the page is drawn without what it asked for.

- **The shipped faces, by family name**, with no `@font-face` needed: `Inter`,
  `Montserrat`, `Liberation Sans`, `Source Serif 4`, `Lora`,
  `Playfair Display`, `Liberation Serif`, `JetBrains Mono`. Each in every weight
  it ships and in italic. A family the page names and was not given falls back
  to a shipped face, the same one everywhere.
- **anime.js 3.2.2**, at `https://lib.scorsese/anime.min.js`.
- **The project's own files, by relative path.** From `pages/`, the project's
  `assets/photo.png` is `../assets/photo.png`, and a font file the project
  carries loads with an ordinary `@font-face` rule. Nothing outside the project
  folder, so the project still survives being copied to another machine.

**Not available:** `<video>` and `<audio>` inside a page (they run on their own
clock, and a warning says so: put footage and sound on the timeline instead),
workers' clocks, and `requestIdleCallback`.

## Seekable, not integrated

Write the page so that **what is on screen is a function of the time**: a CSS
`@keyframes` animation, a transition, a Web Animation, an anime.js timeline, or
a `requestAnimationFrame` callback that reads the timestamp it is handed and
computes the frame from it.

Avoid the loop that **accumulates**: `x += speed * dt` each frame, a particle
system stepping its state, anything whose frame 90 is only reachable by drawing
frames 0 to 89 first. It draws correctly, because the clock steps in order, but
it is the shape that cannot be captured in pieces or scrubbed cheaply, and it
is the one that goes wrong when a frame is dropped. If a page needs randomness,
seed it: `Math.random` is the browser's, and a page that should look the same
twice should not use it.

## What works

These are observations about what reads well on a video, not rules the renderer
holds. Every one is a choice the page makes.

- **Keep inside the safe margins.** Nothing important within about 5% of the
  frame's edge (96 px on a landscape page): players crop, and captions sit at
  the bottom.
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
  set `stroke-dasharray: 1` and animate `stroke-dashoffset` from 1 to 0.

## Warnings

Whatever a page does that its author should hear about comes back in words:
`still` writes a `note:` under the frame, and `render` puts one in its reply
(and the CLI prints one) for

- a request refused because pages render offline,
- a file the page asked for that is not in the project,
- a script that threw, with its message,
- a `<video>` or `<audio>` element,
- and a page that could not be captured at all. That clip shows the page's slug
  card (`PAGE · NOT CAPTURED`) instead, and the render still finishes. The usual
  cause is that there is no browser on the machine: `SCORSESE_CHROME`, then
  `chrome-headless-shell` on `PATH`, are where one is looked for.

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

anime.js counting a figure up, in tabular figures so it never jitters. The
number is drawn from the animated value every frame, never accumulated, so any
frame stands on its own.

```html page stat-counter
<!doctype html>
<html>
<head>
<script src="https://lib.scorsese/anime.min.js"></script>
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
    const shown = { value: 0 };
    const figure = document.getElementById("figure");
    anime.timeline({ easing: "easeOutExpo" })
      .add({
        targets: shown, value: 1250000, round: 1, duration: 2200,
        update: () => { figure.textContent = shown.value.toLocaleString("en-US"); }
      })
      .add({ targets: "#label", opacity: [0, .85], translateY: [20, 0], duration: 700 }, 600);
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
