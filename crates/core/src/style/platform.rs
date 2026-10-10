//! Where a video is going, and the render that place asks for.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::asset::Aspect;

/// A placement a video is made for: a feed, and whether it runs there as a
/// post or as a paid ad.
///
/// Organic and ad are separate placements, not a flag on one, because they
/// are made by different rules — an ad has seconds to earn a call to action
/// and sits under the platform's own buttons, a post has a feed's patience.
///
/// A platform is a **render preset**, chosen per render the way every render
/// setting is. It never enters `project.json`: the same cut can be delivered
/// to two placements, and the document does not change between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    /// A YouTube video, watched landscape.
    Youtube,
    /// YouTube Shorts.
    YoutubeShorts,
    /// An Instagram Reel, posted.
    InstagramReels,
    /// An ad placed in Instagram Reels.
    InstagramReelsAd,
    /// An ad placed between Instagram Stories.
    InstagramStoriesAd,
    /// A TikTok, posted.
    Tiktok,
    /// An ad placed in the TikTok feed.
    TiktokAd,
}

impl Platform {
    /// Every placement, in the order they are listed to a person.
    pub const ALL: [Self; 7] = [
        Self::Youtube,
        Self::YoutubeShorts,
        Self::InstagramReels,
        Self::InstagramReelsAd,
        Self::InstagramStoriesAd,
        Self::Tiktok,
        Self::TiktokAd,
    ];

    /// The id a call, a flag or a stored setting names it by.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Youtube => "youtube",
            Self::YoutubeShorts => "youtube_shorts",
            Self::InstagramReels => "instagram_reels",
            Self::InstagramReelsAd => "instagram_reels_ad",
            Self::InstagramStoriesAd => "instagram_stories_ad",
            Self::Tiktok => "tiktok",
            Self::TiktokAd => "tiktok_ad",
        }
    }

    /// What a person is shown, in pt-BR — the web app's language.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Youtube => "YouTube",
            Self::YoutubeShorts => "YouTube Shorts",
            Self::InstagramReels => "Instagram Reels",
            Self::InstagramReelsAd => "Anúncio no Reels",
            Self::InstagramStoriesAd => "Anúncio nos Stories",
            Self::Tiktok => "TikTok",
            Self::TiktokAd => "Anúncio no TikTok",
        }
    }

    /// Whether the placement is paid: an ad, made by an ad's rules.
    pub const fn is_ad(self) -> bool {
        matches!(
            self,
            Self::InstagramReelsAd | Self::InstagramStoriesAd | Self::TiktokAd
        )
    }

    /// The shape the placement is watched in: landscape for YouTube, upright
    /// for every vertical feed.
    pub const fn aspect(self) -> Aspect {
        match self {
            Self::Youtube => Aspect::Wide,
            _ => Aspect::Tall,
        }
    }

    /// The pixel size a render for this placement is delivered at, as
    /// `(width, height)`: 1920×1080 landscape, 1080×1920 upright.
    ///
    /// Plain numbers rather than a renderer's resolution type, because this
    /// crate sits below the renderer; each surface builds its own from these.
    pub const fn size(self) -> (u32, u32) {
        match self.aspect() {
            Aspect::Wide => (1920, 1080),
            Aspect::Tall => (1080, 1920),
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for Platform {
    type Err = UnknownPlatform;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|platform| platform.id() == text)
            .ok_or_else(|| UnknownPlatform(text.to_owned()))
    }
}

/// A platform id that names no placement. Its message lists the ones that do,
/// so whoever typed it can pick one without looking anything up.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("no platform `{0}`; one of {list}", list = ids())]
pub struct UnknownPlatform(pub String);

/// Every platform id, comma-separated.
fn ids() -> String {
    Platform::ALL.map(Platform::id).join(", ")
}
