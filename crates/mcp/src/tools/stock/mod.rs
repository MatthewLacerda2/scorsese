//! Free stock footage and photos (#900), and animations (#903), without a
//! stock site.
//!
//! Two tools, the way `icons` finds a name before an asset uses it:
//! [`search`] finds and **shows** candidates — one contact sheet of their
//! previews, because picking by tags alone imports the wrong shot — and
//! [`import`] brings the chosen ones in: footage and photos from Pixabay as
//! ordinary `video` or `image` assets, a Lottie animation from LottieFiles as
//! a JSON file under `pages/` for a page to play. Both are free: neither
//! library has a paid tier, so neither quotes.
//!
//! **A Lottie is a `kind` of the same pair, not a pair of its own.** It is
//! found, looked at and chosen exactly as footage is, from the same cache, so
//! an agent that knows the stock tools already knows how to get one; what
//! differs is only where it lands and what plays it, which both descriptions
//! say plainly.
//!
//! The work is `scorsese_providers::stock`'s, and the sheet is
//! `scorsese_render::contact`'s; this is the wiring and the words.

mod import;
mod player;
mod search;

pub(crate) use import::Import;
pub(crate) use search::Search;

use scorsese_providers::stock::{self, Library, Medium};

/// The library `medium` is searched in: Pixabay, keyed from the one
/// resolver, or LottieFiles, which needs no key.
fn library(medium: Medium) -> Result<Box<dyn Library + Send + Sync>, String> {
    stock::library(medium).map_err(|error| format!("{error}"))
}

/// `video`, `image` or `lottie`, with `video` the default.
fn medium(given: Option<&str>) -> Result<Medium, String> {
    match given.map(str::trim) {
        None | Some("") => Ok(Medium::Video),
        Some(word) => Medium::named(word)
            .ok_or_else(|| format!("`kind` is video, image or lottie, not `{word}`")),
    }
}

#[cfg(test)]
mod tests {
    use scorsese_providers::stock::PER_REPLY;
    use scorsese_render::contact::MAX_FRAMES;

    /// A reply's results are one sheet: the two numbers are restated in two
    /// crates that do not depend on each other, and held together here.
    #[test]
    fn a_reply_is_one_sheet() {
        assert_eq!(PER_REPLY, MAX_FRAMES);
    }

    #[test]
    fn the_kind_defaults_to_video() {
        assert_eq!(super::medium(None), Ok(super::Medium::Video));
        assert_eq!(super::medium(Some("lottie")), Ok(super::Medium::Lottie));
        assert!(super::medium(Some("audio")).is_err());
        // The `kind`s both schemas list.
        for kind in ["video", "image", "lottie"] {
            assert_eq!(super::medium(Some(kind)).map(super::Medium::word), Ok(kind));
        }
    }
}
