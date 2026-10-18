# What a generation costs, and why we can only estimate it

Generating a shot, a still, a line of narration or a track of music costs money. This page is where the
numbers live, how they are kept honest, and the one thing worth knowing before
trusting any total scorsese prints: **it is our arithmetic, not a bill.**

`scorsese prices` prints the table below from the code, with the day each
figure was last checked and how old that is.

**Every table here is held to the code** (#1218). A test
(`crates/providers/tests/pricing/documented/`) reads each one — *Shots*, both
of them, *Stills*, *Narration*, *Music*, *The assistant* — with the day each section
says its figures were read, and compares every cell with `prices`, both ways:
a wrong figure, a missing row or an extra one fails `make gates` and names the
cell. So a repricing is one pull request that changes the code's table, this
page and the date together. The web app's multipliers over these costs are
held the same way to `docs/web.md`.

## Shots

Veo 3.1, US dollars per second of finished video. Paid tier — there is no free
tier for video at all.

| | 720p | 1080p | 4k |
| --- | --- | --- | --- |
| Veo 3.1 | $0.40 | $0.40 | $0.60 |
| Veo 3.1 Fast | $0.10 | $0.12 | $0.30 |
| Veo 3.1 Lite | $0.05 | $0.08 | *not sold* |

Last checked against
[Google's pricing page](https://ai.google.dev/gemini-api/docs/pricing) on
**2026-10-08** (the page itself says it was updated 2026-10-07).

**Every row is something scorsese offers.** Scorsese offers what Google sells,
no more and no less (#891), so that page is the checklist: a row here that a
shot's brief cannot ask for, or a choice in the brief with no row, is
something that was not maintained. The one deliberate gap is video extension,
which is a different kind of request rather than another row (the format
reference's *What a generated video asks for* has why).

So every shot scorsese can ask for, at the lengths Veo makes. 1080p and 4k are
only generated at eight seconds:

| | 4s | 6s | 8s |
| --- | --- | --- | --- |
| Standard, 720p | $1.60 | $2.40 | $3.20 |
| Standard, 1080p | — | — | $3.20 |
| Standard, 4k | — | — | $4.80 |
| Fast, 720p | $0.40 | $0.60 | $0.80 |
| Fast, 1080p | — | — | $0.96 |
| Fast, 4k | — | — | $2.40 |
| Lite, 720p | $0.20 | $0.30 | $0.40 |
| Lite, 1080p | — | — | $0.64 |

A twenty-shot cut at the default — Fast, 1080p, eight seconds — is about
**$19**. Sketching that same cut costs nothing at all, because a sketch renders
as a slug card and never reaches a provider.

### Three things the table holds beyond the numbers

**A date per figure.** `Rate` has two fields and both are required, so a price
cannot be added without saying when somebody read it. A price with no date is a
price nobody can audit.

**A staleness signal, never a gate.** Past 180 days `scorsese prices` marks the
row and says so; CI appends that output to its job summary. It blocks nothing —
per CLAUDE.md's gates-versus-signals rule, and because a figure being old does
not make it wrong. 180 days is a judgement, not a rule: vendors reprice on no
schedule.

**Room for what nobody sells.** Lite has no 4k, so the table is a *list of
rows* rather than a grid — the row simply is not there, and asking for it
answers `Unpriced` rather than a number. A grid would need something in that
cell, and whatever went there would be a price for something that cannot be
bought.

The artifact being audited is Google's price list, and a list missing rows is
one nobody can tick through — which is the other half of offering everything
it sells: there is no row kept only for the audit, and none missing from it.

## Stills

Every still is drawn by **Nano Banana 2.1** (`gemini-nano-banana-2.1`), the
one image model scorsese uses (#1308) — there is no model to choose. US dollars
per **picture**, at the size it is drawn. Standard tier — it has no free tier.
The batch tier is half of every figure below; see *Stills in a batch*.

| on the wire | 1K | 2K | 4K | input, per 1M tokens |
| --- | --- | --- | --- | --- |
| `gemini-nano-banana-2.1` | $0.0336 | $0.0504 | $0.113 | $1.50 |

Last checked against
[Google's pricing page](https://ai.google.dev/gemini-api/docs/pricing) (last
updated 2026-10-14) on **2026-10-15**. Its 1K and 2K figures are the
[image-generation page](https://ai.google.dev/gemini-api/docs/image-generation)'s
1,120 and 1,680 output tokens at $30 a million; its 4K figure is not — that
page says 2,520 tokens, which would be $0.0756. The billing page's $0.113 is
the one kept, so an estimate that errs, errs high.

### Stills in a batch

Every still can instead be ordered through Google's
[Batch API](https://ai.google.dev/gemini-api/docs/batch-mode) at **half the
price**, answered **within 24 hours** (#894). Google's page lists a batch
column beside each standard one, picture and input alike, and every batch
figure is half its neighbour — so the table is not copied twice:
`prices::gemini::BATCH_PERCENT` is `50`, read on the same day as the rows, and
`prices::image_in_batch` is the standard arithmetic at that share, rounded up
to the cent once per still. The one exception is 4K, listed at $0.0567 against
half of $0.113 — two hundredths of a cent, which no estimate rounded to the
cent can show. The day a batch figure is off by more, that constant becomes a
table.

How to use it: **`generate` with `batch`** (MCP, local or the web's), or
**`scorsese generate --batch`** — and on the web, the assistant's confirmation
box, which shows both prices as two yeses (#947, `docs/web.md`). It quotes the stills at the batch rate under its own kind of
spending, so a token agreed for *now* is never spent on a batch or the other
way round. Each still goes `queued` with the batch job's name as its
`operation`; the next `generate` (or `--collect`, which never spends) asks
after the job and lands each picture where a still drawn now would have, at
the same brief hash. Batch covers `generateContent` only, so **stills only**:
a batch with a shot or a line to send is refused, naming them. A batch that
fails or expires unfinished bills nothing and puts its stills back to sketches.

**When it is offered.** The default stays *now*. A quote for now whose stills
come to a dollar or more (`quote::OFFER_FROM_CENTS`) also prices them in a
batch, side by side — *"Stills: $X now · $Y in a batch, ready within 24
hours"* — and the tool's description tells an assistant to put that choice to
whoever is paying, never to pick the wait for them. Under a dollar the saving
is pennies, not worth a day or the question, so it is left out.

The picture is priced like a shot: by a choice the request fixes — its size
— not by anything the vendor decides after the fact. The page states it per
million output tokens ($30) and then per picture; the per-picture figure is the
one copied, because it is the one a person checks.

**The default is 2K: $0.0504, about six cents** with its prompt. A
400-character prompt and two references are 2,340 input tokens, 0.35¢, and a
thousand thinking tokens at $7.50 a million another 0.75¢: about **6.1¢**. 2K is
what covers a 1080p frame with room left to push into or pan across; a 1K still
($0.0336, the cheapest picture there is, and the way to test a prompt) is
enlarged before it moves.

### What the estimate counts, and what it does not

**Counted:** the picture; every reference picture, at the **1,120 input
tokens** a Gemini 3 model reads one as at its default media resolution; and the
prompt, at one token per four characters (the vendor's own rule of thumb —
the prompt is never tokenised here). A reference is about **0.17¢**, so even
fourteen of them — ten objects and four characters, the most a still takes —
add under three cents.

**Not counted:** any thinking or words the model writes back — $7.50 per
million, at `medium` unless asked. It is the one part no request can fix in
advance, and a thousand tokens of it is under a cent.

None of these land on whole cents, so the table is kept in micro-dollars and
the total **rounds up** once, for the reason narration does: a ceiling crossed a
fraction of a cent at a time is not a ceiling.

### The scale

A still at the default is **6 cents**; eight seconds of Veo at its default is
**96 cents** — one shot buys sixteen stills, and a still is reused where a shot
rarely is. That is the reason the kind exists.

## Narration

ElevenLabs text-to-speech, US cents per **thousand characters of input text**.

| model | on the wire | per 1000 characters | a 200-character line |
| --- | --- | --- | --- |
| `expressive` | `eleven_v4` | $0.08 | $0.02 |
| `standard` | `eleven_multilingual_v2` | $0.08 | $0.02 |
| `fast` | `eleven_flash_v2_5` | $0.04 | $0.01 |

`fast` is the default, and this table is the reason: `standard` and
`expressive` cost the same, so the only choice with money in it is *fast or
not*, and a line nobody configured takes the cheap side of it.

Last checked against
[ElevenLabs' API pricing page](https://elevenlabs.io/pricing/api) on
**2026-10-14**, read from the page's own HTML: the same rate on every plan.
The 2026-08-05 reading was $0.10 and $0.05, so every quote and web charge made
between the vendor's cut and this correction overstated narration by a quarter
(#1240).

**Which model the vendor recommends** (its *How to choose the right model*
page, read 2026-10-14): `eleven_v4` for quality and for "professional content,
audiobooks, and video narration"; `eleven_v4_turbo` or Flash for low latency.
It calls v4 a net upgrade over v3. So `expressive` is the one to name when the
reading matters. `fast` stays the default on price alone, the argument above,
which is scorsese's and not the vendor's.

**v3 is gone** (#1248). `expressive` was `eleven_v3` until schema v49, and v4
briefly had a name of its own, `natural`. The maintainer moved `expressive` to
v4 knowing the vendor's warning that v4 can sound substantially different, and
`natural` retired, since two names for one model is a choice with no
difference. v3 costs what v4 does, so no rate changed.

The Turbo models are absent, and that is not the same omission as Veo's
*not sold* cell. Veo's per-second table keeps a cell for what Google does not
sell (Lite at 4k) so the vendor's page can be ticked through row by row.

- **v2 and v2.5 Turbo** are rows the **vendor** tells people not to use: its
  own documentation recommends Flash over them in every case, so offering both
  would be offering a choice with a right answer.
- **v4 Turbo** ($0.04, Flash's price) is sold for real-time dialogue: the
  vendor documents it on the Text to Dialogue websocket, and documents only
  `eleven_v4` on the text-to-speech endpoint scorsese calls. scorsese renders
  offline, so its latency buys nothing here. If the vendor documents it on
  *Create speech with timing*, it becomes a candidate for a cheap expressive
  model. v3 Conversational is the same case, and so is v3 itself: no name
  speaks it since `expressive` moved to v4.

### Narration is priced before the call, not after it

This is the one real difference from video, and it is a pleasant one. A shot is
priced by its length, which the request fixes. A line is priced by its text,
which is **already in the document** — so the figure is exact and known before
anything is sent.

It is still called an estimate, for two reasons that survive the arithmetic
being exact:

- The rate above is a page somebody copied.
- A handful of Voice Library voices carry a **credit multiplier** and cost more
  per character than the base rate — and Library voices narrate on any paid
  plan, so a line can land on one. scorsese ignores this deliberately —
  modelling a credit system to move an advisory counter by a few cents is not
  worth what it would cost to keep true. So the estimate can be a little low
  for those voices.

### It rounds up

Unlike video, this does not land on whole cents: 137 characters at 8¢ per
thousand is 1.096¢. Every fraction rounds **up**.

Rounding down would let a run slip past the ceiling a fraction of a cent at a
time, and a ceiling that can be crossed a little at a time is not a ceiling.
Rounding up costs at most a cent per line and is never the wrong side of the
number somebody set.

### The scale is worth holding on to

A 200-character line of narration is **one or two cents**. Eight seconds of
Veo at the default is **96 cents** — sixty to a hundred times more. A cut with
twenty narrated lines and twenty shots costs about $19.40, and $19 of that is
the picture.

Which is why the two are quoted on separate lines and never summed into one
per-item average: an average across them would describe nothing that exists.


## Music

ElevenLabs Music, US cents per **minute of music asked for** (a track's
`music.seconds`), the same on every model.

| model | on the wire | per minute | a 40-second track |
| --- | --- | --- | --- |
| `music` | `music_v2_5` | $0.15 | $0.10 |

Last checked against
[ElevenLabs' API pricing page](https://elevenlabs.io/pricing/api) on
**2026-10-16**: one rate, $0.15 a minute, in every row of its Music table, and
*Commercial use licensing on Starter+ plans* on the product card. The page's
FAQ also says music is "billed per generation", which reads against its own
table; the table is what is priced here. `music_v2_5` is the newest model, the
one the vendor's models page calls its most advanced (read 2026-10-16), and
the only one scorsese composes with — `music_v1` is "outclassed" in the
vendor's words, and no model costs more than another, so there is no choice
worth offering.

Like a shot, a track is priced by its length, which the request fixes: the
figure is known before anything is sent, rounded **up** to the cent. A
`synth_audio` recipe costs nothing at all ([recipes.md](recipes.md),
`guide recipes`), which is why it stays the first choice for a score.
## The assistant

The hosted web app's assistant (#540) runs on the model each project chooses
(#705) — Claude Sonnet 5.5, Opus 5.5 (the default) or Fable 5.1 (#1444),
since #1220 cut Gemini from the picker — priced per **million tokens**, each kind at its own rate —
`prices::chat`:

| model | input | output | cache write, 5 min | cache write, 1 h | cache read | checked |
| --- | --- | --- | --- | --- | --- | --- |
| `claude-fable-5-1` | $10 | $50 | $12.50 | $20 | $0.25 | 2026-10-17 |
| `claude-opus-5-5` | $4 | $20 | $5 | $8 | $0.20 | 2026-10-14 |
| `claude-sonnet-5-5` | $2 | $10 | $2.50 | $4 | $0.10 | 2026-10-14 |

Read off [Anthropic's pricing page](https://platform.claude.com/docs/en/about-claude/pricing)
and prompt-caching page.
Standard tier and global routing only — fast mode, the batch discount,
Priority and US-only inference are rows scorsese never pays, so they are not
here. Things that look like typos and are not:

- **Cache reads are 0.05× input on Opus and Sonnet**, not the usual 0.1×:
  $0.20 on Opus and $0.10 on Sonnet (Anthropic's page, 2026-10-14). Until that
  day the table had Sonnet at the usual 0.1×, $0.20, and charged its cache
  reads double.
- **And 0.025× on Fable 5.1**: $0.25 against $10 input (Anthropic's page,
  2026-10-17: *"a cache hit costs 2.5% of the standard input price"*).
- **A row may carry the day it takes effect** (#718), and
  `prices::chat::rate` is asked for a model *on a date* — the latest row not
  after it. No row is dated today (the one that was, Gemini 3.8 Flash's
  2027-01-01 doubling, left with Gemini); the server passes today, in UTC, and
  the table itself never reads a clock. A vendor's announced change goes in as
  a dated row the day it is announced, not as an edit somebody has to
  remember.
- **Thinking is output**: Claude counts it in `output_tokens`, charged at the
  output rate.

**This is the one table whose total is not a guess about quantity.** Every
response carries a usage block counting the tokens it was billed for — plain
input, output, both kinds of cache write, cache reads — so the cost of a call is
the vendor's own count times this table. The rate is still a page somebody
copied, which is why the date is here; the count is not an estimate.

**Cache savings are the user's.** A cache read is charged at the cache-read
rate, never as full input (the maintainer, 2026-10-03).

**What a turn costs is mostly how much it re-reads.** Every call of an
assistant turn resends the whole conversation, so the server caches it
(`docs/web.md`, *Assistant turns*): on Claude the tools and system prompt for
an hour, shared by every user, the conversation's tail for five minutes. A
cache read is a twentieth of input on Opus and Sonnet and a fortieth on
Fable, which is why a long
turn is cheap per call, why anything that edits earlier messages would be
expensive, and why the first turn after a change of model costs more. The
operator's per-turn cap bounds the rest.

**In micro-dollars, rounded up once per call.** A vendor's page may print a
fraction of a cent per million (Gemini's did), so the table is kept in
micro-dollars per million tokens, a
call is summed in those, and divided once, upwards — `prices::chat::Usage::micros`.
The hosted server's ledger is in micro-dollars for exactly this reason
(`docs/web.md`, *Money*).

## Nobody bills us back

**No provider scorsese talks to reports what a generation cost** — in money.
The assistant's token counts (above) come closest, and they are still counts. This is the
part worth reading twice, because every figure downstream inherits it.

A finished Veo operation is this, whole:

```json
{ "done": true,
  "response": { "generateVideoResponse": { "generatedSamples": [
    { "video": { "uri": "https://generativelanguage.googleapis.com/v1beta/files/…" } } ] } } }
```

A URI. No amount, no usage record, no quantity of anything. Google's text
models do return a `usageMetadata` block, and even that is a count of tokens
rather than money — you multiply it by a published rate yourself, which is the
same estimate under a better name. Veo's operation does not carry one.

Actual spend exists in exactly one place: Google Cloud Billing, for the project
the key belongs to. It arrives about a day late, it is aggregated per SKU
rather than per request, and nothing in it can be attributed back to the shot
that caused it. There is no endpoint that closes that gap, and adding a Cloud
Billing dependency to read a lagging project-wide total would not close it
either.

### What follows from that

The asset field is called **`estimated_cost_cents`**, not `cost_cents`. It was
the shorter name for two days and the rename is why this document exists: these
are summed into a project total shown to somebody deciding whether to spend
more, and a number labelled *what this cost* that means *what we worked out it
would cost* is the kind of quiet wrongness that only ever gets noticed by the
person who already paid.

Wherever a total is printed, it says the same thing. The estimate is right when
the table is right, and the table is a page somebody copied.

### Every take is counted

An asset's `estimated_cost_cents` is what **its current file** cost. A shot that
came back wrong and was bought again carries the second take's figure; the first
take is still in `generated/` and was still paid for. So a project's spending is
not summed over its assets: each run writes one line into **`generations.json`**,
beside `project.json`, the moment it sends a brief — a shot submitted, a still
drawn or ordered in a batch, a line spoken, a track composed — with the asset,
the brief's hash, the estimate and the date. The `budget_cents` ceiling and every
printed project total read that file, and the takes no asset shows any more are
named on a line of their own ("1 superseded take, $0.40"), so a retry reads as a
retry. A shot Veo turns down after it was sent is marked `refused` there and not
counted: Google does not charge for it.

It is a sidecar, like `designed-voices.json`, so it is no format change, and it
is not under `cache/`: deleting it forgets money that was spent. A project from
before the file existed begins it with its assets' current figures (`seeded`
lines): **history before the ledger is the current takes only.** The hosted web
app does not use it — its spending is the credit ledger in Postgres
([web.md](web.md)).

## Checking the clients, not the prices

This page keeps the *rates* honest. Whether the vendors still answer in the
shape our clients read is a different question, asked by a person with real
calls that cost a few cents — [live-check.md](live-check.md).

## In cents, all the way through

Whole US cents everywhere on a local project — the rate tables,
`estimated_cost_cents`, and the `budget_cents` ceiling in
[credentials.md](credentials.md). A total is a sum, and a sum of floats is not
the same number twice; a ceiling off by a rounding error is a ceiling nobody
can reason about.

The hosted server's credit ledger is the one place finer than a cent — integer
**micro-dollars**, because an assistant call costs fractions of one. It takes
a quote's cents at its boundary and multiplies; nothing here changes for it.

It costs nothing here, either: every rate Veo publishes happens to be a whole
number of cents per second, so nothing rounds on the way in. If a vendor ever
prices something at a third of a cent, `Rate::cents_per_second` is the type that
has to change, and it should change loudly.
