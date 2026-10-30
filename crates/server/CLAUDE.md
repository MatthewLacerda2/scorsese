# crates/server

Rules for working on the web app's server. The root `CLAUDE.md` still applies,
and **docs/web.md** has the web side's settled shape; this file holds what only
matters here.

## The built-in assistant runs on the model each project picks

Claude Opus 5.5 by default, or Claude Sonnet 5.5 or Claude Fable 5.1, from a
dropdown in the chat panel, changeable mid-conversation. The choice lets a user
trade quality for credits. `Model` in `crates/providers/src/chat/model.rs` is
the list. Gemini makes images and Veo video, but neither runs the assistant. Each turn is charged at its own model's
rates, and the conversation is kept in a vendor-neutral record so it survives
a switch (`crates/providers/src/chat`). The local MCP server and the desktop
app get no picker: there, the user's own client *is* the model.

Choosing models for **our own client** is a product decision, while the **tool surface**
it calls — the registry, every description, web MCP — still assumes nothing
about who is calling. A user's own Gemini or GPT pointed at web MCP gets
exactly what the built-in assistant gets.

## How the assistant talks

**Lovable is the reference**: it says little and gets things right first time,
from one big prompt planned elsewhere. So the assistant works from the whole
plan, asks for everything at once, and writes a line where it would have
written a paragraph. Its words are its system prompt
(`crates/server/src/assistant/prompt.rs`) and the tool descriptions.

## Its machine is shared with users

One of the operator's machines hosts the service: Docker Compose,
reached through a Cloudflare Tunnel. Users' renders and generations queue on
its cores behind a concurrency limit per kind of job, one account's share of
it is bounded (`config::PerUser`: turns at once, batched stills, storage; a
new shared capacity gets its per-user bound too), and a power cut is an
outage for them too, so the job queue recovers on restart and backups leave the
machine. Anything that executes a user's content there is isolated and offline,
with no opt-out.
