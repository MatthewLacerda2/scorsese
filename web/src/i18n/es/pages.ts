// La lista de proyectos, los archivos de un proyecto y el historial de gastos,
// en español.

import type { Messages } from "@/i18n/catalogue";

export const pages: Messages["pages"] = {
  projectFiles: {
    noSuchProject: "Ese proyecto no existe.",
    back: "Volver a los proyectos",
    openEditor: "Abrirlo en el editor",
    intro: "Los archivos de tu biblioteca que usa este proyecto. Sube archivos nuevos en la",
    libraryLink: "biblioteca",
    introEnd: ".",
  },
  projects: {
    empty: "Todavía no hay proyectos. Pulsa Crear para empezar.",
    create: "Crear",
    newName: "Nombre del proyecto nuevo",
    name: "Nombre del proyecto",
    changed: (date: string) => `Modificado el ${date}`,
    open: "Abrir",
    files: "Archivos",
    delete: "Eliminar",
    deleteLabel: (name: string) => `Eliminar ${name}`,
    deleteTitle: (name: string) => `¿Eliminar «${name}»?`,
    deleteDescription:
      "El proyecto se elimina para siempre. Sus archivos se quedan en tu biblioteca, y lo que costó sigue en tu historial de gastos.",
  },
  spending: {
    balance: "Saldo",
    empty: "Nada ha movido tu saldo todavía.",
    showMore: "Mostrar más",
    project: "Proyecto",
    allProjects: "Todos los proyectos",
    kind: "Tipo",
    everything: "Todo",
    from: "Desde",
    to: "Hasta",
    clear: "Quitar filtros",
  },
  history: {
    kinds: {
      veo_shot: "Generación de video",
      spoken_line: "Generación de voz",
      music_track: "Generación de música",
      still_image: "Generación de imagen",
      voice_design: "Diseño de voz",
      assistant: "Asistente",
      top_up: "Recarga",
      refund: "Reembolso",
      adjustment: "Ajuste",
    },
    status: {
      charged: "Cobrado",
      free: "Gratis: el proveedor falló",
      pending: "Pendiente",
      credited: "Acreditado",
    },
    when: "Cuándo",
    what: "Qué",
    amount: "Importe",
    deletedProject: "un proyecto eliminado",
    fileItMade: "el archivo que creó",
    freePart: (dollars: string) => `${dollars} del crédito gratis`,
  },
};
