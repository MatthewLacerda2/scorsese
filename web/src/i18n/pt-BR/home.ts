import type { Messages } from "@/i18n/catalogue";

// The signed-in home: its sidebar and its rotating title.

export const home: Messages["home"] = {
  // The title cycles through these, one after another.
  titles: ["Descreva seu vídeo", "O que vamos criar hoje?", "Qual é a sua ideia?"],
  menu: "Abrir o menu",
  nav: "Menu principal",
  projects: "Projetos",
  noProjects: "Nenhum projeto ainda.",
  options: (name) => `Opções de ${name}`,
  // A project made from a prompt with no word to name it by; `day` is "24/10".
  untitled: (day) => `Novo vídeo ${day}`,
  rename: "Renomear",
  delete: "Excluir",
};
