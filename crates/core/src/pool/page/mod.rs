//! Web pages in the pool: brought in from a file, or written in place.
//!
//! Two doors onto one kind of asset, and a third onto the files beside it. Import copies a page somebody already
//! has; writing is how an agent authors one — the text arrives as an
//! argument, never as a file on the caller's machine, and the same call that
//! makes a page edits it.

mod file;
mod import;
mod write;

pub use file::{
    MAX_PAGE_FILE_BYTES, PAGE_FILE_KINDS, PageFileError, PageFileWritten, page_file_path,
    read_page_file, write_page_file,
};
pub(super) use import::import_page;
pub use write::{PageError, PageWritten, write_page};
