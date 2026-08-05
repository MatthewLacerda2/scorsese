# What a generation costs, and why we can only estimate it

Generating a shot costs money. This page is where the numbers live, how they
are kept honest, and the one thing worth knowing before trusting any total
scorsese prints: **it is our arithmetic, not a bill.**

`scorsese prices` prints the table below from the code, with the day each
figure was last checked and how old that is.

## The table

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

## Nobody bills us back

**No provider scorsese talks to reports what a generation cost.** This is the
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

## In cents, all the way through

Whole US cents everywhere — the rate table, `estimated_cost_cents`, and the
`budget_cents` ceiling in [credentials.md](credentials.md). A total is a sum,
and a sum of floats is not the same number twice; a ceiling off by a rounding
error is a ceiling nobody can reason about.

It costs nothing here, either: every rate Veo publishes happens to be a whole
number of cents per second, so nothing rounds on the way in. If a vendor ever
prices something at a third of a cent, `Rate::cents_per_second` is the type that
has to change, and it should change loudly.
