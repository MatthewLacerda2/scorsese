//! One module per subcommand. Each is thin: it parses intent, calls the
//! library, and prints. Logic that the MCP server would also want lives in
//! `scorsese-core` or `scorsese-render`, never here.

pub(crate) mod assets;
pub(crate) mod check;
pub(crate) mod describe;
pub(crate) mod dissolve;
pub(crate) mod duck;
pub(crate) mod import;
pub(crate) mod level;
pub(crate) mod new;
pub(crate) mod prices;
pub(crate) mod probe;
pub(crate) mod render;
pub(crate) mod settings;
pub(crate) mod still;
pub(crate) mod synth;
