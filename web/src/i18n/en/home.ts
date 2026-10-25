// The signed-in home: its sidebar and its rotating title.

export const home = {
  // The title cycles through these, one after another.
  titles: ["Describe your video", "What shall we create today?", "What is your idea?"],
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
