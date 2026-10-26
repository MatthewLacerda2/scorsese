import type { Messages } from "@/i18n/catalogue";

// The signed-in home: its sidebar. Its headline is the landing's (`landing.hero`).

export const home: Messages["home"] = {
  menu: "Abrir o menu",
  nav: "Menu principal",
  projects: "Projetos",
  noProjects: "Nenhum projeto ainda.",
  options: (name) => `Opções de ${name}`,
  // A project made from a prompt with no word to name it by; `day` is "24/10".
  untitled: (day) => `Novo vídeo ${day}`,
  rename: "Renomear",
  delete: "Excluir",
  handOff: {
    starting: "Começando seu projeto…",
  },
};
