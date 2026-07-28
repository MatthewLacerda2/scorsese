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
//! That is a judgement, not a principle. If the protocol grows a transport
//! this cannot honestly serve, taking the SDK is the right answer, and this
//! paragraph is the note explaining why it was not taken sooner.
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
//! ## Stateless
//!
//! Every tool takes the project directory it works on. There is no server-side
//! "open project" to go stale, so a client may crash, reconnect, or run two
//! conversations against one project without anything getting out of step.

pub mod rpc;
pub mod session;
pub mod tools;

pub use session::serve;
pub use tools::{Tool, registry};
