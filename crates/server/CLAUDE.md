# crates/server

Rules for working on the web app's server. The root `CLAUDE.md` still applies,
and **docs/web.md** has the web side's settled shape; this file holds what only
matters here.

## The built-in assistant runs on the model each project picks

Claude Opus 5.5 by default, or Claude Sonnet 5.5 or Claude Fable 5.1, from a
dropdown in the chat panel, changeable mid-conversation (#705; the list and the
default as of #1444). The choice lets a user trade quality for credits; Gemini
was offered too until the maintainer cut it as not good enough at the job
(#1220). `Model` in `crates/providers/src/chat/model.rs` is the list. Gemini
still makes images and Veo video. Each turn is charged at its own model's
rates, and the conversation is kept in a vendor-neutral record so it survives
a switch (`crates/providers/src/chat`). The local MCP server and the desktop
app get no picker: there, the user's own client *is* the model.

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
isolated and offline, with no opt-out (decided in #594, built in #778).
