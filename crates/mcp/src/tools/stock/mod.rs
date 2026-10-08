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
//!
//! **Where the library comes from is the caller's to say** (#906). The
//! registry's pair resolves each medium's own — Pixabay keyed from the one
//! resolver, LottieFiles unkeyed — which is right wherever the user's own
//! environment is. [`stocked_from`] builds the same pair, same names and same
//! words, answering from one library given instead: how the hosted server
//! hands them a fake in its tests, with no key and no network. Nothing here
//! learns who is calling.

mod import;
mod player;
mod search;

pub(crate) use import::Import;
pub(crate) use search::Search;

use std::sync::Arc;

use scorsese_providers::stock::{self, Library, Medium};

use super::Tool;

/// A stock library the stock tools can be handed in place of each medium's
/// own ([`stocked_from`]).
pub type Stock = Arc<dyn Library + Send + Sync>;

/// `stock_search` and `stock_import`, exactly as the registry has them, but
/// searching and importing every kind from `library`.
pub fn stocked_from(library: &Stock) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(Search::from(Some(library.clone()))),
        Box::new(Import::from(Some(library.clone()))),
    ]
}

/// The library `medium` is searched in: `given`, when the tools were built
/// around one, and otherwise Pixabay, keyed from the one resolver, or
/// LottieFiles, which needs no key.
fn library(given: Option<&Stock>, medium: Medium) -> Result<Stock, String> {
    match given {
        Some(library) => Ok(library.clone()),
        None => stock::library(medium)
            .map(Arc::from)
            .map_err(|error| format!("{error}")),
    }
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
    use std::sync::Arc;

    use scorsese_core::{Fps, Project};
    use scorsese_providers::stock::{
        Candidate, Library, Medium, PER_REPLY, Page, Query, StockError,
    };
    use scorsese_render::contact::MAX_FRAMES;
    use serde_json::json;

    /// A library that has nothing at all, and says so by its own name.
    struct Empty;

    impl Library for Empty {
        fn name(&self) -> &'static str {
            "Empty"
        }

        fn page(&self, _: &Query, _: u32) -> Result<Page, StockError> {
            Ok(Page {
                candidates: Vec::new(),
                total: 0,
            })
        }

        fn one(&self, _: Medium, _: u64) -> Result<Option<Candidate>, StockError> {
            Ok(None)
        }

        fn download(
            &self,
            url: &str,
            _: u64,
            _: &mut dyn std::io::Write,
        ) -> Result<u64, StockError> {
            unreachable!("nothing is there to download: {url}")
        }
    }

    /// The pair built around a library is the registry's pair by name, and
    /// both answer from that library — never Pixabay's, which here would
    /// want a key or the network.
    #[test]
    fn stocked_tools_answer_from_the_library_given() {
        let dir = std::env::temp_dir().join(format!("scorsese-mcp-stocked-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Project::new("stocked", Fps::THIRTY).save(&dir).unwrap();
        let library: super::Stock = Arc::new(Empty);
        let tools = super::stocked_from(&library);
        let names: Vec<&str> = tools.iter().map(|tool| tool.name()).collect();
        assert_eq!(names, ["stock_search", "stock_import"]);

        let project = dir.to_string_lossy();
        let search = json!({ "project": project, "query": "sunrise", "kind": "image" });
        let found = tools[0].call(&search).expect("the library answers");
        assert!(
            found.parts[0].text.starts_with("Nothing matched"),
            "{}",
            found.parts[0].text
        );
        let import = json!({ "project": project, "id": 5, "kind": "image" });
        let Err(refused) = tools[1].call(&import) else {
            panic!("the library has no image 5");
        };
        assert!(refused.contains("Empty has no image 5"), "{refused}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

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
