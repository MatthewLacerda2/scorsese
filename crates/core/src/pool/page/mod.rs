//! Web pages in the pool: brought in from a file, or written in place.
//!
//! Two doors onto one kind of asset. Import copies a page somebody already
//! has; writing is how an agent authors one — the text arrives as an
//! argument, never as a file on the caller's machine, and the same call that
//! makes a page edits it.

mod import;
mod write;

pub(super) use import::import_page;
pub use write::{PageError, PageWritten, write_page};
