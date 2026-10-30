//! What a stock library is, from scorsese's side: the trait tests mock.

use std::io::Write;
use std::path::PathBuf;

use crate::api::http::{HttpError, host, https_host};
use crate::credentials::{CredentialError, Secret};

use super::candidate::{Candidate, Medium, Orientation};

/// What to look for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// Footage, pictures or animations.
    pub medium: Medium,
    /// The words, as a person would type them into a stock site.
    pub words: String,
    /// `film` or `animation` for footage; `photo`, `illustration` or
    /// `vector` for pictures. `None` means `film` or `photo` — real footage
    /// and real photographs, which is what a generic shot almost always is;
    /// Pixabay's own default mixes in animation and clip art. Animations have
    /// no style.
    pub style: Option<String>,
    /// Only results this way round. The vendor filters pictures itself and
    /// has no such filter for footage, so footage is filtered here, by its
    /// measured size.
    pub orientation: Option<Orientation>,
    /// Only footage at least this many seconds long.
    pub min_seconds: Option<u32>,
    /// Only results suitable for all ages, where the vendor can say:
    /// Pixabay can, LottieFiles' public search has no such filter.
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

    /// The hosts its files are served from: a download goes only to one of
    /// these or a subdomain of one, over https ([`fetchable`]), whatever page
    /// named it.
    fn hosts(&self) -> &'static [&'static str];

    /// Downloads `url` into `to`, at most `limit` bytes. Answers how many
    /// arrived. Called only once [`fetchable`] has passed `url`; a real
    /// library follows a redirect only where [`fetchable`] passes it too.
    fn download(&self, url: &str, limit: u64, to: &mut dyn Write) -> Result<u64, StockError>;
}

/// `url`, when it is one `library` may be asked to download: https, on one of
/// its [`hosts`](Library::hosts) or a subdomain of one, on the usual port, and
/// with no user name in it. Read the way the transport reads it, so what is
/// checked is what would be called.
///
/// A cached page is a file on the disk, and what it says is never taken as
/// the vendor's word for where a file is: this is asked before every download.
pub(crate) fn fetchable(library: &dyn Library, url: &str) -> Result<(), StockError> {
    let refused = || StockError::OffHost {
        library: library.name(),
        url: url.chars().take(200).collect(),
    };
    let host = https_host(url).ok_or_else(refused)?;
    let theirs = library.hosts().iter().any(|own| {
        host == *own
            || host
                .strip_suffix(own)
                .and_then(|sub| sub.strip_suffix('.'))
                .is_some_and(|sub| !sub.is_empty())
    });
    theirs.then_some(()).ok_or_else(refused)
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

    /// A still asked to be looked through: a photo has no frames to take,
    /// and its preview is already the whole of it.
    #[error(
        "image {id} is a still, so there is nothing to look through: its preview in the \
         search is the whole of it. Looking is for a video or a lottie."
    )]
    Still {
        /// The image's id.
        id: u64,
    },

    /// A download named somewhere that is not the library's own, or not over
    /// https: never asked for.
    #[error(
        "{library} lists a file at {url}, which is not over https on one of {library}'s own \
         hosts, so nothing was downloaded. Search again and import an id the results name."
    )]
    OffHost {
        /// Whose result named it.
        library: &'static str,
        /// The address, cut short.
        url: String,
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

    /// What was downloaded as a Lottie animation is not one.
    #[error("{library}'s animation {id} did not read as a Lottie file: {said}")]
    NotLottie {
        /// Who sent it.
        library: &'static str,
        /// The id.
        id: u64,
        /// What the reading said.
        said: String,
    },

    /// A Lottie asked for as an asset: it is not footage or a picture, and
    /// is imported for a page to play instead.
    #[error(
        "lottie {id} is an animation a page plays, not footage or a picture: import it as \
         a lottie, and play it from a page (the pages guide: `guide pages`, or docs/pages.md)"
    )]
    NotMedia {
        /// The id.
        id: u64,
    },
}

/// A download's transport error from `library`, in words that never carry
/// what the server sent: the host, and its status, or that it could not be
/// reached or the file broke off. A file host's error page is not a sentence
/// for an agent.
///
/// The host is the one that failed, which after a redirect is not always the
/// one first asked.
pub(super) fn fetch_failed(library: &'static str, error: &HttpError) -> StockError {
    if error.rate_limited() {
        return StockError::RateLimited { library };
    }
    let (HttpError::Refused { url, .. }
    | HttpError::Unreachable { url, .. }
    | HttpError::Unreadable { url, .. }) = error;
    let host = host(url).unwrap_or_else(|| String::from("its file host"));
    let said = match error {
        HttpError::Refused { status, .. } if (300..400).contains(status) => format!(
            "{host} redirected a download off {library}'s own hosts, or too many times, so \
             nothing was downloaded"
        ),
        HttpError::Refused { status, .. } => format!("{host} answered {status} to a download"),
        HttpError::Unreachable { .. } => format!("{host} could not be reached for a download"),
        HttpError::Unreadable { .. } => {
            format!("a download from {host} broke off, or could not be kept on this machine")
        }
    };
    StockError::Provider { library, said }
}

/// A transport error from `library` as the refusal it means, the way every
/// library reads one: a 429 as [`StockError::RateLimited`], anything else in
/// the words [`HttpError::explained`] gives it, `key` scrubbed out.
pub(super) fn failed(library: &'static str, error: &HttpError, key: Option<&Secret>) -> StockError {
    if error.rate_limited() {
        return StockError::RateLimited { library };
    }
    StockError::Provider {
        library,
        said: error.explained(key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Answers nothing: only its hosts are asked.
    struct Hosted;

    impl Library for Hosted {
        fn name(&self) -> &'static str {
            "Hosted"
        }

        fn page(&self, _: &Query, _: u32) -> Result<Page, StockError> {
            unreachable!("only the hosts are asked")
        }

        fn one(&self, _: Medium, _: u64) -> Result<Option<Candidate>, StockError> {
            unreachable!("only the hosts are asked")
        }

        fn hosts(&self) -> &'static [&'static str] {
            &["pixabay.com", "lottiefiles.com"]
        }

        fn download(&self, _: &str, _: u64, _: &mut dyn Write) -> Result<u64, StockError> {
            unreachable!("only the hosts are asked")
        }
    }

    #[test]
    fn a_vendors_own_file_over_https_is_fetchable() {
        for url in [
            "https://cdn.pixabay.com/video/2020/05/13/39009-420224623_large.mp4",
            "https://pixabay.com/get/TOKEN_1280.png",
            "https://CDN.Pixabay.com:443/photo/x.jpg",
            "https://assets-v2.lottiefiles.com/a/abc/x.json",
        ] {
            assert!(fetchable(&Hosted, url).is_ok(), "{url}");
        }
    }

    #[test]
    fn anything_else_is_refused() {
        for url in [
            "http://cdn.pixabay.com/video/x.mp4",
            "https://cdn.pixabay.com.evil.invalid/x.mp4",
            "https://notpixabay.com/x.mp4",
            "https://.pixabay.com/x.mp4",
            "https://cdn.pixabay.com@127.0.0.1/x.mp4",
            "https://user:pass@cdn.pixabay.com/x.mp4",
            "https://cdn.pixabay.com:8443/x.mp4",
            "https://127.0.0.1/x.mp4",
            "https://[::1]/x.mp4",
            "file:///etc/passwd",
            "cdn.pixabay.com/x.mp4",
            "",
        ] {
            let refused = fetchable(&Hosted, url).expect_err(url).to_string();
            assert!(
                refused.contains("nothing was downloaded"),
                "{url}: {refused}"
            );
        }
    }

    #[test]
    fn a_failed_download_never_says_what_the_server_sent() {
        let url = "https://cdn.pixabay.com/video/x.mp4";
        let refused = HttpError::Refused {
            url: url.to_owned(),
            status: 500,
            body: String::from("<html>internal details</html>"),
        };
        let said = fetch_failed("Pixabay", &refused).to_string();
        assert!(said.contains("cdn.pixabay.com answered 500"), "{said}");
        assert!(!said.contains("internal details"), "{said}");
        let unreachable = HttpError::Unreachable {
            url: url.to_owned(),
            message: String::from("dns error naming a resolver"),
        };
        let said = fetch_failed("Pixabay", &unreachable).to_string();
        assert!(!said.contains("resolver"), "{said}");
        let limited = HttpError::Refused {
            url: url.to_owned(),
            status: 429,
            body: String::new(),
        };
        assert!(matches!(
            fetch_failed("Pixabay", &limited),
            StockError::RateLimited { .. }
        ));
        let moved = HttpError::Refused {
            url: String::from("https://pixabay.com/get/x"),
            status: 302,
            body: String::new(),
        };
        let said = fetch_failed("Pixabay", &moved).to_string();
        assert!(
            said.contains("pixabay.com redirected a download off"),
            "{said}"
        );
    }
}
