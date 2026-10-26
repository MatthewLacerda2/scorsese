//! Every integration test of `mcp`, in one binary (#1128).
//!
//! Each module is what used to be a test binary of its own; one binary
//! links once instead of once per file.

mod described;
mod neutral;
mod protocol;
mod table;
mod unknown;
