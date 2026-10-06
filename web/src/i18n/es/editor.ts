// El editor — la página, el renderizado, las eliminaciones, las plantillas, la
// vista previa y la línea de tiempo. El inspector está en `inspector.ts`.

import type { Messages } from "@/i18n/catalogue";

const more = (named: string, rest: number) => `${named} y ${rest} más`;

export const editor: Messages["editor"] = {
  page: {
    opening: "Abriendo el proyecto…",
    frameShape: "Formato del cuadro",
    render: "Renderizar",
    renderTitle: "Renderizar el video",
    renderDescription: "El montaje completo tal como está ahora, en MP4 para descargar.",
    dismiss: "Descartar",
  },
  refusal: {
    failed: "no se pudo hacer la edición",
    conflict:
      "El proyecto cambió mientras editabas (el asistente u otra pestaña), así que esa edición no se hizo. Esto es lo que hay ahora.",
  },
  render: {
    resolution: "Resolución",
    render: "Renderizar",
    stop: "Detener",
    download: "Descargar",
    audio: "audio",
    none: "Todavía no hay renderizados guardados para este proyecto.",
    note: "Los renderizados se guardan un tiempo y se vuelven a hacer cuando los pides; no cuestan créditos.",
    jobs: {
      waiting: "En cola, detrás de otros renderizados…",
      running: "Renderizando…",
      done: "Listo — está abajo.",
      failed: "El renderizado falló",
      stuck: "El renderizado se atascó",
      cancelled: "Detenido — no se guardó nada",
    },
  },
  progress: {
    preparing: (percent: string) => `${percent} · preparando…`,
    mixing: (percent: string) => `${percent} · mezclando el sonido…`,
    finishing: (percent: string) => `${percent} · terminando el archivo…`,
  },
  removal: {
    more,
    asset: (asset: string) => `¿Quitar “${asset}” del proyecto?`,
    assetClips: (asset: string, count: number, clips: string) =>
      `¿Quitar “${asset}” del proyecto? ${count === 1 ? "Este clip lo usa y también se eliminará" : `Estos ${count} clips lo usan y también se eliminarán`}: ${clips}.`,
    track: (track: string) => `¿Quitar la pista “${track}”?`,
    trackClips: (track: string, count: number, clips: string) =>
      `¿Quitar la pista “${track}”? ${count === 1 ? "El clip que tiene también se eliminará" : `Los ${count} clips que tiene también se eliminarán`}: ${clips}.`,
  },
  states: {
    sketch: "boceto",
    queued: "en cola",
    generated: "generado",
    stale: "desactualizado",
  },
  trackKinds: { video: "video", audio: "audio" },
  timeline: {
    empty: "Arrastra algo aquí desde la izquierda.",
    newTrack: "Suelta aquí para una pista nueva",
    removeTrack: (track: string) => `Quitar la pista ${track}`,
    removeTrackTitle: "Quitar la pista — los clips que tiene se van con ella",
    clipTitle: (clip: string) =>
      `${clip} — arrastra para mover (también a otra pista), arrastra un borde para recortar, Mayús+clic para seleccionar varios, Supr para quitar`,
  },
  preview: {
    queued: "Vista previa en cola…",
    rendering: "Renderizando la vista previa…",
    ready: "Vista previa lista",
    failed: "No se pudo renderizar la vista previa.",
    failedWhy: (why: string) => `No se pudo renderizar la vista previa: ${why}`,
    empty: "Todavía no hay nada en la línea de tiempo.",
    frame: "El cuadro bajo el cabezal",
    start: "Ir al inicio",
    back: "Un cuadro atrás",
    play: "Reproducir",
    pause: "Pausar",
    forward: "Un cuadro adelante",
    end: "Ir al final",
    scrub: "Recorrer",
    quality: "Calidad de la vista previa",
    qualities: {
      full: { label: "Completa", says: "calidad completa: la imagen del propio renderizado" },
      half: { label: "1/2", says: "media calidad: menos píxeles, con proxies donde los haya" },
      quarter: {
        label: "1/4",
        says: "un cuarto de calidad: la más rápida, con proxies donde los haya",
      },
    },
  },
  templates: {
    title: "Plantillas",
    none: "Todavía no hay ninguna. Selecciona clips en la línea de tiempo y guárdalos como plantilla.",
    summary: (seconds: string, clips: number) =>
      `${seconds}s · ${clips === 1 ? "1 clip" : `${clips} clips`}`,
    insert: (name: string) => `Insertar ${name} en el cabezal`,
    insertTitle: (seconds: string) => `Insertar en el cabezal (${seconds}s)`,
    remove: (name: string) => `Eliminar ${name}`,
    removeTitle: "Eliminar la plantilla — los videos donde se usó conservan sus copias",
    confirmRemove: (name: string) => `¿Eliminar la plantilla “${name}”?`,
    save: "Guardar como plantilla",
    saveTitle:
      "Selecciona clips en la línea de tiempo (Mayús+clic para varios) y guárdalos para reutilizarlos",
    saveDescription: (count: number) =>
      `${count === 1 ? "El clip seleccionado" : `Los ${count} clips seleccionados`}, para usarlos luego en cualquiera de tus proyectos. Cambiar la plantilla nunca cambia un video donde ya se usó.`,
    name: "Nombre de la plantilla",
    placeholder: "Intro, cierre, rótulo…",
    replace: "Reemplazar la plantilla que tiene este nombre",
  },
};
