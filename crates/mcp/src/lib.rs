//! # scorsese-mcp — the MCP server
//!
//! Responsibility: exposing scorsese over the Model Context Protocol, as a
//! thin wrapper over the same library logic the CLI uses. If a tool here needs
//! code the CLI does not share, that code is in the wrong place — push it down
//! into `core`, `render` or `providers`.
//!
//! **MCP is a protocol, not a Claude feature.** This server speaks it to
//! whatever client is on the other end; Gemini, GPT and anything else that
//! speaks MCP get the same tools, the same way an HTTP API does not care
//! whether a browser, a phone or curl is calling. Claude is who this is
//! developed and tested against, not a dependency, and nothing here may assume
//! otherwise.
//!
//! Boundary: protocol handling only. No editing logic of its own, no display,
//! no direct ffmpeg or provider calls — everything goes through the lower
//! crates.
//!
//! ## Hand-rolled, deliberately
//!
//! MCP over stdio is JSON-RPC 2.0, one message per line, and the part of it a
//! tools-only server answers — `initialize`, `tools/list`, `tools/call`,
//! `ping`, and the cancel and progress notifications (#647, #700) — is small.
//! [`protocol`] is what a message means, written so that neither transport
//! owns it: this binary reads lines; `scorsese-server` answers Streamable
//! HTTP `POST`s (#539) on the axum stack it already runs.
//!
//! The official Rust SDK, `rmcp`, was weighed for this twice (#539) and then
//! measured (#746), once the first reason recorded against it — "a beta" —
//! had gone stale. It is still not taken, for reasons that do not go stale
//! with a version number:
//!
//! - **It is async-only.** A protocol that is one stream read in order would
//!   carry a tokio runtime, and the stdio binary its tree: a one-tool server
//!   built both ways was 69 crates and 2.6 MB against 23 and 0.6 MB.
//! - **It would replace the framing and nothing else.** The tools are a
//!   registry of trait objects that run with a [`Context`] — a cancel, the
//!   session's renders, a progress callback — so its tool macros do not fit,
//!   and a hand-written handler would keep this crate's listing and parsing.
//! - **What agents read would change unless fought.** Its schemas bring back
//!   what `tools::args` strips (`$schema`, `format`, nullable optionals), and
//!   its argument errors read `failed to deserialize parameters: invalid
//!   type: string "soon", expected f64` where this server says `` `start` has
//!   to be a number, not "soon" ``.
//! - **It moves faster than the protocol.** Three major versions in 2026
//!   (March, June, July) — each a migration on its schedule — in months
//!   where the specification itself shipped one revision.
//!
//! `cargo deny` cleared it, so that is not a reason. The cost of keeping this
//! is tracking the specification by hand: [`PROTOCOLS`](protocol::PROTOCOLS)
//! stops at 2025-06-18, and a client asking for 2025-11-25 or 2026-07-28 is
//! answered with that and decides whether it can hold it. Nothing in either
//! is required of a tools-only server that negotiates down, and 2026-07-28's
//! headline — no handshake, the protocol version and client named in each
//! request's `_meta` — is the shape this server already has, since it holds
//! no session and answers a call that never said `initialize`.
//!
//! **What reopens it**, and this paragraph is the note to revise:
//!
//! - a client this project cares about stops accepting every revision in
//!   `PROTOCOLS` — then the choice is speaking the new one by hand (per-request
//!   `_meta`, its HTTP headers: small) or taking the SDK, weighed again;
//! - a tool needs the server to ask the client something — elicitation,
//!   sampling, tasks — which is a request in the other direction and a
//!   different shape of server;
//! - web MCP needs resumable streams or OAuth discovery (#533's later
//!   issue), which is the transport the SDK exists to get right.
//!
//! ## Every tool is described, and that is a gate
//!
//! A tool's description is the entire interface a client has to it, and an
//! undescribed tool is a capability that exists and cannot be found — nothing
//! fails, the assistant on the other end simply never calls it. `tests/` walks
//! the registry and fails on a tool or an argument that says nothing about
//! itself. The same gate already covers the CLI's `--help` and the property
//! table in `docs/project-format.md`.
//!
//! ## The page is generated from the registry, not kept in step with it
//!
//! `docs/mcp.md`'s table of tools used to be typed by hand, which made it a
//! second copy of facts this crate already holds — and a tool added without a
//! row was a capability nobody reading that page could learn about, failing
//! nothing. [`tool_table`] renders it from [`registry`] instead, and
//! `tests/table.rs` fails when the checked-in page is not what it produces.
//! `make mcp-table` rewrites it.
//!
//! That is why [`Tool::costs`] exists at all. What a call spends was a fact
//! living only in markdown; it belongs on the tool for the same reason its
//! description does, and a client can be shown it there.
//!
//! ## Stateless
//!
//! Every tool takes the project directory it works on. There is no server-side
//! "open project" to go stale, so a client may crash, reconnect, or run two
//! conversations against one project without anything getting out of step.
//!
//! The one thing a session does hold is its renders (#700). A render runs in
//! the background so an assistant can ask how far it has got, and a running
//! thread has to belong to something; it belongs to the `serve` that started
//! it, and ends with it. Nothing about the project is held there — a render
//! reads the project when it is asked for, as every other tool does.
//!
//! ## Paid tools quote first
//!
//! A tool that costs money answers its first call with a quote and a token,
//! and spends only when called again with that token — one rule, with no
//! one-step path even here, where the key is the operator's own. The tokens
//! live in the project's `cache/`, so being stateless survives it; the rules
//! are `scorsese_providers::quote`'s, so the hosted server keeps the same ones.
//!
//! ## What this publishes
//!
//! `serve`, which is what the binary runs, and `registry` with the `Tool` it
//! yields, the `Costs` that tool declares and the `Reply` it answers with (and
//! the `Context` it may run in, named by `Tool::call_in` and made only here) —
//! published because the gates above are integration tests and walk the
//! registry from outside the crate, and because the hosted server serves the
//! same tools (#539). [`protocol`] is what a message means apart from how it
//! arrived, published for that second transport. `tool_table`, `regenerated` and the two
//! markers they write between are published for that reason and no other.
//!
//! Everything else is plumbing: the modules are private, so `rpc`, `session`
//! and the tools themselves are reachable only from in here.

pub mod protocol;
mod renderer;
mod renders;
mod rpc;
mod session;
mod table;
mod tools;

pub use renderer::fetch_on_first_use;
pub use session::serve;
pub use table::{BEGIN, END, regenerated, tool_table};
pub use tools::{Context, Costs, Part, Reply, Tool, registry};
