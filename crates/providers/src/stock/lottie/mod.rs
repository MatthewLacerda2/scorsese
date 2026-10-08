//! LottieFiles behind the trait (#903): its animations in, candidates out.
//!
//! Translation only, like [`super::PixabayLibrary`] — the wire is
//! [`api::lottiefiles`](crate::api::lottiefiles)'s. And [`keep`], what an
//! import of one is: not an asset, but a JSON file under `pages/` that a page
//! plays.
//!
//! **No key.** LottieFiles answers its public search anonymously, so nothing
//! here goes near the credential resolver.

mod keep;

pub use keep::{Kept, keep};

use std::io::Write;

use crate::api::http::{self, HttpError};
use crate::api::lottiefiles::{Animation, LottieFiles};

use super::PAGE_SIZE;
use super::candidate::{Candidate, Medium, Rendition};
use super::library::{Library, Page, Query, StockError};

/// What this library is called wherever a message names it.
const NAME: &str = "LottieFiles";

/// LottieFiles, as a library of animations.
#[derive(Debug, Clone, Default)]
pub struct LottieLibrary {
    api: LottieFiles,
}

impl LottieLibrary {
    /// One that asks anonymously.
    pub fn new() -> Self {
        Self::default()
    }
}

/// A transport error as the refusal it means.
fn failure(error: &HttpError) -> StockError {
    match error {
        HttpError::Refused { status: 429, .. } => StockError::RateLimited { library: NAME },
        other => StockError::Provider {
            library: NAME,
            said: other.to_string(),
        },
    }
}

impl Library for LottieLibrary {
    fn name(&self) -> &'static str {
        NAME
    }

    /// Page `page` is asked for as the first `page × PAGE_SIZE` results, the
    /// tail kept: the API pages by cursor, and a cursor is not something a
    /// stateless page number can hold. A search reads at most four pages, 200
    /// results, which is as many as the API hands out at once.
    fn page(&self, query: &Query, page: u32) -> Result<Page, StockError> {
        let page = page.max(1);
        let words: String = query.words.chars().take(100).collect();
        let found = self
            .api
            .search(&words, page.saturating_mul(PAGE_SIZE))
            .map_err(|error| failure(&error))?;
        let skip = usize::try_from((page - 1) * PAGE_SIZE).unwrap_or(usize::MAX);
        Ok(Page {
            candidates: found
                .edges
                .iter()
                .skip(skip)
                .filter_map(|edge| candidate(&edge.node))
                .collect(),
            total: found.total_count,
        })
    }

    fn one(&self, medium: Medium, id: u64) -> Result<Option<Candidate>, StockError> {
        if medium != Medium::Lottie {
            return Ok(None);
        }
        let found = self.api.one(id).map_err(|error| failure(&error))?;
        Ok(found.as_ref().and_then(candidate))
    }

    fn download(&self, url: &str, limit: u64, to: &mut dyn Write) -> Result<u64, StockError> {
        // LottieFiles' CDN is public: nothing is signed.
        http::stream_to(url, limit, to, &mut |_, _| {}).map_err(|error| failure(&error))
    }
}

/// Who published `animation`, in words: their name where they gave one, else
/// their handle.
fn author(animation: &Animation) -> String {
    let Some(by) = &animation.created_by else {
        return String::from("someone on LottieFiles");
    };
    let named: Vec<&str> = [&by.first_name, &by.last_name]
        .into_iter()
        .filter_map(|part| part.as_deref().map(str::trim))
        .filter(|part| !part.is_empty())
        .collect();
    if named.is_empty() {
        by.username
            .as_deref()
            .map(|name| name.trim_start_matches('/').to_owned())
            .unwrap_or_else(|| String::from("someone on LottieFiles"))
    } else {
        named.join(" ")
    }
}

/// A whole, positive number from one of the API's floats.
fn whole(value: Option<f64>) -> Option<u32> {
    value
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.round().clamp(1.0, f64::from(u32::MAX)) as u32)
}

/// An animation as a candidate — `None` for one with no JSON to download,
/// which nothing could play.
fn candidate(animation: &Animation) -> Option<Candidate> {
    let json = animation.json_url.clone().filter(|url| !url.is_empty())?;
    let metadata = animation.metadata.as_ref();
    let measured = |pick: fn(&crate::api::lottiefiles::Metadata) -> Option<f64>| {
        whole(metadata.and_then(pick))
    };
    let (width, height) = (measured(|m| m.width), measured(|m| m.height));
    let tags = animation
        .description
        .as_deref()
        .map(str::trim)
        .filter(|said| !said.is_empty())
        .map(|said| vec![said.chars().take(120).collect()])
        .unwrap_or_default();
    Some(Candidate {
        medium: Medium::Lottie,
        id: animation.id,
        title: animation.name.clone().unwrap_or_default(),
        style: String::new(),
        tags,
        seconds: measured(|m| m.duration),
        fps: measured(|m| m.frame_rate),
        author: author(animation),
        page_url: animation.url.clone().unwrap_or_default(),
        preview_url: animation.image_url.clone().unwrap_or_default(),
        motion_url: animation.video_url.clone().filter(|url| !url.is_empty()),
        animated_url: animation.gif_url.clone().filter(|url| !url.is_empty()),
        ai_generated: false,
        renditions: vec![Rendition {
            name: String::from("json"),
            url: json,
            width: width.unwrap_or_default(),
            height: height.unwrap_or_default(),
            bytes: animation.lottie_file_size,
        }],
    })
}

/// The translation, read against a body LottieFiles actually sent.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::lottiefiles::{Reply, Searched};

    const FOUND: &str = include_str!("../../../fixtures/lottiefiles/search.json");

    fn found() -> Vec<Candidate> {
        let reply: Reply<Searched> = serde_json::from_str(FOUND).unwrap();
        let found = reply.into_data().unwrap().found;
        found
            .edges
            .iter()
            .filter_map(|edge| candidate(&edge.node))
            .collect()
    }

    #[test]
    fn an_animation_is_a_candidate_with_its_json_as_the_one_rendition() {
        let kitty = found().into_iter().find(|one| one.id == 121035).unwrap();
        assert_eq!(kitty.medium, Medium::Lottie);
        assert_eq!(kitty.title, "Waving kitty");
        assert_eq!((kitty.seconds, kitty.fps), (Some(4), Some(48)));
        assert_eq!(kitty.author, "Kati");
        assert!(kitty.preview_url.ends_with(".png"));
        assert!(kitty.motion_url.as_deref().unwrap().ends_with(".mp4"));
        assert!(kitty.animated_url.as_deref().unwrap().ends_with(".gif"));
        let json = kitty.largest().unwrap();
        assert_eq!((json.width, json.height), (1291, 1200));
        assert!(json.url.ends_with(".json"));
        assert_eq!(json.bytes, Some(120_733));
        let line = kitty.says();
        assert!(
            line.starts_with("lottie 121035  \"Waving kitty\"  4s at 48 fps  up to 1291x1200"),
            "{line}"
        );
    }

    #[test]
    fn an_author_is_named_by_name_before_handle() {
        let mut animation: Animation = serde_json::from_value(serde_json::json!({
            "id": 1, "jsonUrl": "https://x/a.json",
            "createdBy": { "username": "/abc", "firstName": " ", "lastName": null }
        }))
        .unwrap();
        assert_eq!(author(&animation), "abc");
        animation.created_by = None;
        assert_eq!(author(&animation), "someone on LottieFiles");
        animation.json_url = None;
        assert!(candidate(&animation).is_none(), "nothing to play");
    }

    #[test]
    fn a_footage_id_is_not_asked_of_lottiefiles() {
        let library = LottieLibrary::new();
        assert!(library.one(Medium::Video, 1).unwrap().is_none());
    }
}
