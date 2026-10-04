// El explorador de archivos de la biblioteca, los detalles de un archivo y la
// bandeja de subidas, en español.

import type { Messages } from "@/i18n/catalogue";

export const files: Messages["files"] = {
  kinds: { video: "Video", image: "Imagen", audio: "Audio", midi: "MIDI" },
  sorts: {
    newest: "Más recientes",
    oldest: "Más antiguos",
    name: "Nombre",
    largest: "Más grandes",
  },
  browser: {
    label: "Archivos",
    search: "Buscar por nombre",
    kind: "Tipo",
    allKinds: "Todos los tipos",
    sort: "Ordenar",
    upload: "Subir",
    noMatch: "Ningún archivo coincide.",
    emptyLibrary:
      "Tu biblioteca está vacía. Sube videos, imágenes, sonidos y archivos MIDI — o suéltalos aquí — para usarlos en cualquier proyecto.",
    emptyProject: "Este proyecto todavía no usa archivos de tu biblioteca.",
    add: (name: string) => `Añadir ${name}`,
  },
  edit: (label: string) => `Editar ${label.toLowerCase()}`,
  viewer: {
    midi: "Un archivo MIDI son notas, no sonido. Pídele al asistente que musicalice tu video con él.",
  },
  download: {
    button: "Descargar",
    file: (name: string) => `Descargar ${name}`,
  },
  details: {
    notFound: "No encontrado",
    name: "Nombre",
    generated: "generado",
    view: "Ver",
    open: "Abrir",
    play: "Reproducir",
    size: "Tamaño",
    dimensions: "Dimensiones",
    duration: "Duración",
    added: "Añadido",
    description: "Descripción",
    descriptionPlaceholder: "Lo que el asistente lee al elegir un archivo",
    usedIn: "Se usa en",
    unused: "Ningún proyecto lo usa todavía.",
    templates: (names: string) => `Plantillas: ${names}`,
    delete: "Eliminar",
    deleteTitle: (name: string) => `¿Eliminar «${name}»?`,
    deleteDescription: "El archivo se elimina de tu biblioteca para siempre.",
  },
  generation: {
    title: {
      veo_shot: "Video generado",
      still_image: "Imagen generada",
      spoken_line: "Voz generada",
      voice_design: "Muestra de diseño de voz",
    },
    model: "Modelo",
    made: "Creado",
    cost: "Costo",
    free: "No se cobró nada",
    estimate: "Estimación",
    listedPrice: (amount: string) => `${amount} según el precio publicado del proveedor`,
    resolution: "Resolución",
    length: "Duración",
    seconds: (seconds: string) => `${seconds} s`,
    aspect: "Relación de aspecto",
    references: "Referencias",
    seed: "Semilla",
    voice: "Voz",
  },
  uploads: {
    title: "Subidas",
    clear: "Quitar terminadas",
    phase: {
      hashing: "Comprobando…",
      uploading: "Subiendo…",
      done: "Subido",
      duplicate: "Ya está en tu biblioteca",
      failed: "Falló",
    },
    alreadyHave: (name: string) => `ya tienes este archivo como «${name}»`,
  },
};
