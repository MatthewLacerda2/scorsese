//! The `scorsese-mcp` binary: an MCP server on stdin and stdout.
//!
//! A client spawns this and talks to it over the pipe. There are no arguments
//! and no configuration: which project a call works on is an argument of the
//! call, so one server serves every project without being told about any of
//! them.

use std::io::{BufReader, stdin, stdout};

fn main() -> std::io::Result<()> {
    // stdout is the protocol channel and nothing else may write to it — a
    // stray `println!` would be read as a malformed message and take the
    // session down. Anything this server has to say about itself goes to
    // stderr, which the client shows as logs.
    scorsese_mcp::serve(BufReader::new(stdin().lock()), stdout().lock())
}
