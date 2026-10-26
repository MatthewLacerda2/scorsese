// The signed-in home: its sidebar. Its headline is the landing's (`landing.hero`).

export const home = {
  menu: "Open the menu",
  nav: "Main menu",
  projects: "Projects",
  noProjects: "No projects yet.",
  options: (name: string) => `Options for ${name}`,
  // A project made from a prompt with no word to name it by; `day` is "24/10".
  untitled: (day: string) => `New video ${day}`,
  rename: "Rename",
  delete: "Delete",
  /** The page a signed-out prompt lands on after the sign-in (#1535). */
  handOff: {
    starting: "Starting your project…",
  },
};
