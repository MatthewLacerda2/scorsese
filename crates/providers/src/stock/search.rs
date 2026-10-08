//! Searching, and finding one result again by its id.

use std::path::Path;

use super::cache;
use super::candidate::{Candidate, Medium};
use super::library::{Library, Query, StockError};

/// How many results one reply shows: one contact sheet's worth.
///
/// The compositor tiles at most five cells to a sheet, and a reply whose
/// pictures are one sheet costs one image to look at. Restated rather than
/// imported because this crate does not depend on the compositor; the
/// surfaces that draw the sheet hold the two to each other in a test.
pub const PER_REPLY: usize = 5;

/// How many results are asked of the vendor at a time — and cached at a time.
///
/// Fifty, so one request answers ten replies' worth when nothing is filtered
/// here, which keeps a key's 100 requests a minute far away.
pub const PAGE_SIZE: u32 = 50;

/// The most vendor pages one search reads through, filtering, before saying
/// it found what it found: 200 results.
const MOST_PAGES: u32 = 4;

/// One reply's worth of results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The results, at most [`PER_REPLY`] of them.
    pub candidates: Vec<Candidate>,
    /// Which reply page this is, from 1.
    pub page: u32,
    /// Whether asking for the next page would find more.
    pub more: bool,
    /// How many results the vendor will hand out for the words in all,
    /// before anything here filtered them.
    pub total: u64,
    /// Whether the vendor was asked on this call, rather than the cache.
    pub fetched: bool,
    /// Who was asked.
    pub library: &'static str,
}

impl Found {
    /// The line that closes a listing: how many, from whom, how to see more.
    ///
    /// It names the library on every reply, because Pixabay asks that
    /// displayed results say where they come from.
    pub fn summary(&self) -> String {
        let source = if self.fetched {
            "just now"
        } else {
            "out of the 24-hour cache"
        };
        let mut said = format!(
            "{} result{} from {} (page {}, {} matching in all), read {source}.",
            self.candidates.len(),
            if self.candidates.len() == 1 { "" } else { "s" },
            self.library,
            self.page,
            self.total,
        );
        if self.more {
            said.push_str(&format!(" More on page {}.", self.page + 1));
        }
        said
    }
}

/// Reply page `page` (from 1) of `query`'s results.
///
/// Vendor pages are read from the cache when it has them fresh and asked for
/// when it does not, then filtered here — orientation and length for
/// footage, which the vendor cannot filter — until the reply page is full or
/// [`MOST_PAGES`] have been read. Expired files are pruned on the way.
pub fn search(
    cache: &Path,
    library: &dyn Library,
    query: &Query,
    page: u32,
) -> Result<Found, StockError> {
    cache::prune(cache);
    let page = page.max(1);
    let first = usize::try_from(page - 1).unwrap_or(usize::MAX) * PER_REPLY;
    // One past the page, to know whether there is a next one.
    let wanted = first + PER_REPLY + 1;
    let mut admitted: Vec<Candidate> = Vec::new();
    let (mut total, mut fetched) = (0, false);
    for at in 1..=MOST_PAGES {
        let key = query.key(at);
        let read = match cache::read(cache, &key) {
            Some(read) => read,
            None => {
                let read = library.page(query, at)?;
                // A cache that will not write is not worth failing a search
                // over: the results are in hand.
                let _ = cache::write(cache, &key, &read);
                fetched = true;
                read
            }
        };
        total = read.total;
        let short = read.candidates.len() < PAGE_SIZE as usize;
        admitted.extend(read.candidates.into_iter().filter(|one| query.admits(one)));
        if admitted.len() >= wanted || short || u64::from(at * PAGE_SIZE) >= total {
            break;
        }
    }
    let more = admitted.len() > first + PER_REPLY;
    Ok(Found {
        candidates: admitted.into_iter().skip(first).take(PER_REPLY).collect(),
        page,
        more,
        total,
        fetched,
        library: library.name(),
    })
}

/// The result `medium` `id` names: out of a fresh cached page when a search
/// returned it, else asked of the vendor by id — and then cached too.
pub fn find(
    cache: &Path,
    library: &dyn Library,
    medium: Medium,
    id: u64,
) -> Result<Candidate, StockError> {
    if let Some(found) = cache::find_cached(cache, medium, id) {
        return Ok(found);
    }
    let found = library.one(medium, id)?.ok_or(StockError::NotFound {
        library: library.name(),
        medium,
        id,
    })?;
    let page = super::library::Page {
        candidates: vec![found.clone()],
        total: 1,
    };
    let _ = cache::write(cache, &format!("{} id={id}", medium.word()), &page);
    Ok(found)
}
