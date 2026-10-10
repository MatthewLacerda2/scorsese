// Empezar un proyecto (#1016): la ventana de proyecto nuevo y el control de
// plataforma y estilo del editor, en español.

import type { Messages } from "@/i18n/catalogue";

export const start: Messages["start"] = {
  title: "Proyecto nuevo",
  description: "Ponle un nombre y, si ya lo sabes, di para dónde va. Todo lo demás puede esperar.",
  steps: {
    name: "Nombre",
    assets: "Archivos",
    platform: "Plataforma",
    style: "Estilo",
  },
  stepOf: (step: number, total: number) => `Paso ${step} de ${total}`,
  name: "Nombre del proyecto",
  nameMissing: "Escribe el nombre del proyecto",
  assetsHint: "Elige archivos de tu biblioteca para traer al proyecto. Puedes añadir más después.",
  picked: (n: number) => (n === 1 ? "1 archivo elegido" : `${n} archivos elegidos`),
  platformHint: "Puedes elegir o cambiar de plataforma después.",
  styleHint: "Puedes elegir o cambiar de estilo después.",
  noPlatform: "Sin plataforma",
  noStyle: "Sin estilo",
  everyStyle: "Se muestran todos los estilos: elige una plataforma para ver los hechos para ella.",
  noPreview: "Vista previa pronto",
  back: "Atrás",
  next: "Siguiente",
  create: "Crear",
  button: "Plataforma y estilo",
  changeTitle: "Plataforma y estilo",
  changeDescription:
    "El asistente actualiza el guion y te cuenta qué cambia. Nada en la edición cambia hasta que respondas.",
  save: "Guardar",
  changedMessage: "Cambié la plataforma o el estilo de este video.",
  notTold: (why: string) => `Guardado, pero no se pudo avisar al asistente: ${why}`,
};
