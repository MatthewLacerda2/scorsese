//! `new`, `import`, `probe`, `migrate`, `page`, `caption` and `assets` — the
//! commands that change what is on disk, driven end to end against real
//! project directories.
//!
//! What each asserts is the effect: the directory that exists afterwards, the
//! bytes that were copied, the table the reloaded document holds. `assets` is
//! the partial exception, because reporting what the pool contains is the
//! whole of what it does — so its tests assert the substance of the report,
//! asset by asset, and never a whole line of it.

mod assets;
mod caption;
mod import;
mod migrate;
mod new;
mod page;
mod probe;
mod reframe;
mod transcribe;
