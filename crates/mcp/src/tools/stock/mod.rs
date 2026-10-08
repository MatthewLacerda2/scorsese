//! Free stock footage and photos, without a stock site (#900).
//!
//! Two tools, the way `icons` finds a name before an asset uses it:
//! [`search`] finds and **shows** candidates — one contact sheet of their
//! previews, because picking by tags alone imports the wrong shot — and
//! [`import`] brings the chosen ones in as ordinary `video` or `image` assets.
//! Both are free: Pixabay's API has no paid tier, so neither quotes.
//!
//! The work is `scorsese_providers::stock`'s, and the sheet is
//! `scorsese_render::contact`'s; this is the wiring and the words.

mod import;
mod search;

pub(crate) use import::Import;
pub(crate) use search::Search;

use scorsese_providers::credentials::{Provider, resolve};
use scorsese_providers::stock::{Medium, PixabayLibrary};

/// The library every call searches, keyed from the one resolver.
fn library() -> Result<PixabayLibrary, String> {
    let key = resolve(Provider::Pixabay).map_err(|error| format!("{error}"))?;
    Ok(PixabayLibrary::new(&key.secret))
}

/// `video` or `image`, with `video` the default.
fn medium(given: Option<&str>) -> Result<Medium, String> {
    match given.map(str::trim) {
        None | Some("" | "video") => Ok(Medium::Video),
        Some("image") => Ok(Medium::Image),
        Some(other) => Err(format!("`kind` is video or image, not `{other}`")),
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
        assert!(super::medium(Some("audio")).is_err());
    }
}
