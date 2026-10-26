# The live provider check

There is no official Rust SDK for Veo, ElevenLabs or Claude, so every client
scorsese has is plain HTTP somebody here wrote (`crates/providers/src/api/`).
The tests prove that code agrees with **our fixtures**. They cannot notice a
vendor changing its API, or a change of ours drifting from what the vendor
accepts — and on the hosted web app that failure is a paying user's generation
breaking in production.

`scorsese check-providers` is the other half: the smallest **real** call to each
vendor that exercises the path scorsese depends on, made through the product's
own clients, reporting for each vendor whether the reply is still the shape the
code reads. It is also how a hand-written fixture gets replaced by a body the
vendor actually sent.

It spends money and needs a network, so **no test and no CI job ever runs it**
— *no real provider calls in tests, ever*. It is run by a person, on purpose:
before a deploy, after bumping a provider model, or when a generation fails in
a way that smells like the vendor moved. Nothing schedules it; a cron would
spend money unattended, and that is a decision for once its cost is known.

## Running it

```sh
make live-check                                   # quote, ask, run
make live-check ARGS="--dry-run"                  # quote only; sends nothing
make live-check ARGS="--include-veo --record /tmp/live"
```

`make live-check` is `scorsese check-providers`; the flags are the command's.

- **It quotes first and asks**, exactly as `scorsese generate` does: anything
  but a plain `y` sends nothing, and with stdin not a terminal it refuses unless
  `--yes` was passed. A run that would cost nothing is not asked about.
- **It is held to `budget_cents`** (docs/credentials.md), even under `--yes`:
  a plan that would cross the ceiling is refused whole, before any call.
- **Keys resolve the one way they always do** — the environment or `.env`,
  then the settings file. A vendor with no key is reported **skipped**, never
  failed; today that is Anthropic, until a key exists.
- **A vendor that needs no key is always called** — today that is
  LottieFiles, whose public search answers anonymously. It is free, so it adds
  nothing to the quote. (In the library, `live::Options::keyless` is what turns
  it on; it is off by default so a run built from defaults, as a test's is, has
  nothing it could reach.)
- It exits non-zero when any vendor's verdict is a failure.

## What each vendor's check calls, and costs

| vendor | calls | costs |
| --- | --- | --- |
| Gemini (Veo) | `GET models/{id}` for each tier — `veo-3.1-generate-preview`, `…-fast-…` and `…-lite-…` — each listing `predictLongRunning` | free |
| | with `--include-veo`: one shot — 4 s, Lite, 720p — submitted, polled, downloaded | $0.20 |
| Gemini (stills) | `GET models/gemini-nano-banana-2.1`, the one image model, listing `generateContent` and `batchGenerateContent` | free |
| | one 1024x1024 still — Nano Banana 2.1, 1K, `1:1`, no references, the cheapest picture there is — through the product's own request | $0.04 |
| | the same still asked for as `image/png` (#1341), **reported**, never failed: accepted (the picture's header), refused (the vendor's words) or a JPEG drawn anyway | at most $0.04, nothing if refused |
| ElevenLabs | `GET /v1/voices?category=premade` (also picks the voice to speak with) | free |
| | text-to-speech of `Checking.` on the `fast` model, with its timestamps | $0.01 |
| | Voice Design from a 100-character passage — three candidates, **none kept** | $0.01 |
| | a 3-second instrumental track on `music_v2_5`, as `mp3_44100_128`, through the product's own `Music::compose` — a bare MP3, and how long it took | $0.01 |
| Anthropic | two streamed calls to `claude-opus-5-5` at `low` effort: ask for a tool call; then send the reply back unchanged, with an effort change, the tool's result and a mid-conversation `system` message | at most $0.15 |
| Pixabay | `GET /api/videos/?q=sunrise` — hits, each with a usable rendition | free |
| LottieFiles | `searchPublicAnimations("rocket", first: 3)`, anonymously — each result with an id, a `jsonUrl` and an `imageUrl`; `metadata`'s width, height, frame rate and duration carried by at least one | free, no key |

The ElevenLabs and Veo figures are the rate tables' own arithmetic
(docs/prices.md). Claude's quote is a ceiling — each call's `max_tokens` plus a
generous input, priced as the dearest input there is — and what the report says
was spent afterwards comes from the vendor's own token counts.

The still settles what #461's client took from Google's page without a
reply to check it against: the `interactions` endpoint and body, where the
picture sits in the reply, and that the size and shape are taken. A size or
shape the endpoint ignored is drawn at its own default and billed as asked; the
check reads the JPEG's frame header, so a picture that is not 1024x1024 is
reported as **shape changed** naming `image_size`. The PNG probe passes
whatever the vendor answers and fails only on no answer or a rejected key: its
line's note is the answer, for `ResponseFormat::mime_type` to follow.

The track settles what #1264 read off ElevenLabs' API reference: that
`mp3_44100_128` is taken on the music endpoint by a Starter key, that the
model and `force_instrumental` are taken as sent, and that the reply is the
MP3 itself. Its note says how long the track took, which the vendor does not
document.

The Anthropic calls confirm the three things #540 could not: the stream's real
events (every `fixtures/anthropic/*.sse` is hand-written), the
`thinking-display-updates-2026-08-18` beta header being accepted, and a
mid-conversation `role: "system"` message on `claude-opus-5-5`.

### Why the Veo shot is opt-in

A real generation is the only way to see a finished operation and the download
behind the key, but it costs ten to a hundred times the rest of the check
together and takes minutes. So by default Veo gets the free model lookups —
which catch the two likeliest breakages, a rejected key and a retired
`-preview` id — and the shot is one flag away, quoted and under the ceiling.

A request the API refuses on purpose, to check the endpoint without paying, was
considered and rejected: whether a malformed request is refused is the vendor's
call, and the day it starts clamping instead of refusing, a "free" check spends
money nobody quoted. `crates/providers/src/live/veo.rs` has the argument whole.

## Reading the report

Each vendor gets one verdict — its worst step — and each call a line:

| word | meaning |
| --- | --- |
| OK | the reply was the shape our client reads |
| shape changed | it was not; the line names the field (often in serde's own words) |
| refused | the vendor said no for a reason other than the key — a plan, a filter, a malformed request |
| auth failed | the vendor said no to the key. For ElevenLabs this includes a valid key missing a scope, and says so. For LottieFiles, which has no key, it means anonymous access was withdrawn — a `401`/`403`, or a GraphQL error about authorisation answered as `200` — and the line names the fallback: a key from the maintainer's LottieFiles account (#903) |
| unreachable | no answer arrived |
| unfinished | the call worked but stopped short of what could be checked — a shot still generating, a model that answered instead of calling the tool. Not a failure |
| skipped | not called: no key, or not asked for |

A **refused** with a `400` from Anthropic on the second call is the interesting
one: it is the API telling us, in its own words, which of the replay or the
mid-conversation system message it would not take.

## Replacing a fixture with a recorded body

`--record DIR` writes every reply the run received — one file per reply, named
for the vendor, its order, the endpoint and the status
(`anthropic-1-messages-200.sse`) — scrubbed of the key, of account ids
(`owner_id`, `public_owner_id`, `user_id`, …) and of signed URL queries. Media
bodies are not written. **Read them before committing**: scrubbing is
mechanical, and it only knows what it has been told to look for.

Then, for each hand-written fixture below:

1. Copy the recorded body over it, keeping the file name the tests use. Cut a
   long list down if the tests do not need it all, the way `premade.json` was
   cut to three voices — cut, never edit what remains.
2. The recorded body has no `_provenance` field and no leading `:` comment
   lines, so the "hand-written" mark goes with the old body.
3. Run the tests. A test that now fails was asserting a shape we imagined:
   fix the client or the test to what the vendor really sends, and say so in
   the pull request.

## Which fixtures are real

Every response shape scorsese parses, and where its test body came from.

| shape | read by | fixture | provenance |
| --- | --- | --- | --- |
| ElevenLabs `/v1/voices` | `api::elevenlabs::voices::Listing` | `fixtures/elevenlabs/premade.json` | **captured** 2026-08-05 |
| ElevenLabs `/v1/shared-voices` | `Listing` | `fixtures/elevenlabs/shared-voices-pt.json` | **captured** 2026-08-05 |
| ElevenLabs refusals (401 scope, 402 plan) | `api::elevenlabs::refusal` | inline in `tests/speaking/refusals.rs`, `src/api/elevenlabs/refusal.rs`, `tests/live/elevenlabs.rs` | **captured** |
| ElevenLabs speech | bytes, an MP3 | — | nothing to parse |
| ElevenLabs music | bytes, an MP3 | — | nothing to parse |
| ElevenLabs Voice Design | `api::elevenlabs::design::DesignReply` | `fixtures/elevenlabs/design.json` | hand-written |
| ElevenLabs keep a candidate | `CreatedVoice` | — | not exercised: it leaves a voice in the account |
| Veo model lookup | `api::veo::response::ModelInfo` | `fixtures/veo/model.json` | **captured** 2026-10-07 |
| Veo submit | `Submitted` | `fixtures/veo/submitted.json` | hand-written |
| Veo operation: running, done, failed, blocked by the filters | `Operation` | `fixtures/veo/{running,done,failed,blocked}.json` | hand-written; `blocked` from the fields `google-genai` reads (2026-10-11) |
| Gemini image interaction | `api::gemini::response::Interaction` | `fixtures/gemini/interaction.json` | **captured** 2026-10-07 (signature and picture cut down) |
| Gemini image model lookup | `api::veo::response::ModelInfo` | `fixtures/gemini/model.json` | hand-written, in the shape of `veo/model.json` |
| Gemini refusals | carried whole, never parsed | inline in `tests/live/veo.rs`, `tests/live/image.rs` | hand-written |
| LottieFiles search | `api::lottiefiles::Searched` | `fixtures/lottiefiles/search.json` | **captured** 2026-10-08 (three results) |
| Pixabay video search | `api::pixabay::Listing<VideoHit>` | `fixtures/pixabay/videos.json` | **captured** 2026-10-08 (three hits) |
| Pixabay image search | `api::pixabay::Listing<ImageHit>` | `fixtures/pixabay/images.json` | **captured** (two hits, their signed URLs' tokens replaced) |
| Claude streams: tool use, answer, refusal, overload | `api::anthropic::stream` + `claude::Assembler` | `fixtures/anthropic/*.sse` | hand-written |
| Anthropic refusals | carried whole, never parsed | inline in `tests/live/claude.rs` | hand-written |

`.github/scripts/tests/test_live_check_fixtures.py` holds this table to
`crates/providers/fixtures/`: a file with no row, or a row naming no file,
fails the `scripts` gate (#1226).

Hand-written JSON fixtures carry a `_provenance` field the parsers ignore;
hand-written streams open with `:` comment lines, which server-sent events
define as ignorable. Either way the file says what it is.
