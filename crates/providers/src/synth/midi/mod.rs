//! MIDI both ways: a `.mid` in as a song recipe, and a song recipe out as a
//! `.mid` for a DAW.
//!
//! Both conversions are `scorsese-zimmer`'s, because turning bytes into a song
//! and a song into bytes is arithmetic on a document. What lives here is the
//! half that touches a disk, and the words both front doors report the result
//! in — one set of lines per direction, so the command line and the MCP tool
//! cannot describe one conversion differently.

mod export;
mod import;

pub use export::{ToMidi, export_midi};
pub use import::{FromMidi, import_midi};
