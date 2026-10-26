import type { Messages } from "@/i18n/catalogue";

export const landing: Messages["landing"] = {
  meta: {
    title: "scorsese · El editor de video con el que conversas",
    description:
      "Promos, anuncios y videos de eventos para tu negocio, hechos a partir de tus archivos y una descripción. Sin contratar a un editor ni aprender un programa.",
  },
  openApp: "Abrir scorsese",
  signIn: "Iniciar sesión",
  hero: {
    title: "Crea tus videos ahora mismo",
    subtitle: "Describe el video. Nosotros lo armamos.",
    subtitleB: "No necesitas saber editar. Lo editamos por ti.",
    free: (dollars?: string) =>
      dollars
        ? `Empieza gratis: US$ ${dollars} al día, sin tarjeta.`
        : "Empieza gratis, sin tarjeta.",
  },
  how: {
    eyebrow: "Cómo funciona",
    title: "Tres pasos. Solo tienes que decir lo que quieres.",
    files: {
      title: "Envía tus archivos",
      body: "Fotos, clips, tu logo, la canción que te gusta. Lo que tengas, tal como está.",
    },
    describe: {
      title: "Describe el video",
      body: "Di para qué es y cómo debe sentirse, con tus palabras, como se lo explicarías a una persona.",
    },
    adjust: {
      title: "Míralo y ajústalo",
      body: "Mira el corte, pide cambios en una frase y descárgalo cuando esté como quieres.",
    },
  },
  showcase: {
    eyebrow: "Hecho con scorsese",
    label: "Videos hechos con scorsese",
    soundOn: "Activar el sonido",
    soundOff: "Silenciar",
    volume: "Volumen",
    play: "Reproducir",
    pause: "Pausar",
  },
  handles: {
    eyebrow: "Lo que hace por ti",
    title: "Todo lo que haría un editor.",
    music: {
      title: "Música",
      body: "Una banda sonora escrita para el corte, o tu canción, debajo de todo.",
    },
    titles: {
      title: "Títulos",
      body: "Texto en movimiento, con las fuentes y los colores de tu marca.",
    },
    cuts: {
      title: "Cortes",
      body: "Los mejores momentos de tu material, en un orden que cuenta la historia.",
    },
    pacing: {
      title: "Ritmo",
      body: "Más rápido o más lento, más corto o más largo, en una frase.",
    },
    narration: { title: "Narración", body: "Una voz que lee tu guion, al compás de la imagen." },
  },
  ask: { title: "Tu turno. Describe el video que necesitas." },
  closing: {
    title: "Cuando quieras.",
    body: "El video está a un mensaje de distancia",
  },
  footer: "Todos los videos de esta página se hicieron con scorsese.",
  dialog: {
    title: "Te damos la bienvenida a scorsese",
    description: "Inicia sesión para ver tus proyectos y archivos, o crea tu cuenta.",
  },
  legal: {
    terms: "Términos de uso",
    privacy: "Política de privacidad",
    pricingShort: "Precios",
    translation:
      "Esta es una traducción. El texto en portugués es el vinculante: si las versiones difieren, prevalece el texto en portugués.",
    readPortuguese: "Leer el texto en portugués",
    readTranslation: "Volver a la traducción",
  },
};
