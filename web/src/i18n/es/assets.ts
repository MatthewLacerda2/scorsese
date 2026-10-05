// La barra lateral de recursos del editor y el modal de la biblioteca, en
// español. Los IDs de recursos y los nombres de archivo son del usuario y se
// quedan como están.

import type { Messages } from "@/i18n/catalogue";

export const assets: Messages["assets"] = {
  heading: "Recursos",
  empty: "Nada todavía. Añade archivos de tu biblioteca y arrástralos a una pista.",
  alsoAlone: "también se usa sola",
  fold: (id) => `Ocultar las fotos de ${id}`,
  unfold: (id) => `Mostrar las fotos de ${id}`,
  drag: "Arrastra a una pista para colocarlo",
  photos: (name, n) => `${name} — ${n === 1 ? "1 foto" : `${n} fotos`}`,
  remove: (id) => `Quitar ${id}`,
  removeTitle: "Quitar del proyecto: los clips que lo usan también se van",
  state: { sketch: "boceto", queued: "en cola", generated: "generado", stale: "desactualizado" },
  library: {
    button: "Biblioteca",
    title: "Añadir desde tu biblioteca",
    description:
      "Elige un archivo para añadirlo a los recursos de este proyecto y luego arrástralo desde " +
      "ahí a una pista. Puedes subir archivos nuevos aquí o soltarlos en esta ventana.",
    notTrack: "no puede ir en una pista",
    inProject: "ya está en el proyecto",
  },
  kinds: {
    video: "video",
    image: "imagen",
    audio: "audio",
    text: "texto",
    color: "color",
    shape: "forma",
    icon: "icono",
    group: "grupo",
    image_sequence: "secuencia de imágenes",
    html: "página",
    generated_video: "video generado",
    generated_image: "imagen generada",
    generated_audio: "voz generada",
    synth_audio: "audio sintetizado",
  },
};
