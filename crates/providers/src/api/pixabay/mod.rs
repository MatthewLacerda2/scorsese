//! Pixabay's search API: stock footage and photos, free (#900).
//!
//! Two endpoints and one envelope. `/api/` searches images, `/api/videos/`
//! searches videos, and both answer `{"total", "totalHits", "hits": [...]}` —
//! one [`Listing`] reads either, its hits typed per endpoint. Fetching one hit
//! by id is the same search with `id=` and nothing else.
//!
//! What is particular to this vendor, and checked against
//! <https://pixabay.com/api/docs/> on 2026-10-08:
//!
//! - **The key is a query parameter**, `key=`, never a header — so the
//!   [`Caller`] adds it to the request itself and no URL built here, and no
//!   error naming one, ever holds it.
//! - **A refusal is plain text**, `[ERROR 400] Invalid or missing API key`,
//!   with the status code a refusal deserves; 429 past the rate limit of 100
//!   requests a minute.
//! - **A video's renditions vary per video.** `large`, `medium`, `small`,
//!   `tiny`, each with its own width and height: `medium` was 1280×720 on one
//!   and 2560×1440 on another, a vertical video's are taller than wide, and a
//!   rendition Pixabay has not got arrives with an empty `url` and a zero size.
//!   So nothing here trusts a name for a size.
//! - **Image URLs past the 150 px preview are signed and expire in 24 hours**
//!   (`pixabay.com/get/…`), which is the same window Pixabay asks results to
//!   be cached for.

use serde::Deserialize;

use crate::api::http::{Caller, HttpError, encoded};
use crate::credentials::Secret;

/// Every Pixabay endpoint hangs off this.
const BASE: &str = "https://pixabay.com/api";

/// The query parameter Pixabay reads the key from.
const KEY_PARAMETER: &str = "key";

/// The most hits Pixabay puts on one page.
pub const MAX_PER_PAGE: u32 = 200;

/// The fewest hits Pixabay will put on one page: it refuses a smaller one.
pub const MIN_PER_PAGE: u32 = 3;

/// One search, in Pixabay's own parameter names.
///
/// Optional throughout, and an absent one is not sent — a filter with an empty
/// value is a different request from no filter. `orientation` is the images
/// endpoint's alone: the videos endpoint has none, and sending it there is
/// ignored, so it is left off.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Search {
    /// The words to search for, at most 100 characters.
    pub q: Option<String>,
    /// One hit by its id, instead of a search.
    pub id: Option<u64>,
    /// `film` or `animation` for videos; `photo`, `illustration` or
    /// `vector` for images.
    pub kind: Option<String>,
    /// `horizontal` or `vertical`, images only.
    pub orientation: Option<String>,
    /// `popular` (Pixabay's default) or `latest`.
    pub order: Option<String>,
    /// Which page, from 1.
    pub page: Option<u32>,
    /// Hits per page, [`MIN_PER_PAGE`] to [`MAX_PER_PAGE`].
    pub per_page: Option<u32>,
    /// Only results suitable for all ages.
    pub safesearch: bool,
}

impl Search {
    /// These parameters as the query string to hang off an endpoint, with
    /// `type_name` the endpoint's name for the kind (`video_type`,
    /// `image_type`).
    fn query(&self, type_name: &str, images: bool) -> String {
        let mut pairs: Vec<String> = Vec::new();
        let mut text = |name: &str, value: Option<&str>| {
            if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
                pairs.push(format!("{name}={}", encoded(value)));
            }
        };
        text("q", self.q.as_deref());
        text(type_name, self.kind.as_deref());
        if images {
            text("orientation", self.orientation.as_deref());
        }
        text("order", self.order.as_deref());
        if let Some(id) = self.id {
            pairs.push(format!("id={id}"));
        }
        if let Some(page) = self.page {
            pairs.push(format!("page={page}"));
        }
        if let Some(per_page) = self.per_page {
            pairs.push(format!("per_page={per_page}"));
        }
        if self.safesearch {
            pairs.push(String::from("safesearch=true"));
        }
        if pairs.is_empty() {
            String::new()
        } else {
            format!("?{}", pairs.join("&"))
        }
    }
}

/// What a search answers with.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Listing<H> {
    /// How many hits there are in all.
    #[serde(default)]
    pub total: u64,
    /// How many of them the API will hand out — at most 500 a query.
    #[serde(default, rename = "totalHits")]
    pub total_hits: u64,
    /// This page of them.
    #[serde(default = "Vec::new")]
    pub hits: Vec<H>,
}

/// What the parts of a hit both endpoints send have in common.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Common {
    /// The hit's id, unique within its endpoint — an image and a video may
    /// share one.
    pub id: u64,
    /// Its page on pixabay.com.
    #[serde(default, rename = "pageURL")]
    pub page_url: String,
    /// `film` or `animation`; `photo`, `illustration` or `vector`.
    #[serde(default, rename = "type")]
    pub kind: String,
    /// Comma-separated words.
    #[serde(default)]
    pub tags: String,
    /// Who published it.
    #[serde(default)]
    pub user: String,
    /// Whether Pixabay marks it as AI-generated.
    #[serde(default, rename = "isAiGenerated")]
    pub is_ai_generated: bool,
}

/// A video hit.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VideoHit {
    /// What every hit carries.
    #[serde(flatten)]
    pub common: Common,
    /// Seconds, whole.
    #[serde(default)]
    pub duration: u32,
    /// The renditions, by Pixabay's names for them.
    #[serde(default)]
    pub videos: Renditions,
}

/// A video's four renditions. Any of them may be missing — an empty `url`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Renditions {
    /// Usually 3840×2160, often 1920×1080 on an older video.
    #[serde(default)]
    pub large: Rendition,
    /// Usually 1920×1080.
    #[serde(default)]
    pub medium: Rendition,
    /// Usually 1280×720.
    #[serde(default)]
    pub small: Rendition,
    /// Usually 960×540 or 640×360.
    #[serde(default)]
    pub tiny: Rendition,
}

/// One rendition of a video.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Rendition {
    /// The MP4, or empty when there is no such rendition.
    #[serde(default)]
    pub url: String,
    /// Pixels.
    #[serde(default)]
    pub width: u32,
    /// Pixels.
    #[serde(default)]
    pub height: u32,
    /// Bytes.
    #[serde(default)]
    pub size: u64,
    /// A JPEG of the video at this rendition's size.
    #[serde(default)]
    pub thumbnail: String,
}

/// An image hit.
///
/// Without full API access the largest file offered is `largeImageURL`, at
/// most 1280 px on its longer side; `fullHDURL` and `imageURL` arrive only
/// once Pixabay grants that access, so they are optional here rather than
/// assumed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ImageHit {
    /// What every hit carries.
    #[serde(flatten)]
    pub common: Common,
    /// About 640 px on its longer side; signed, valid 24 hours.
    #[serde(default, rename = "webformatURL")]
    pub webformat_url: String,
    /// Pixels.
    #[serde(default, rename = "webformatWidth")]
    pub webformat_width: u32,
    /// Pixels.
    #[serde(default, rename = "webformatHeight")]
    pub webformat_height: u32,
    /// At most 1280 px on its longer side; signed, valid 24 hours.
    #[serde(default, rename = "largeImageURL")]
    pub large_image_url: String,
    /// At most 1920 px on its longer side — full API access only.
    #[serde(default, rename = "fullHDURL")]
    pub full_hd_url: Option<String>,
    /// The original — full API access only.
    #[serde(default, rename = "imageURL")]
    pub image_url: Option<String>,
    /// The original's width, in pixels.
    #[serde(default, rename = "imageWidth")]
    pub image_width: u32,
    /// The original's height, in pixels.
    #[serde(default, rename = "imageHeight")]
    pub image_height: u32,
}

/// Pixabay, reachable.
#[derive(Debug, Clone)]
pub struct Pixabay {
    caller: Caller,
}

impl Pixabay {
    /// One that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            caller: Caller::in_query(KEY_PARAMETER, key),
        }
    }

    /// The same client, copying every reply into `tap` — for the live
    /// provider check ([`crate::live`]).
    pub fn tapped(mut self, tap: &crate::api::tap::Tap) -> Self {
        self.caller = self.caller.tapped(tap);
        self
    }

    /// Searches the videos.
    pub fn videos(&self, search: &Search) -> Result<Listing<VideoHit>, HttpError> {
        self.caller.get(&format!(
            "{BASE}/videos/{}",
            search.query("video_type", false)
        ))
    }

    /// Searches the images.
    pub fn images(&self, search: &Search) -> Result<Listing<ImageHit>, HttpError> {
        self.caller
            .get(&format!("{BASE}/{}", search.query("image_type", true)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three video hits as Pixabay sent them on 2026-10-08: an older one
    /// whose `large` is 1920×1080, a newer one whose `medium` is 2560×1440,
    /// and a vertical one.
    const VIDEOS: &str = include_str!("../../../fixtures/pixabay/videos.json");

    /// Two image hits, their signed URLs' tokens replaced.
    const IMAGES: &str = include_str!("../../../fixtures/pixabay/images.json");

    #[test]
    fn a_video_listing_reads_every_rendition() {
        let listing: Listing<VideoHit> = serde_json::from_str(VIDEOS).unwrap();
        assert_eq!(listing.hits.len(), 3);
        let first = &listing.hits[0];
        assert_eq!(first.common.id, 39009);
        assert_eq!(first.duration, 11);
        assert_eq!(
            (first.videos.large.width, first.videos.large.height),
            (1920, 1080)
        );
        assert!(first.videos.tiny.thumbnail.ends_with(".jpg"));
        assert!(
            listing
                .hits
                .iter()
                .any(|hit| hit.videos.large.width == 3840)
        );
        assert!(
            listing
                .hits
                .iter()
                .any(|hit| hit.videos.large.height > hit.videos.large.width)
        );
    }

    #[test]
    fn an_image_listing_reads_without_full_access() {
        let listing: Listing<ImageHit> = serde_json::from_str(IMAGES).unwrap();
        let first = &listing.hits[0];
        assert_eq!(first.common.kind, "photo");
        assert!(first.large_image_url.ends_with("_1280.jpg"));
        assert_eq!(first.full_hd_url, None);
        assert_eq!((first.image_width, first.image_height), (6000, 4000));
    }

    #[test]
    fn a_search_sends_only_what_was_given() {
        assert_eq!(Search::default().query("video_type", false), "");
        let search = Search {
            q: Some(String::from("cat asleep")),
            kind: Some(String::from("film")),
            orientation: Some(String::from("vertical")),
            page: Some(2),
            per_page: Some(50),
            safesearch: true,
            ..Search::default()
        };
        assert_eq!(
            search.query("video_type", false),
            "?q=cat%20asleep&video_type=film&page=2&per_page=50&safesearch=true"
        );
        assert!(
            search
                .query("image_type", true)
                .contains("&orientation=vertical&")
        );
    }
}
