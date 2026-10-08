//! Pixabay's part of the live check: one search, free.
//!
//! **`GET /api/videos/?q=…`** — the key, and the shape [`crate::stock`]
//! reads: hits, each with an id and at least one rendition that has a URL and
//! a size. Pixabay has no paid tier, so this part never spends anything.

use crate::api::http::HttpError;
use crate::api::pixabay::{Listing, Pixabay, Search, VideoHit};
use crate::api::tap::Tap;
use crate::credentials::Secret;

use super::{Step, Verdict, judge};

/// What the search looks for: something the library is certain to have.
pub const WORDS: &str = "sunrise";

/// The calls, as a quote lists them.
pub(super) fn calls() -> Vec<String> {
    vec![format!("GET api/videos/?q={WORDS} — free")]
}

/// Runs the Pixabay part; it spends nothing.
pub(super) fn check(key: &Secret, tap: &Tap) -> (Vec<Step>, u64) {
    let search = Search {
        q: Some(WORDS.to_owned()),
        per_page: Some(crate::api::pixabay::MIN_PER_PAGE),
        ..Search::default()
    };
    (
        vec![search_step(Pixabay::new(key).tapped(tap).videos(&search))],
        0,
    )
}

/// What the search gave back.
pub fn search_step(answer: Result<Listing<VideoHit>, HttpError>) -> Step {
    let call = "GET api/videos/";
    let listing = match answer {
        Err(error) => return Step::new(call, judge::pixabay(&error)),
        Ok(listing) => listing,
    };
    let field = if listing.hits.is_empty() {
        Some(String::from("hits: the search came back empty"))
    } else if listing.hits.iter().any(|hit| {
        let each = [
            &hit.videos.tiny,
            &hit.videos.small,
            &hit.videos.medium,
            &hit.videos.large,
        ];
        !each.iter().any(|one| !one.url.is_empty() && one.width > 0)
    }) {
        Some(String::from(
            "hits[].videos: a hit arrived with no usable rendition",
        ))
    } else {
        None
    };
    match field {
        Some(field) => Step::new(call, Verdict::ShapeChanged { field }),
        None => Step::new(call, Verdict::Ok).noting(format!(
            "{} hits of {}",
            listing.hits.len(),
            listing.total_hits
        )),
    }
}
