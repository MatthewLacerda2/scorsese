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
        description: "A narração vira o texto na tela, cada trecho entrando na sua primeira \
                      palavra, sobre imagens de apoio. A voz comanda os cortes.",
        platforms: &[Shorts, Reels, ReelsAd, StoriesAd, Tiktok, TiktokAd],
        prompt: prompts::NARRATED_CAPTIONS,
    },
    Style {
        id: "kinetic_type",
        name: "Tipografia em movimento",
        description: "Palavras grandes e fortes entrando no ritmo da música. Não precisa \
                      de nenhuma filmagem.",
        platforms: EVERYWHERE,
        prompt: prompts::KINETIC_TYPE,
    },
    Style {
        id: "whiteboard",
        name: "Quadro que se desenha",
        description: "Um quadro branco que se desenha sozinho enquanto a narração explica \
                      a ideia, traço a traço.",
        platforms: &[Youtube, Shorts, Reels, Tiktok],
        prompt: prompts::WHITEBOARD,
    },
    Style {
        id: "flat_explainer",
        name: "Explicador ilustrado",
        description: "Personagens ilustrados em estilo flat encenando uma ideia, com \
                      narração e cenários simples.",
        platforms: &[Youtube, Shorts, Reels, ReelsAd, Tiktok],
        prompt: prompts::FLAT_EXPLAINER,
    },
    Style {
        id: "product_tour",
        name: "Tour do produto",
        description: "As telas reais do seu produto, mostradas uma a uma com uma narração \
                      que explica o que cada parte faz.",
        platforms: &[Youtube, Shorts, ReelsAd, StoriesAd, TiktokAd],
        prompt: prompts::PRODUCT_TOUR,
    },
    Style {
        id: "photo_montage",
        name: "Montagem de fotos",
        description: "Suas fotos com aproximações lentas, cortadas no ritmo da música.",
        platforms: &[Youtube, Shorts, Reels, StoriesAd, Tiktok],
        prompt: prompts::PHOTO_MONTAGE,
    },
    Style {
        id: "top_list",
        name: "Lista / Top N",
        description: "Uma contagem regressiva, um item por cena, guardando o melhor para \
                      o final.",
        platforms: &[Youtube, Shorts, Reels, Tiktok],
        prompt: prompts::TOP_LIST,
    },
    Style {
        id: "before_after",
        name: "Antes e depois",
        description: "O problema, depois o resultado — lado a lado ou em corte — para a \
                      diferença falar sozinha.",
        platforms: &[Reels, ReelsAd, StoriesAd, Tiktok, TiktokAd],
        prompt: prompts::BEFORE_AFTER,
    },
    Style {
        id: "narrated_documentary",
        name: "Documentário narrado",
        description: "Imagens cinematográficas sob uma voz que conta uma história, com \
                      trilha e respiros.",
        platforms: &[Youtube, Shorts, Reels],
        prompt: prompts::NARRATED_DOCUMENTARY,
    },
    Style {
        id: "testimonial",
        name: "Depoimento",
        description: "Frases e avaliações de clientes em cartões, uma de cada vez, \
                      fechando com o seu chamado.",
        platforms: &[Youtube, ReelsAd, StoriesAd, TiktokAd],
        prompt: prompts::TESTIMONIAL,
    },
    Style {
        id: "pov_hook",
        name: "POV",
        description: "Um texto de gancho em estilo meme (\"POV: …\") sobre a filmagem, \
                      que faz quem rola o feed parar.",
        platforms: &[Shorts, Reels, Tiktok, TiktokAd],
        prompt: prompts::POV_HOOK,
    },
    Style {
        id: "flash_offer",
        name: "Oferta relâmpago",
        description: "O produto, o preço, a urgência e o chamado para comprar, em poucos \
                      segundos.",
        platforms: &[ReelsAd, StoriesAd, TiktokAd],
        prompt: prompts::FLASH_OFFER,
    },
    Style {
        id: "step_by_step",
        name: "Passo a passo",
        description: "Um tutorial, um passo por cena, cada um numerado e mostrado \
                      enquanto é explicado.",
        platforms: &[Youtube, Shorts, Reels, Tiktok],
        prompt: prompts::STEP_BY_STEP,
    },
    Style {
        id: "numbers_story",
        name: "Números que contam",
        description: "Números e gráficos animados carregando o argumento, um dado de \
                      cada vez.",
        platforms: &[Youtube, Shorts, ReelsAd],
        prompt: prompts::NUMBERS_STORY,
    },
];
