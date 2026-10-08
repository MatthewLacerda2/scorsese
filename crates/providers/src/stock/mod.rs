//! Free stock footage and photos (#900), and animations (#903): search, look,
//! import.
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
//! # Animations are a third medium, not a third tool
//!
//! LottieFiles' free animations — characters, mascots, animated icons — are
//! searched, looked at and chosen exactly as footage is, so they are
//! [`Medium::Lottie`] behind the same [`Library`] trait ([`LottieLibrary`])
//! and the same cache, and need no key. Only the import differs: a Lottie is
//! played by a page, not placed on the timeline, so [`keep`] writes its JSON
//! under `pages/` for a page to load, and [`import`] refuses one.
//!
//! # The trait is the test seam
//!
//! [`Library`] is what everything above talks to, downloads included, so a
//! search, its cache and an import run in tests with no network and no key.

mod cache;
mod candidate;
mod fetch;
mod library;
mod lottie;
mod pixabay;
mod search;

pub use cache::{FRESH_FOR, cache_dir, find_cached};
pub use candidate::{Candidate, Medium, Orientation, Rendition, named_in};
pub use fetch::{Choice, Fetched, footage, import, previews};
pub use library::{Library, Page, Query, StockError};
pub use lottie::{Kept, LottieLibrary, keep};
pub use pixabay::PixabayLibrary;
pub use search::{Found, PAGE_SIZE, PER_REPLY, find, search};

/// What every reply that shows results says about the licence, in one line —
/// the same on every surface.
pub const LICENCE: &str = "Pixabay Content License: free to use, commercially too, with no \
attribution needed; not to be resold as stock as it is. Identifiable people, logos or brands \
in a commercial video may need their consent, which is the user's to get.";

/// [`LICENCE`]'s line for LottieFiles' free animations.
pub const LOTTIE_LICENCE: &str = "Lottie Simple License: free to use and change, commercially \
too, with attribution encouraged but not required; not to be redistributed as a file on its own \
or gathered into a library.";

/// The licence line for results of `medium`.
pub const fn licence(medium: Medium) -> &'static str {
    match medium {
        Medium::Video | Medium::Image => LICENCE,
        Medium::Lottie => LOTTIE_LICENCE,
    }
}

/// The library `medium` is searched in: LottieFiles for animations, which
/// needs no key, and Pixabay otherwise, keyed from the one resolver as
/// `PIXABAY_API_KEY`.
pub fn library(medium: Medium) -> Result<Box<dyn Library + Send + Sync>, StockError> {
    match medium {
        Medium::Lottie => Ok(Box::new(LottieLibrary::new())),
        Medium::Video | Medium::Image => {
            let key = crate::credentials::resolve(crate::credentials::Provider::Pixabay)?;
            Ok(Box::new(PixabayLibrary::new(&key.secret)))
        }
    }
}
