//! The first library: fourteen kinds of video common among those made to earn
//! money, each suited to the placements it is actually made for (#1014).
//!
//! Every placement has at least six. The order is the order they are shown.

use super::Style;
use super::platform::Platform::{
    self, InstagramReels as Reels, InstagramReelsAd as ReelsAd, InstagramStoriesAd as StoriesAd,
    Tiktok, TiktokAd, Youtube, YoutubeShorts as Shorts,
};
use super::prompts;

/// Every placement, for a style made for all of them.
const EVERYWHERE: &[Platform] = &Platform::ALL;

/// The whole library, in the order it is shown.
pub static STYLES: &[Style] = &[
    Style {
        id: "narrated_captions",
        name: "Narração em legenda",
        platforms: &[Shorts, Reels, ReelsAd, StoriesAd, Tiktok, TiktokAd],
        prompt: prompts::NARRATED_CAPTIONS,
    },
    Style {
        id: "kinetic_type",
        name: "Tipografia em movimento",
        platforms: EVERYWHERE,
        prompt: prompts::KINETIC_TYPE,
    },
    Style {
        id: "whiteboard",
        name: "Quadro que se desenha",
        platforms: &[Youtube, Shorts, Reels, Tiktok],
        prompt: prompts::WHITEBOARD,
    },
    Style {
        id: "flat_explainer",
        name: "Explicador ilustrado",
        platforms: &[Youtube, Shorts, Reels, ReelsAd, Tiktok],
        prompt: prompts::FLAT_EXPLAINER,
    },
    Style {
        id: "product_tour",
        name: "Tour do produto",
        platforms: &[Youtube, Shorts, ReelsAd, StoriesAd, TiktokAd],
        prompt: prompts::PRODUCT_TOUR,
    },
    Style {
        id: "photo_montage",
        name: "Montagem de fotos",
        platforms: &[Youtube, Shorts, Reels, StoriesAd, Tiktok],
        prompt: prompts::PHOTO_MONTAGE,
    },
    Style {
        id: "top_list",
        name: "Lista / Top N",
        platforms: &[Youtube, Shorts, Reels, Tiktok],
        prompt: prompts::TOP_LIST,
    },
    Style {
        id: "before_after",
        name: "Antes e depois",
        platforms: &[Reels, ReelsAd, StoriesAd, Tiktok, TiktokAd],
        prompt: prompts::BEFORE_AFTER,
    },
    Style {
        id: "narrated_documentary",
        name: "Documentário narrado",
        platforms: &[Youtube, Shorts, Reels],
        prompt: prompts::NARRATED_DOCUMENTARY,
    },
    Style {
        id: "testimonial",
        name: "Depoimento",
        platforms: &[Youtube, ReelsAd, StoriesAd, TiktokAd],
        prompt: prompts::TESTIMONIAL,
    },
    Style {
        id: "pov_hook",
        name: "POV",
        platforms: &[Shorts, Reels, Tiktok, TiktokAd],
        prompt: prompts::POV_HOOK,
    },
    Style {
        id: "flash_offer",
        name: "Oferta relâmpago",
        platforms: &[ReelsAd, StoriesAd, TiktokAd],
        prompt: prompts::FLASH_OFFER,
    },
    Style {
        id: "step_by_step",
        name: "Passo a passo",
        platforms: &[Youtube, Shorts, Reels, Tiktok],
        prompt: prompts::STEP_BY_STEP,
    },
    Style {
        id: "numbers_story",
        name: "Números que contam",
        platforms: &[Youtube, Shorts, ReelsAd],
        prompt: prompts::NUMBERS_STORY,
    },
];
