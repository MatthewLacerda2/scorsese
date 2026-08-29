//! The note renderer: the shape of the buffer it returns, and that each
//! optional stage actually reaches the signal.

#[path = "../common/mod.rs"]
mod common;

mod envelopes;
mod shape;
mod stages;
mod velocity;
mod wobble;
