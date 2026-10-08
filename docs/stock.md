# Stock footage, photos and animations — before a shot is generated

A shot comes from one of three places: the user's own footage, a paid
generation (`generate`; `guide prompts` and `guide prices`), or **free stock**
from Pixabay (#900). This page is about the third, and about when it is the
right call — and about its sibling, **free animations** from LottieFiles
(#903), at the end.

## When stock, when a generation

Reach for stock first when the shot is **generic**: a city at night, hands
typing on a laptop, a sunrise, an office, coffee being poured, a cat asleep,
a crowd, traffic, nature. A large share of the shots in a promotional, product
or social video are this kind, and Pixabay covers them for **$0**. The
cheapest Veo shot is $0.64 for 8 s (`guide prices`); real users have found a
few of those a video too expensive to repeat.

Generate when the shot has to be **unique**: the user's product, a specific
character or place, a camera move the story needs, anything no library could
have. Stock is someone else's shot; it will never show the user's thing.

Mixing is normal. A cut can be stock for its establishing and B-roll shots and
generated only where it must be, often for the price of its narration.

## How it works

1. **`stock_search`** (`scorsese stock search`) — words, plus `kind`
   (`video` or `image`), `orientation`, `style`, `min_seconds`, `page`. It
   answers five results a page — id, length, largest size, tags, author and
   Pixabay page — and **one contact sheet** of their previews, numbered in
   order. **Look at the sheet**: tags alone pick the wrong shot. For a video,
   `look: <id>` shows five frames across the whole shot before anything is
   imported.
2. **`stock_import`** (`scorsese stock import`) — one id or several, and the
   render `resolution` (default 1920x1080). The smallest file that fills that
   frame without being enlarged is downloaded, by its measured size — Pixabay's
   rendition names do not say a size. It becomes an ordinary `video` or `image`
   asset at `assets/pixabay-<id>.<ext>`, probed and hashed like any import: no
   new asset kind and no format change, and where it came from is in its name.
   From there it is footage like any other — trim, crop, speed, grade.

Both are free and need only `PIXABAY_API_KEY` (`docs/credentials.md`); neither
quotes.

## What Pixabay asks, and what that does here

- **Results are cached 24 hours**, in `cache/stock/` (rebuildable, never
  carried with a project). A second search for the same words in that window
  asks nothing of Pixabay, and an id a search returned is found again from the
  cache. On the web the cache is the user's own and outlives the call.
- **100 requests a minute per key.** One request fetches fifty results, so a
  search is usually one request and paging through it none.
- **Only what is chosen is downloaded**, one at a time, and always copied into
  the project — nothing is hotlinked. On the web, when several results are
  equally good the assistant shows them to the user as pictures to pick from
  (#901, `docs/web.md`); the picker displays the source's own preview and
  file links while they choose — a Lottie by LottieFiles' animated GIF (#908)
  — names Pixabay or LottieFiles as the source, and downloads only what they
  pick.
- **Results name their source.** Every reply that lists results says they come
  from Pixabay.

## Limits worth knowing

- **Photos top out at 1280 px wide** until the maintainer is granted Pixabay's
  full API access — soft as a full 1080p frame, fine as an inset or behind
  text. The import says when a file is smaller than the frame.
- **Footage has no orientation filter at Pixabay**; scorsese filters by the
  measured size, reading further pages until a page of results is full. A
  horizontal shot can also be cropped for a vertical cut.
- **No music or sound effects**: Pixabay's API does not offer them.
- **The licence** (Pixabay Content License) allows commercial use and
  modification with no attribution, but not reselling the media as-is.
  Identifiable people, logos or brands in a commercial video may need their
  consent, which is the user's responsibility.

## Animations: Lottie from LottieFiles

A character, a mascot, an animated icon or illustration — a cat waving hello, a
rocket taking off, a check mark ticking — is the kind of thing an agent cannot
draw well in a page and a generation charges for every time. LottieFiles hosts
a very large library of them, made by artists: free, transparent, vector (sharp
at any size) and seconds to place. **Reach for one first** when the idea is an
illustration in motion rather than a filmed shot.

1. **`stock_search` with `kind: lottie`** (`scorsese stock search --lottie`):
   five a page, each with its id, title, length, frame rate and size, and the
   same one contact sheet of their previews. `look: <id>` (with
   `kind: lottie`) shows five frames across it from LottieFiles' own video of
   it playing.
2. **`stock_import` with `kind: lottie`** (`scorsese stock import --lottie`):
   the animation's JSON is written **beside the pages**, as
   `pages/lottie-<id>.json`. It is **not an asset** and is never placed by
   itself: a page loads it and plays it with the shipped lottie-web, driven
   from the page's clock, and the page goes on the timeline (`guide pages`,
   section "A Lottie animation", has the page to copy). Beside the pages rather
   than in `assets/` because that is where a page's own files live, in a
   `.scor` folder and on the web alike.

**No key at all**: LottieFiles answers its public search anonymously. Results
are cached 24 hours beside Pixabay's, in `cache/stock/`.

**The licence** (Lottie Simple License) allows commercial use and changes —
recolouring one to a brand colour is a page concern (`guide pages`) —
with attribution encouraged and not required. It forbids redistributing the
files on their own or gathering them into a library or competing service, which
is why scorsese **never ships one**: an animation is downloaded into one project
when it is used, which is what the licence permits. Premium animations are not
in the public search and are out of reach.

**Limits worth knowing.** On the web a project keeps the files beside its pages
up to 1 MB each, so a heavier animation (one with many pictures embedded)
cannot be kept there; the import says so. A Lottie that names pictures it does
not carry is rare, and the import names them. dotLottie (`.lottie`) files are
not used: the plain JSON is enough.
