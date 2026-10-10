// Las plataformas y los estilos con que empieza un proyecto (#1051), en
// español.

import type { Messages } from "@/i18n/catalogue";

export const menu: Messages["menu"] = {
  platforms: {
    youtube: "YouTube",
    youtube_shorts: "YouTube Shorts",
    instagram_reels: "Instagram Reels",
    instagram_reels_ad: "Anuncio en Reels",
    instagram_stories_ad: "Anuncio en Stories",
    tiktok: "TikTok",
    tiktok_ad: "Anuncio en TikTok",
  },
  styles: {
    narrated_captions: {
      name: "Narración en subtítulos",
      description:
        "La narración se vuelve el texto en pantalla, cada fragmento entrando en su primera palabra, sobre imágenes de apoyo. La voz manda en los cortes.",
    },
    kinetic_type: {
      name: "Tipografía en movimiento",
      description:
        "Palabras grandes y fuertes que entran al ritmo de la música. No hace falta ninguna grabación.",
    },
    whiteboard: {
      name: "Pizarra que se dibuja",
      description:
        "Una pizarra blanca que se dibuja sola mientras la narración explica la idea, trazo a trazo.",
    },
    flat_explainer: {
      name: "Explicador ilustrado",
      description:
        "Personajes ilustrados en estilo plano que representan una idea, con narración y escenarios sencillos.",
    },
    product_tour: {
      name: "Recorrido por el producto",
      description:
        "Las pantallas reales de tu producto, mostradas una a una con una narración que explica qué hace cada parte.",
    },
    photo_montage: {
      name: "Montaje de fotos",
      description: "Tus fotos con acercamientos lentos, cortadas al ritmo de la música.",
    },
    top_list: {
      name: "Lista / Top N",
      description:
        "Una cuenta regresiva, un elemento por escena, guardando lo mejor para el final.",
    },
    before_after: {
      name: "Antes y después",
      description:
        "El problema y luego el resultado — lado a lado o en corte — para que la diferencia hable sola.",
    },
    narrated_documentary: {
      name: "Documental narrado",
      description:
        "Imágenes cinematográficas bajo una voz que cuenta una historia, con música y pausas para respirar.",
    },
    testimonial: {
      name: "Testimonio",
      description:
        "Frases y reseñas de clientes en tarjetas, una a la vez, cerrando con tu llamado a la acción.",
    },
    pov_hook: {
      name: "POV",
      description:
        'Un texto gancho estilo meme ("POV: …") sobre la grabación, que hace que quien desliza el feed se detenga.',
    },
    flash_offer: {
      name: "Oferta relámpago",
      description: "El producto, el precio, la urgencia y el llamado a comprar, en pocos segundos.",
    },
    step_by_step: {
      name: "Paso a paso",
      description:
        "Un tutorial, un paso por escena, cada uno numerado y mostrado mientras se explica.",
    },
    numbers_story: {
      name: "Números que cuentan",
      description: "Números y gráficos animados que sostienen el argumento, un dato a la vez.",
    },
  },
};
