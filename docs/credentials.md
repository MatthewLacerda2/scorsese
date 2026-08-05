# Keys, and the ceiling on what they may spend

Two providers cost money — Gemini (for Veo video) and ElevenLabs (for
narration) — and both need a key. This page is where a key comes from, in one
order, for every way scorsese is run.

## The order

1. **The environment.** `GEMINI_API_KEY` and `ELEVENLABS_API_KEY`, exported in
   the shell or written in a `.env` at the root of a checkout. A variable that
   is set wins; a `.env` fills gaps and never overrides one.
2. **The settings file**, per machine, at the platform's config location:

   | | |
   | --- | --- |
   | Linux / BSD | `$XDG_CONFIG_HOME/scorsese/settings.json`, else `~/.config/scorsese/settings.json` |
   | macOS | `~/Library/Application Support/scorsese/settings.json` |
   | Windows | `%APPDATA%\scorsese\settings.json` |

That is the whole of it. `scorsese settings` prints where the file is, which
`.env` was found, and which keys resolved from where — and **never prints a
key**, not masked and not truncated: a masked key is still four characters of a
credential in a terminal that scrolls into a log, and the question anybody
actually has is *did it find one, and from where*.

The order is that way round because an exported variable is somebody being
deliberate right now, and a settings file is what they decided once. What it
prevents is two sources of truth — a key saved in the window that `scorsese
generate` cannot see, so the GUI works and the terminal does not. The MCP server
is the case that settles it: it runs headless, will never see a settings screen,
and has to find the key the screen just saved.

`.env` is the **development** path and only that. It is gitignored, documented
by `.env.example` beside it, and it stops existing the moment somebody installs
a build. The settings file is what a shipped program has, which is why the
window writes there and why the resolver reads both.

**A key never goes near `project.json`.** A project directory is promised to
survive `scp -r` to another machine, and a credential inside one would travel
with it.

## A key can be perfectly valid and still be refused

**ElevenLabs keys carry permissions**, chosen when the key is created, and a key
missing one is refused with a `401` — the same status an absent or wrong key
gets. The two are fixed in opposite places, so scorsese reads the refusal body
rather than the status code and reports them differently:

| What the vendor says | What it means | Where to fix it |
| --- | --- | --- |
| `missing_permissions` | The key is fine and lacks a named scope, e.g. `voices_read` | The ElevenLabs dashboard, under Profile → API Keys |
| anything else at `401` | No key, or the wrong one | `.env`, or the settings file below |

The refusal names the exact permission, and scorsese passes that sentence
through whole rather than summarising it. Telling somebody to check `.env` when
their key is already correct is the most expensive kind of wrong error message:
it is confidently specific, and it points the wrong way.

Gemini has no equivalent — a Gemini key either works or it does not.

## The settings file

```json
{
  "gemini_api_key": "…",
  "elevenlabs_api_key": "…",
  "budget_cents": 5000
}
```

Written owner-only on Unix — set on the file rather than left to the umask,
because this holds credentials and a default that happens to be safe on one
machine is not a guarantee on the next. Every field is optional; a machine
nobody has configured has an empty file or none at all, and that is not an
error.

## The ceiling

`budget_cents` is the most one run may spend, and it is refused **even under
`--yes`**. That is the point rather than an oversight: `--yes` says nobody is
there to be asked, which is exactly the situation a ceiling exists for. An agent
working overnight cannot be asked a question, so it is given a number it may not
cross. A limit a flag can wave through is not a limit.

A refusal says what the run would have cost, what has been spent, and by how
much it went over — enough to decide with, rather than a wall.

**Absent means no ceiling**, which is what a fresh install has. A limit nobody
chose is not a limit, and inventing one would refuse the first honest run
somebody made.

Money is counted in whole US cents throughout. A project's total is a sum of
what each generated asset recorded it cost, so it is correct by construction and
stays correct when a shot is deleted — a running tally kept somewhere else would
not.
