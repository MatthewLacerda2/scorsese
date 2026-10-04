// El inspector — los valores de un clip, la duración y el bucle de una
// secuencia de imágenes, y la descripción de un clip generado.

import type { Messages } from "@/i18n/catalogue";

export const inspector: Messages["inspector"] = {
  notInAssets: "no está en la lista de recursos",
  onTrack: (track: string) => `en la pista ${track}`,
  start: "Inicio",
  duration: "Duración",
  speed: "Velocidad",
  fit: "Ajuste",
  fits: {
    fit: "Encajar — imagen completa",
    fill: "Rellenar — sin barras",
    native: "Original — su propio tamaño en píxeles",
  },
  position: "Posición",
  positionTitle:
    "Cuánto se aleja de donde queda naturalmente: a la derecha y hacia abajo, en % del cuadro",
  rotation: "Rotación",
  scale: "Escala",
  animated: "Animado",
  points: (n: number) => (n === 1 ? "1 punto" : `${n} puntos`),
  byHand: "a mano",
  animationNote: "Para cambiar una animación, pídeselo al asistente.",
  animatedValue: "animado",
  animatedTitle: "Esto cambia a lo largo del clip — nada aquí lo dejará fijo",
  stretched: (wide: number, tall: number) => `${wide}% de ancho, ${tall}% de alto`,
  stretchedTitle: "Estirado a propósito — pídele al asistente un valor único",
  sequence: {
    title: "Secuencia",
    count: (stills: number, hold: number, frames: number) =>
      `${stills} ${stills === 1 ? "imagen" : "imágenes"} × ${hold} ${hold === 1 ? "cuadro" : "cuadros"} = ${frames} cuadros`,
    hold: "Duración",
    holdTitle: "Cuántos cuadros se queda cada imagen en pantalla",
    loop: "Repetir",
    loopTitle: "Volver a empezar desde la primera imagen, en vez de quedarse en la última",
  },
  brief: {
    title: "Descripción",
    noneYet: "todavía nada",
    note: "Para cambiar la descripción, o crearla, pídeselo al asistente.",
    labels: { prompt: "Prompt", line: "Frase", recipe: "Receta" },
    choices: {
      model: "Modelo",
      size: "Tamaño",
      length: "Duración",
      aspect: "Proporción",
      voice: "Voz",
      language: "Idioma",
    },
    states: {
      sketch: "aún no se ha hecho — se ve en la vista previa como una tarjeta, sin costo",
      queued: "haciéndose",
      generated: "hecho",
      stale:
        "la descripción cambió después de hacerse — se ve como una tarjeta hasta que se vuelva a hacer",
    },
  },
};
