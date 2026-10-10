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
    entries: (count: string, n: number) =>
      `${count} ${n === 1 ? "movimiento" : "movimientos"}, que suman`,
    empty: "Nada ha movido tu saldo todavía.",
    showMore: "Mostrar más",
    project: "Proyecto",
    allProjects: "Todos los proyectos",
    kind: "Tipo",
    everything: "Todo",
    from: "Desde (UTC)",
    to: "Hasta (UTC)",
    clear: "Quitar filtros",
  },
  history: {
    kinds: {
      veo_shot: "Generación de video",
      spoken_line: "Generación de voz",
      still_image: "Generación de imagen",
      voice_design: "Diseño de voz",
      assistant: "Asistente",
      top_up: "Recarga",
      monthly_fee: "Cuota mensual",
      refund: "Reembolso",
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
    balanceAfter: "Saldo después",
    deletedProject: "un proyecto eliminado",
    fileItMade: "el archivo que creó",
  },
};
