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
//! MCP over stdio is JSON-RPC 2.0, one message per line. That is a small
//! enough surface — `initialize`, `tools/list`, `tools/call`, `ping` — that
//! taking an SDK for it would cost more than it saved: at the time of writing
//! the official Rust SDK is a beta, and it brings an async runtime to a
//! protocol that is one stream read in order. A blocking read of a line is
//! exactly the right shape, and every dependency here is one `cargo deny` has
//! to keep clearing.
//!
//! That is a judgement, not a principle, and it was tested when the protocol
//! grew the transport the hosted server needs (#539): **Streamable HTTP**, a
//! client `POST`ing each message to one URL. The SDK was weighed again then and
//! still not taken. What the server has to serve is a `POST` answered with
//! JSON — no server-initiated stream, no session — on an HTTP stack
//! (`scorsese-server`'s axum) that already exists; what it has to *share* with
//! this binary is what a message means, which is [`protocol`], written so that
//! neither transport owns it. Taking the SDK would have meant its tool macros
//! and its registry shape beside this one, and an async runtime in a binary
//! whose protocol is still one stream read in order. The day a client needs
//! something this cannot honestly serve — server-to-client requests,
//! resumable streams, OAuth discovery — the SDK is the answer again, and this
//! paragraph is the note to revise.
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
//! yields, the `Costs` that tool declares and the `Reply` it answers with —
//! published because the gates above are integration tests and walk the
//! registry from outside the crate, and because the hosted server serves the
//! same tools (#539). [`protocol`] is what a message means apart from how it
//! arrived, published for that second transport. `tool_table`, `regenerated` and the two
//! markers they write between are published for that reason and no other.
//!
//! Everything else is plumbing: the modules are private, so `rpc`, `session`
//! and the tools themselves are reachable only from in here.

mod base64;
pub mod protocol;
mod rpc;
mod session;
mod table;
mod tools;

pub use session::serve;
pub use table::{BEGIN, END, regenerated, tool_table};
pub use tools::{Costs, Part, Reply, Tool, registry};
