import type { Messages } from "@/i18n/catalogue";

// The signed-in home: its sidebar. Its headline is the landing's (`landing.hero`).

export const home: Messages["home"] = {
  menu: "Abrir el menú",
  nav: "Menú principal",
  projects: "Proyectos",
  noProjects: "Aún no hay proyectos.",
  options: (name) => `Opciones de ${name}`,
  // A project made from a prompt with no word to name it by; `day` is "24/10".
  untitled: (day) => `Nuevo video ${day}`,
  rename: "Renombrar",
  delete: "Eliminar",
  handOff: {
    starting: "Empezando tu proyecto…",
  },
};
