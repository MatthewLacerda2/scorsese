//! One module per subcommand. Each is thin: it parses intent, calls the
//! library, and prints. Logic that the MCP server would also want lives in
//! `scorsese-core` or `scorsese-render`, never here.

pub mod assets;
pub mod check;
pub mod import;
pub mod new;
pub mod render;
