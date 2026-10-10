// As plataformas e os estilos de um projeto (#1051), em português do Brasil:
// as mesmas palavras da biblioteca em `scorsese_core::style`, e o teste
// `start/words.test.ts` as mantém iguais.

import type { Messages } from "@/i18n/catalogue";

export const menu: Messages["menu"] = {
  platforms: {
    youtube: "YouTube",
    youtube_shorts: "YouTube Shorts",
    instagram_reels: "Instagram Reels",
    instagram_reels_ad: "Anúncio no Reels",
    instagram_stories_ad: "Anúncio nos Stories",
    tiktok: "TikTok",
    tiktok_ad: "Anúncio no TikTok",
  },
  styles: {
    narrated_captions: {
      name: "Narração em legenda",
      description:
        "A narração vira o texto na tela, cada trecho entrando na sua primeira palavra, sobre imagens de apoio. A voz comanda os cortes.",
    },
    kinetic_type: {
      name: "Tipografia em movimento",
      description:
        "Palavras grandes e fortes entrando no ritmo da música. Não precisa de nenhuma filmagem.",
    },
    whiteboard: {
      name: "Quadro que se desenha",
      description:
        "Um quadro branco que se desenha sozinho enquanto a narração explica a ideia, traço a traço.",
    },
    flat_explainer: {
      name: "Explicador ilustrado",
      description:
        "Personagens ilustrados em estilo flat encenando uma ideia, com narração e cenários simples.",
    },
    product_tour: {
      name: "Tour do produto",
      description:
        "As telas reais do seu produto, mostradas uma a uma com uma narração que explica o que cada parte faz.",
    },
    photo_montage: {
      name: "Montagem de fotos",
      description: "Suas fotos com aproximações lentas, cortadas no ritmo da música.",
    },
    top_list: {
      name: "Lista / Top N",
      description: "Uma contagem regressiva, um item por cena, guardando o melhor para o final.",
    },
    before_after: {
      name: "Antes e depois",
      description:
        "O problema, depois o resultado — lado a lado ou em corte — para a diferença falar sozinha.",
    },
    narrated_documentary: {
      name: "Documentário narrado",
      description:
        "Imagens cinematográficas sob uma voz que conta uma história, com trilha e respiros.",
    },
    testimonial: {
      name: "Depoimento",
      description:
        "Frases e avaliações de clientes em cartões, uma de cada vez, fechando com o seu chamado.",
    },
    pov_hook: {
      name: "POV",
      description:
        'Um texto de gancho em estilo meme ("POV: …") sobre a filmagem, que faz quem rola o feed parar.',
    },
    flash_offer: {
      name: "Oferta relâmpago",
      description: "O produto, o preço, a urgência e o chamado para comprar, em poucos segundos.",
    },
    step_by_step: {
      name: "Passo a passo",
      description:
        "Um tutorial, um passo por cena, cada um numerado e mostrado enquanto é explicado.",
    },
    numbers_story: {
      name: "Números que contam",
      description: "Números e gráficos animados carregando o argumento, um dado de cada vez.",
    },
  },
};
