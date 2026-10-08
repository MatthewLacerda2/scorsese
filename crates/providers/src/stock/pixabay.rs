//! Pixabay behind the trait: its hits in, candidates out, refusals read.
//!
//! Translation only, in both directions — the wire is
//! [`api::pixabay`](crate::api::pixabay)'s, what to do with an answer is
//! [`super`]'s.

use std::io::Write;

use crate::api::http::{self, HttpError};
use crate::api::pixabay::{self as wire, ImageHit, Listing, Pixabay, Search, VideoHit};
use crate::credentials::Secret;

use super::PAGE_SIZE;
use super::candidate::{Candidate, Medium, Rendition};
use super::library::{Library, Page, Query, StockError};

/// What this library is called wherever a message names it.
const NAME: &str = "Pixabay";

/// The longest side of `largeImageURL`, the largest picture offered without
/// full API access.
const LARGE_IMAGE_SIDE: u32 = 1280;

/// The longest side of `fullHDURL`, offered with full API access.
const FULL_HD_SIDE: u32 = 1920;

/// Pixabay, as a stock library.
#[derive(Debug, Clone)]
pub struct PixabayLibrary {
    api: Pixabay,
    /// Kept to scrub out of any message the transport writes, which should
    /// never hold it — the key travels as a query parameter the caller adds —
    /// and is scrubbed anyway, because a key in a log is not undone.
    key: Secret,
}

impl PixabayLibrary {
    /// One that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            api: Pixabay::new(key),
            key: key.clone(),
        }
    }

    /// A transport error as the refusal it means.
    fn failure(&self, error: &HttpError) -> StockError {
        let scrub = |text: &str| text.replace(self.key.expose(), "<key>");
        match error {
            HttpError::Refused { status: 429, .. } => StockError::RateLimited { library: NAME },
            HttpError::Refused { status, body, .. }
                if matches!(status, 400 | 401 | 403) && body.contains("API key") =>
            {
                StockError::KeyRefused {
                    library: NAME,
                    said: scrub(body.trim()),
                }
            }
            other => StockError::Provider {
                library: NAME,
                said: scrub(&other.to_string()),
            },
        }
    }
}

impl Library for PixabayLibrary {
    fn name(&self) -> &'static str {
        NAME
    }

    fn page(&self, query: &Query, page: u32) -> Result<Page, StockError> {
        if query.medium == Medium::Lottie {
            // Pixabay has no animations of that kind; LottieFiles is asked.
            return Ok(Page::default());
        }
        let search = Search {
            q: Some(query.words.chars().take(100).collect()),
            kind: Some(query.style.clone().unwrap_or_else(|| {
                String::from(match query.medium {
                    Medium::Video => "film",
                    Medium::Image | Medium::Lottie => "photo",
                })
            })),
            orientation: query.orientation.map(|way| way.word().to_owned()),
            page: Some(page),
            per_page: Some(PAGE_SIZE),
            safesearch: query.safe,
            ..Search::default()
        };
        match query.medium {
            Medium::Video | Medium::Lottie => self
                .api
                .videos(&search)
                .map(|listing| paged(&listing, video))
                .map_err(|error| self.failure(&error)),
            Medium::Image => self
                .api
                .images(&search)
                .map(|listing| paged(&listing, image))
                .map_err(|error| self.failure(&error)),
        }
    }

    fn one(&self, medium: Medium, id: u64) -> Result<Option<Candidate>, StockError> {
        let search = Search {
            id: Some(id),
            ..Search::default()
        };
        let found = match medium {
            Medium::Video => self.api.videos(&search).map(|l| paged(&l, video)),
            Medium::Image => self.api.images(&search).map(|l| paged(&l, image)),
            Medium::Lottie => return Ok(None),
        };
        match found {
            Ok(page) => Ok(page.candidates.into_iter().find(|one| one.id == id)),
            // Pixabay answers an id it has not got with a 404 and a sentence.
            Err(HttpError::Refused { status: 404, .. }) => Ok(None),
            Err(error) => Err(self.failure(&error)),
        }
    }

    fn download(&self, url: &str, limit: u64, to: &mut dyn Write) -> Result<u64, StockError> {
        // A file on Pixabay's CDN is public: nothing is signed, so no key
        // goes anywhere near it.
        http::stream_to(url, limit, to, &mut |_, _| {}).map_err(|error| self.failure(&error))
    }
}

/// A listing as a page of candidates.
fn paged<H>(listing: &Listing<H>, each: fn(&H) -> Candidate) -> Page {
    Page {
        candidates: listing.hits.iter().map(each).collect(),
        total: listing.total_hits,
    }
}

/// The tags, as words.
fn tags(text: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for word in text
        .split(',')
        .map(str::trim)
        .filter(|word| !word.is_empty())
    {
        if !words.iter().any(|had| had.eq_ignore_ascii_case(word)) {
            words.push(word.to_owned());
        }
    }
    words
}

/// A candidate with everything but its renditions and preview filled in.
fn candidate(medium: Medium, common: &wire::Common) -> Candidate {
    Candidate {
        medium,
        id: common.id,
        title: String::new(),
        style: common.kind.clone(),
        tags: tags(&common.tags),
        seconds: None,
        fps: None,
        author: common.user.clone(),
        page_url: common.page_url.clone(),
        preview_url: String::new(),
        motion_url: None,
        ai_generated: common.is_ai_generated,
        renditions: Vec::new(),
    }
}

/// A video hit as a candidate: every rendition it really has, smallest first.
fn video(hit: &VideoHit) -> Candidate {
    let named = [
        ("tiny", &hit.videos.tiny),
        ("small", &hit.videos.small),
        ("medium", &hit.videos.medium),
        ("large", &hit.videos.large),
    ];
    let mut renditions: Vec<Rendition> = named
        .iter()
        .filter(|(_, one)| !one.url.is_empty() && one.width > 0 && one.height > 0)
        .map(|(name, one)| Rendition {
            name: (*name).to_owned(),
            url: one.url.clone(),
            width: one.width,
            height: one.height,
            bytes: (one.size > 0).then_some(one.size),
        })
        .collect();
    renditions.sort_by_key(|one| u64::from(one.width) * u64::from(one.height));
    // The smallest thumbnail there is: a preview is looked at small.
    let preview = named
        .iter()
        .map(|(_, one)| one.thumbnail.as_str())
        .find(|url| !url.is_empty())
        .unwrap_or_default();
    Candidate {
        seconds: Some(hit.duration),
        preview_url: preview.to_owned(),
        renditions,
        ..candidate(Medium::Video, &hit.common)
    }
}

/// An image hit as a candidate.
///
/// Only `webformat` comes with its size; the others are the original scaled
/// to the longest side Pixabay documents for them, never past the original.
fn image(hit: &ImageHit) -> Candidate {
    let (width, height) = (hit.image_width, hit.image_height);
    let scaled = |side: u32| -> (u32, u32) {
        let longest = width.max(height);
        if longest <= side || longest == 0 {
            return (width, height);
        }
        let scale = |value: u32| {
            u32::try_from(u64::from(value) * u64::from(side) / u64::from(longest)).unwrap_or(value)
        };
        (scale(width), scale(height))
    };
    let mut renditions = Vec::new();
    let mut add = |name: &str, url: Option<&str>, size: (u32, u32)| {
        if let Some(url) = url.filter(|url| !url.is_empty()) {
            renditions.push(Rendition {
                name: name.to_owned(),
                url: url.to_owned(),
                width: size.0,
                height: size.1,
                bytes: None,
            });
        }
    };
    add(
        "webformat",
        Some(&hit.webformat_url),
        (hit.webformat_width, hit.webformat_height),
    );
    add(
        "large",
        Some(&hit.large_image_url),
        scaled(LARGE_IMAGE_SIDE),
    );
    add("fullhd", hit.full_hd_url.as_deref(), scaled(FULL_HD_SIDE));
    add("original", hit.image_url.as_deref(), (width, height));
    renditions.dedup_by(|one, two| one.width == two.width && one.height == two.height);
    Candidate {
        preview_url: hit.webformat_url.clone(),
        renditions,
        ..candidate(Medium::Image, &hit.common)
    }
}

/// The translation, read against bodies Pixabay actually sent.
#[cfg(test)]
mod tests {
    use super::*;

    const VIDEOS: &str = include_str!("../../fixtures/pixabay/videos.json");
    const IMAGES: &str = include_str!("../../fixtures/pixabay/images.json");

    #[test]
    fn a_video_keeps_every_rendition_smallest_first() {
        let listing: Listing<VideoHit> = serde_json::from_str(VIDEOS).unwrap();
        let page = paged(&listing, video);
        let first = &page.candidates[0];
        assert_eq!(first.id, 39009);
        assert_eq!(first.seconds, Some(11));
        let sizes: Vec<(u32, u32)> = first
            .renditions
            .iter()
            .map(|r| (r.width, r.height))
            .collect();
        assert_eq!(sizes, [(640, 360), (960, 540), (1280, 720), (1920, 1080)]);
        assert!(first.preview_url.ends_with("_tiny.jpg"));
        assert_eq!(first.tags[0], "cat");
        assert_eq!(page.total, 500);
    }

    #[test]
    fn a_picture_without_full_access_tops_out_at_1280() {
        let listing: Listing<ImageHit> = serde_json::from_str(IMAGES).unwrap();
        let first = image(&listing.hits[0]);
        let largest = first.largest().unwrap();
        assert_eq!(
            (largest.name.as_str(), largest.width, largest.height),
            ("large", 1280, 853)
        );
        assert_eq!(first.renditions.len(), 2);
    }

    #[test]
    fn repeated_tags_are_said_once() {
        assert_eq!(
            tags("seoul, city, Seoul, , night"),
            ["seoul", "city", "night"]
        );
    }

    #[test]
    fn a_refused_key_is_named_and_never_echoed() {
        let library = PixabayLibrary::new(&Secret::new("sekrit"));
        let refused = HttpError::Refused {
            url: String::from("https://pixabay.com/api/videos/"),
            status: 400,
            body: String::from("[ERROR 400] Invalid or missing API key sekrit"),
        };
        let said = library.failure(&refused).to_string();
        assert!(said.contains("refused the key"), "{said}");
        assert!(!said.contains("sekrit"), "{said}");
        let limited = HttpError::Refused {
            url: String::new(),
            status: 429,
            body: String::new(),
        };
        assert!(matches!(
            library.failure(&limited),
            StockError::RateLimited { .. }
        ));
    }
}
