//! Free stock footage and photos (#900): search, look, import.
//!
//! A large share of the shots in a promotional, product or social video are
//! generic — a city at night, hands on a keyboard, a cat asleep — and paying a
//! generation for one is the wrong tool. Pixabay's library covers them for
//! nothing, under a licence that allows commercial use without attribution,
//! so this is where an agent looks **before** it reaches for
//! [`video`](crate::video).
//!
//! # What holds this together
//!
//! **Nothing here spends.** Pixabay's API has no paid tier, so there is no
//! quote, no ceiling and no ledger — only a key, from the one resolver, as
//! `PIXABAY_API_KEY`.
//!
//! **Results are cached 24 hours, as Pixabay requires** — under a cache
//! folder the caller names ([`cache_dir`] for a project), one file per page of
//! one query. That is also what makes an id from a search **addressable
//! afterwards** without asking again: [`find_cached`] reads a result back out
//! by id, which is how an import, and a picker on the web (#901), hold an id
//! to what a search really returned.
//!
//! **An agent looks before it chooses.** [`previews`] brings each result's
//! small JPEG down, for a surface to tile into one contact sheet, and
//! [`footage`] brings a video's smallest rendition down for a sheet of frames
//! across it. Picking by tags alone imports the wrong shot.
//!
//! **An import is an ordinary import.** [`import`] downloads the rendition
//! that fills the render's frame — by measured size, never by the vendor's
//! name for it — and hands the file to `scorsese_core::import_path`, so it is
//! a `video` or `image` asset like a file somebody dropped in: probed, hashed,
//! copied into `assets/`. No new asset kind and no format change; where it
//! came from is in the file name, `pixabay-<id>`.
//!
//! # The trait is the test seam
//!
//! [`Library`] is what everything above talks to, downloads included, so a
//! search, its cache and an import run in tests with no network and no key.

mod cache;
mod candidate;
mod fetch;
mod library;
mod pixabay;
mod search;

pub use cache::{FRESH_FOR, cache_dir, find_cached};
pub use candidate::{Candidate, Medium, Orientation, Rendition};
pub use fetch::{Choice, Fetched, footage, import, previews};
pub use library::{Library, Page, Query, StockError};
pub use pixabay::PixabayLibrary;
pub use search::{Found, PAGE_SIZE, PER_REPLY, find, search};

/// What every reply that shows results says about the licence, in one line —
/// the same on every surface.
pub const LICENCE: &str = "Pixabay Content License: free to use, commercially too, with no \
attribution needed; not to be resold as stock as it is. Identifiable people, logos or brands \
in a commercial video may need their consent, which is the user's to get.";
