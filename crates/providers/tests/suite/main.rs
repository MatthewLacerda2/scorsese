//! Every integration test of `providers`, in one binary (#1128).
//!
//! Each module is what used to be a test binary of its own; one binary
//! links once instead of once per file.

mod chatting;
mod claude;
mod common;
mod composing;
mod credentials;
mod designing;
mod documented;
mod drawing;
mod generating;
mod live;
mod offline;
mod pricing;
mod quoting;
mod speaking;
mod stock;
mod synthesis;
mod transcribing;
mod voices;
