//! What a stock library is, from scorsese's side: the trait tests mock.

use std::io::Write;
use std::path::PathBuf;

use crate::credentials::CredentialError;

use super::candidate::{Candidate, Medium, Orientation};

/// What to look for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// Footage or pictures.
    pub medium: Medium,
    /// The words, as a person would type them into a stock site.
    pub words: String,
    /// `film` or `animation` for footage; `photo`, `illustration` or
    /// `vector` for pictures. `None` means the first of each.
    pub style: Option<String>,
    /// Only results this way round. The vendor filters pictures itself and
    /// has no such filter for footage, so footage is filtered here, by its
    /// measured size.
    pub orientation: Option<Orientation>,
    /// Only footage at least this many seconds long.
    pub min_seconds: Option<u32>,
    /// Only results suitable for all ages.
    pub safe: bool,
}

impl Query {
    /// The words with the medium, and every filter the vendor is asked to
    /// apply: what a cached page is keyed by, and what it says it was.
    pub(crate) fn key(&self, page: u32) -> String {
        format!(
            "{} {:?} style={} orientation={} safe={} page={page}",
            self.medium.word(),
            self.words.trim().to_lowercase(),
            self.style.as_deref().unwrap_or("default"),
            self.orientation.map_or("any", Orientation::word),
            self.safe,
        )
    }

    /// Whether `candidate` passes the filters applied here rather than by
    /// the vendor.
    pub(crate) fn admits(&self, candidate: &Candidate) -> bool {
        let shaped = match (self.orientation, candidate.largest()) {
            (Some(way), Some(largest)) => way.fits(largest.width, largest.height),
            _ => true,
        };
        let long = match (self.min_seconds, candidate.seconds) {
            (Some(least), Some(seconds)) => seconds >= least,
            _ => true,
        };
        shaped && long
    }
}

/// One page of the vendor's results, before anything here filtered it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    /// The results on it.
    pub candidates: Vec<Candidate>,
    /// How many results the vendor will hand out for the query in all.
    pub total: u64,
}

/// Somewhere stock media can be searched and downloaded.
///
/// The seam every test drives instead of the network. A download goes
/// through it too, so an import is exercised end to end with no vendor.
pub trait Library {
    /// What this library is called, for a reply that names its source — which
    /// Pixabay asks of any displayed results.
    fn name(&self) -> &'static str;

    /// Page `page` (from 1) of `query`'s results, [`PAGE_SIZE`](super::PAGE_SIZE)
    /// to a page.
    fn page(&self, query: &Query, page: u32) -> Result<Page, StockError>;

    /// One result by its id, or `None` when there is none.
    fn one(&self, medium: Medium, id: u64) -> Result<Option<Candidate>, StockError>;

    /// Downloads `url` into `to`, at most `limit` bytes. Answers how many
    /// arrived.
    fn download(&self, url: &str, limit: u64, to: &mut dyn Write) -> Result<u64, StockError>;
}

/// Why a search, a preview or an import did not happen.
#[derive(Debug, thiserror::Error)]
pub enum StockError {
    /// No key anywhere the resolver looks.
    #[error(transparent)]
    Credential(#[from] CredentialError),

    /// The vendor refused the key.
    #[error(
        "{library} refused the key: {said}\nCheck PIXABAY_API_KEY — the key is on \
         https://pixabay.com/api/docs/ while logged in, and it is free."
    )]
    KeyRefused {
        /// Who refused.
        library: &'static str,
        /// What it said.
        said: String,
    },

    /// Past the vendor's rate limit — 100 requests a minute for Pixabay.
    #[error(
        "{library} has had too many requests this minute (its limit is 100 a minute). \
         Wait a minute and ask again; results already found are cached for 24 hours."
    )]
    RateLimited {
        /// Who said so.
        library: &'static str,
    },

    /// Nothing answers to that id.
    #[error("{library} has no {} {id}. Search again and use an id the results name.", medium.word())]
    NotFound {
        /// Who was asked.
        library: &'static str,
        /// What was asked for.
        medium: Medium,
        /// The id.
        id: u64,
    },

    /// The result has no file to download.
    #[error("{library}'s {} {id} lists no file to download", medium.word())]
    NoFile {
        /// Who was asked.
        library: &'static str,
        /// What was asked for.
        medium: Medium,
        /// The id.
        id: u64,
    },

    /// The vendor could not be reached, or refused in a way nothing above
    /// names.
    #[error("{library} could not be asked: {said}")]
    Provider {
        /// Who.
        library: &'static str,
        /// What went wrong, with no key in it.
        said: String,
    },

    /// A file under `cache/` could not be written or read.
    #[error("{}: {source}", path.display())]
    Io {
        /// Which file.
        path: PathBuf,
        /// What the filesystem said.
        source: std::io::Error,
    },

    /// The download arrived and would not import.
    #[error(transparent)]
    Import(#[from] scorsese_core::ImportError),
}
