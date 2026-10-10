// The platforms and styles a project starts from (#1051), by the id
// `GET /api/styles` gives them. The server sends the library's pt-BR; the
// menu is shown in these words instead, and a platform or style missing here
// keeps the server's. `start/words.test.ts` holds every catalogue to the
// server's list (`start/menu.json`), so a style added to the library without
// its words here fails the web gate.

export const menu = {
  platforms: {
    youtube: "YouTube",
    youtube_shorts: "YouTube Shorts",
    instagram_reels: "Instagram Reels",
    instagram_reels_ad: "Reels ad",
    instagram_stories_ad: "Stories ad",
    tiktok: "TikTok",
    tiktok_ad: "TikTok ad",
  },
  styles: {
    narrated_captions: {
      name: "Narrated captions",
      description:
        "The narration becomes the text on screen, each line arriving on its first word, over supporting footage. The voice drives the cuts.",
    },
    kinetic_type: {
      name: "Kinetic type",
      description: "Big, bold words landing on the beat of the music. No footage needed.",
    },
    whiteboard: {
      name: "Board that draws itself",
      description:
        "A whiteboard that draws itself while the narration explains the idea, stroke by stroke.",
    },
    flat_explainer: {
      name: "Illustrated explainer",
      description:
        "Flat-style illustrated characters acting out an idea, with narration and simple scenery.",
    },
    product_tour: {
      name: "Product tour",
      description:
        "Your product's real screens, shown one by one, with narration explaining what each part does.",
    },
    photo_montage: {
      name: "Photo montage",
      description: "Your photos with slow zooms, cut to the beat of the music.",
    },
    top_list: {
      name: "Top N list",
      description: "A countdown, one item per scene, saving the best for last.",
    },
    before_after: {
      name: "Before and after",
      description:
        "The problem, then the result — side by side or as a cut — so the difference speaks for itself.",
    },
    narrated_documentary: {
      name: "Narrated documentary",
      description:
        "Cinematic footage under a voice telling a story, with a score and room to breathe.",
    },
    testimonial: {
      name: "Testimonial",
      description:
        "Customer quotes and reviews on cards, one at a time, closing with your call to action.",
    },
    pov_hook: {
      name: "POV",
      description:
        'A meme-style hook line ("POV: …") over the footage that stops whoever is scrolling the feed.',
    },
    flash_offer: {
      name: "Flash offer",
      description: "The product, the price, the urgency and the call to buy, in a few seconds.",
    },
    step_by_step: {
      name: "Step by step",
      description: "A tutorial, one step per scene, each numbered and shown as it is explained.",
    },
    numbers_story: {
      name: "Numbers that tell",
      description: "Animated numbers and charts carrying the argument, one figure at a time.",
    },
  },
};
