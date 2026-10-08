//! One piece of stock media, as an agent chooses it.

use serde::{Deserialize, Serialize};

/// Footage or a picture: what a search looks for and what an import becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Medium {
    /// Moving footage — a `video` asset once imported.
    Video,
    /// A still — an `image` asset once imported.
    Image,
}

impl Medium {
    /// The word a reply and an argument use.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Image => "image",
        }
    }

    /// The asset kind an import of this becomes.
    pub const fn asset_kind(self) -> scorsese_core::AssetKind {
        match self {
            Self::Video => scorsese_core::AssetKind::Video,
            Self::Image => scorsese_core::AssetKind::Image,
        }
    }
}

/// Which way a picture is longer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    /// Wider than tall.
    Horizontal,
    /// Taller than wide.
    Vertical,
}

impl Orientation {
    /// Whether `width`×`height` is this way round. A square is both.
    pub const fn fits(self, width: u32, height: u32) -> bool {
        match self {
            Self::Horizontal => width >= height,
            Self::Vertical => height >= width,
        }
    }

    /// The word the vendor and the arguments use.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}

/// One file a candidate can be downloaded as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rendition {
    /// The vendor's name for it — `large`, `medium` — which says nothing
    /// reliable about its size: that is what `width` and `height` are for.
    pub name: String,
    /// Where it is.
    pub url: String,
    /// Pixels.
    pub width: u32,
    /// Pixels.
    pub height: u32,
    /// Bytes, where the vendor said.
    #[serde(default)]
    pub bytes: Option<u64>,
}

impl Rendition {
    /// Whether this fills a `width`×`height` frame without being enlarged.
    pub const fn covers(&self, width: u32, height: u32) -> bool {
        self.width >= width && self.height >= height
    }
}

/// One search result.
///
/// Serialisable because it is what the 24-hour cache holds — and what a
/// later call, or a picker on the web (#901), resolves an id against without
/// asking the vendor again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// Footage or a picture.
    pub medium: Medium,
    /// The vendor's id for it, unique within its medium.
    pub id: u64,
    /// `film`, `animation`, `photo`, `illustration`, `vector`.
    pub style: String,
    /// The vendor's words for what is in it.
    pub tags: Vec<String>,
    /// Seconds long, for footage.
    #[serde(default)]
    pub seconds: Option<u32>,
    /// Who published it.
    pub author: String,
    /// Its page at the vendor.
    pub page_url: String,
    /// A small JPEG of it, to look at before choosing.
    pub preview_url: String,
    /// Whether the vendor marks it as AI-generated.
    #[serde(default)]
    pub ai_generated: bool,
    /// What it can be downloaded as, smallest first.
    pub renditions: Vec<Rendition>,
}

impl Candidate {
    /// The largest rendition there is.
    pub fn largest(&self) -> Option<&Rendition> {
        self.renditions.last()
    }

    /// The rendition to download for a `width`×`height` render: the smallest
    /// that fills it without being enlarged, or the largest when none does.
    ///
    /// By measured size, never by name — a vendor's `medium` was 1280×720 on
    /// one video and 2560×1440 on another.
    pub fn rendition_for(&self, width: u32, height: u32) -> Option<&Rendition> {
        self.renditions
            .iter()
            .find(|one| one.covers(width, height))
            .or_else(|| self.largest())
    }

    /// The line a listing prints for this candidate.
    pub fn says(&self) -> String {
        let mut line = format!("{} {}", self.medium.word(), self.id);
        if let Some(seconds) = self.seconds {
            line.push_str(&format!("  {seconds}s"));
        }
        if let Some(largest) = self.largest() {
            line.push_str(&format!("  up to {}x{}", largest.width, largest.height));
        }
        if self.style != "film" && self.style != "photo" && !self.style.is_empty() {
            line.push_str(&format!("  {}", self.style));
        }
        if self.ai_generated {
            line.push_str("  AI-generated");
        }
        line.push_str(&format!("  ({})", self.tags.join(", ")));
        line
    }

    /// What a contact sheet writes under its preview: its number in the
    /// reply and its id, and nothing else — a vertical cell is narrow, and
    /// the rest is in the words beside the sheet.
    pub fn label(&self, index: usize) -> String {
        format!("{}. {}", index + 1, self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendition(name: &str, width: u32, height: u32) -> Rendition {
        Rendition {
            name: name.to_owned(),
            url: format!("https://example.invalid/{name}.mp4"),
            width,
            height,
            bytes: None,
        }
    }

    fn candidate(renditions: Vec<Rendition>) -> Candidate {
        Candidate {
            medium: Medium::Video,
            id: 1,
            style: String::from("film"),
            tags: vec![String::from("cat")],
            seconds: Some(11),
            author: String::from("someone"),
            page_url: String::new(),
            preview_url: String::new(),
            ai_generated: false,
            renditions,
        }
    }

    #[test]
    fn the_smallest_rendition_that_fills_the_frame_is_chosen() {
        let shot = candidate(vec![
            rendition("tiny", 960, 540),
            rendition("small", 1280, 720),
            rendition("medium", 1920, 1080),
            rendition("large", 3840, 2160),
        ]);
        assert_eq!(shot.rendition_for(1920, 1080).unwrap().name, "medium");
        assert_eq!(shot.rendition_for(1280, 720).unwrap().name, "small");
        // A vertical cut of horizontal footage fills its height from 4K.
        assert_eq!(shot.rendition_for(1080, 1920).unwrap().name, "large");
    }

    #[test]
    fn short_of_the_frame_the_largest_is_chosen() {
        let shot = candidate(vec![
            rendition("tiny", 640, 360),
            rendition("large", 1920, 1080),
        ]);
        assert_eq!(shot.rendition_for(3840, 2160).unwrap().name, "large");
    }
}
