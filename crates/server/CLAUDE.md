# crates/server

Rules for working on the web app's server. The root `CLAUDE.md` still applies,
and **docs/web.md** has the web side's settled shape; this file holds what only
matters here.

## The built-in assistant runs on the model each project picks

Decided in #705 (2026-10-03): Claude Sonnet 5.5 by default, Claude Opus 5.5,
Gemini 3.8 Flash or Gemini 3.5 Flash Lite, from a dropdown in the chat panel,
changeable mid-conversation. This reverses #540's *Claude Opus 5.5, never
downgraded* (2026-09-25): choice lets a user trade quality for credits. The
default was Gemini 3.8 Flash, on cost, until the maintainer moved it to Sonnet
on 2026-10-07 as the balance of quality and price; a new project starts there
and an existing one keeps what it is on. Each turn is charged at its own
model's rates; the conversation is kept in a vendor-neutral record so it
survives a switch (`crates/providers/src/chat`). The local MCP server and the
desktop app get no picker: there, the user's own client *is* the model.

None of this contradicts *MCP is a protocol, not a Claude feature*: choosing
models for **our own client** is a product decision, while the **tool surface**
it calls — the registry, every description, web MCP — still assumes nothing
about who is calling. A user's own Gemini or GPT pointed at web MCP gets
exactly what the built-in assistant gets.

## Its machine is shared with users

One of the operator's machines hosts the service (#527, #532): Docker Compose,
reached through a Cloudflare Tunnel. Users' renders and generations queue on
its cores behind a concurrency limit per kind of job, and a power cut is an
outage for them too — which is why the job queue recovers on restart and
backups leave the machine. Anything that executes a user's content there is
isolated and offline, with no opt-out (#594).
