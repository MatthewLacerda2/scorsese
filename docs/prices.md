# What a generation costs, and why we can only estimate it

Generating a shot, a still or a line of narration costs money. This page is where the
numbers live, how they are kept honest, and the one thing worth knowing before
trusting any total scorsese prints: **it is our arithmetic, not a bill.**

`scorsese prices` prints the table below from the code, with the day each
figure was last checked and how old that is.

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
**2026-08-04**.

So the shots scorsese can actually ask for, at the three lengths Veo makes:

| | 4s | 6s | 8s |
| --- | --- | --- | --- |
| Fast, 720p | $0.40 | $0.60 | $0.80 |
| Fast, 1080p | $0.48 | $0.72 | $0.96 |
| Lite, 720p | $0.20 | $0.30 | $0.40 |
| Lite, 1080p | $0.32 | $0.48 | $0.64 |

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

Rows scorsese does not offer are in the table anyway — Standard, and 4k —
marked as not offered. The artifact being audited is Google's price list, and a
list missing rows is one nobody can tick through.

## Stills

Gemini 3.1 Flash Image ("Nano Banana 2") and Flash Lite Image, US dollars per
**picture**, at the size it is drawn. Standard tier — neither has a free tier,
and scorsese does not use the half-price batch tier, which answers in hours.

| model | on the wire | 0.5K | 1K | 2K | 4K | input, per 1M tokens |
| --- | --- | --- | --- | --- | --- | --- |
| `flash` | `gemini-3.1-flash-image` | $0.045 | $0.067 | $0.101 | $0.151 | $0.50 |
| `lite` | `gemini-3.1-flash-lite-image` | *not sold* | $0.0336 | *not sold* | *not sold* | $0.25 |

Last checked against
[Google's pricing page](https://ai.google.dev/gemini-api/docs/pricing) on
**2026-10-02** (the page itself says it was updated 2026-10-01).

The picture is priced like a shot: by a choice the request fixes — its size —
not by anything the vendor decides after the fact. The page states it as $60
per million output tokens and then per picture; the per-picture figure is the
one copied, because it is the one a person checks.

**The default is `flash` at 2K: $0.101, about eleven cents** with its prompt.
2K is what covers a 1080p frame with room left to push into or pan across; a
1K still is enlarged before it moves. `lite` draws 1K only, at a third of that,
and is the choice for a picture that matters less than the money.

### What the estimate counts, and what it does not

**Counted:** the picture; every reference picture, at the **1,120 input
tokens** a Gemini 3 model reads one as at its default media resolution; and the
prompt, at one token per four characters (the vendor's own rule of thumb —
the prompt is never tokenised here). A reference is about **0.06¢** on `flash`,
so fourteen of them add under a cent.

**Not counted:** any thinking or words the model writes back, billed at $3 per
million on `flash`. scorsese leaves thinking at its `minimal` default, so this
is a fraction of a cent — and it is the one part no request can fix in advance.

None of these land on whole cents, so the table is kept in micro-dollars and
the total **rounds up** once, for the reason narration does: a ceiling crossed a
fraction of a cent at a time is not a ceiling.

### The scale

A still at the default is **11 cents**; eight seconds of Veo at its default is
**96 cents** — one shot buys eight or nine stills, and a still is reused where a
shot rarely is. That is the reason the kind exists.

## Narration

ElevenLabs text-to-speech, US cents per **thousand characters of input text**.

| model | on the wire | per 1000 characters | a 200-character line |
| --- | --- | --- | --- |
| `expressive` | `eleven_v3` | $0.10 | $0.02 |
| `standard` | `eleven_multilingual_v2` | $0.10 | $0.02 |
| `fast` | `eleven_flash_v2_5` | $0.05 | $0.01 |

`fast` is the default, and this table is the reason: `standard` and
`expressive` cost the same, so the only choice with money in it is *fast or
not*, and a line nobody configured takes the cheap side of it.

Last checked against
[ElevenLabs' pricing page](https://elevenlabs.io/pricing) on **2026-08-05**.

The Turbo models are absent, and that is not the same omission as Veo's
`Standard` row. Veo's table keeps a tier scorsese does not offer so the vendor's
page can be ticked through row by row. Turbo is a row the **vendor** tells
people not to use: its own documentation recommends Flash over Turbo in every
case, so offering both would be offering a choice with a right answer.

### Narration is priced before the call, not after it

This is the one real difference from video, and it is a pleasant one. A shot is
priced by its length, which the request fixes. A line is priced by its text,
which is **already in the document** — so the figure is exact and known before
anything is sent.

It is still called an estimate, for two reasons that survive the arithmetic
being exact:

- The rate above is a page somebody copied.
- A handful of Voice Library voices carry a **credit multiplier** and cost more
  per character than the base rate. scorsese ignores this deliberately —
  modelling a credit system to move an advisory counter by a few cents is not
  worth what it would cost to keep true. So the estimate can be a little low
  for those voices.

### It rounds up

Unlike video, this does not land on whole cents: 137 characters at 10¢ per
thousand is 1.37¢. Every fraction rounds **up**.

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

## The assistant

The hosted web app's assistant (#540) runs on the model each project chooses
(#705), priced per **million tokens**, each kind at its own rate —
`prices::chat`:

| model | input | output | cache write, 5 min | cache write, 1 h | cache read | checked |
| --- | --- | --- | --- | --- | --- | --- |
| `claude-opus-5-5` | $4 | $20 | $5 | $8 | $0.20 | 2026-10-03 |
| `claude-sonnet-5-5` | $2 | $10 | $2.50 | $4 | $0.20 | 2026-10-03 |
| `gemini-3.8-flash` | $0.75 | $3.75 | — | — | $0.075 | 2026-10-03 |
| `gemini-3.8-flash` from 2027-01-01 | $1.50 | $7.50 | — | — | $0.15 | 2026-10-03 |
| `gemini-3.5-flash-lite` | $0.30 | $2.50 | — | — | (none sold) | 2026-10-03 |

Read off [Anthropic's pricing page](https://platform.claude.com/docs/en/about-claude/pricing)
and prompt-caching page, and [Google's](https://ai.google.dev/gemini-api/docs/pricing).
Standard tier and global routing only — fast mode, the batch discount,
Priority and US-only inference are rows scorsese never pays, so they are not
here. Things that look like typos and are not:

- **Opus 5.5's cache reads are 0.05× input**; Sonnet 5.5's are the usual 0.1×,
  which happens to land on the same $0.20.
- **Gemini 3.8 Flash's price is introductory and doubles on 2027-01-01**
  ($1.50 / $7.50 / $0.15 cached), so it has two rows. A row may carry the day
  it takes effect, and `prices::chat::rate` is asked for a model *on a date* —
  the latest row not after it (#718). The server passes today, in UTC; the
  table itself never reads a clock. A vendor's announced change goes in as a
  dated row the day it is announced, not as an edit somebody has to remember.
- **Gemini writes no cache, so its write columns are empty.** 3.8 Flash caches
  implicitly and reads at a tenth of input; 3.5 Flash Lite has no caching sold
  and is not on Google's implicit-caching list, so its cached column is its
  input rate — a cached count, should one appear, is billed as the input it is.
- **Thinking is output** on both vendors: Claude counts it in `output_tokens`,
  Gemini reports `thoughtsTokenCount` beside `candidatesTokenCount`, and both
  are charged at the output rate.

**This is the one table whose total is not a guess about quantity.** Every
response carries a usage block counting the tokens it was billed for — plain
input, output, both kinds of cache write, cache reads — so the cost of a call is
the vendor's own count times this table. The rate is still a page somebody
copied, which is why the date is here; the count is not an estimate.

**Cache savings are the user's.** A cache read is charged at the cache-read
rate, never as full input (the maintainer, 2026-10-03): Gemini's prompt count
includes its cached part, so the cached tokens are taken out of input and
priced as reads.

**What a turn costs is mostly how much it re-reads.** Every call of an
assistant turn resends the whole conversation, so the server caches it
(`docs/web.md`, *Assistant turns*): on Claude the tools and system prompt for
an hour, shared by every user, the conversation's tail for five minutes; on
Gemini 3.8 Flash whatever recent prefix Google's implicit cache holds. A cache
read is a twentieth (Opus) or a tenth (the rest) of input, which is why a long
turn is cheap per call, why anything that edits earlier messages would be
expensive, and why the first turn after a change of model costs more. The
operator's per-turn cap bounds the rest.

**In micro-dollars, rounded up once per call.** One Gemini figure is half a
cent per million, so the table is kept in micro-dollars per million tokens, a
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
