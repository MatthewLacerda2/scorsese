import type { Messages } from "@/i18n/catalogue";

// The signed-in home: its sidebar and its rotating title.

export const home: Messages["home"] = {
  // The title cycles through these, one after another.
  titles: ["Describe tu video", "¿Qué vamos a crear hoy?", "¿Cuál es tu idea?"],
  menu: "Abrir el menú",
  nav: "Menú principal",
  projects: "Proyectos",
  noProjects: "Aún no hay proyectos.",
  options: (name) => `Opciones de ${name}`,
  // A project made from a prompt with no word to name it by; `day` is "24/10".
  untitled: (day) => `Nuevo video ${day}`,
  rename: "Renombrar",
  delete: "Eliminar",
};
