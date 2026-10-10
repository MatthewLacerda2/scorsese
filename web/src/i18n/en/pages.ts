// The projects list, a project's files and the spending history, in English —
// the source the other catalogues are typed against. Project names, memos and
// the server's own sentences stay as they come.

export const pages = {
  projectFiles: {
    noSuchProject: "There is no such project.",
    back: "Back to projects",
    openEditor: "Open it in the editor",
    // "…Upload new ones in the [library]." — the link sits between the two halves.
    intro: "The files from your library this project uses. Upload new ones in the",
    libraryLink: "library",
    introEnd: ".",
  },
  projects: {
    empty: "No projects yet. Create one to start.",
    create: "Create",
    name: "Project name",
    changed: (date: string) => `Changed ${date}`,
    open: "Open",
    files: "Files",
    delete: "Delete",
    deleteLabel: (name: string) => `Delete ${name}`,
    deleteTitle: (name: string) => `Delete “${name}”?`,
    deleteDescription:
      "The project is deleted for good. Its files stay in your library, and what it cost stays in your spending history.",
  },
  spending: {
    balance: "Balance",
    // Followed by the total in dollars and a full stop.
    entries: (count: string, n: number) =>
      `${count} ${n === 1 ? "entry" : "entries"}, adding up to`,
    empty: "Nothing has moved your balance yet.",
    showMore: "Show more",
    project: "Project",
    allProjects: "All projects",
    kind: "Kind",
    everything: "Everything",
    from: "From (UTC)",
    to: "To (UTC)",
    clear: "Clear filters",
  },
  history: {
    kinds: {
      veo_shot: "Video generation",
      spoken_line: "Speech generation",
      still_image: "Still generation",
      voice_design: "Voice design",
      assistant: "Assistant",
      top_up: "Top-up",
      monthly_fee: "Monthly fee",
      refund: "Refund",
    },
    status: {
      charged: "Charged",
      free: "Free: the provider failed",
      pending: "Pending",
      credited: "Credited",
    },
    when: "When",
    what: "What",
    amount: "Amount",
    balanceAfter: "Balance after",
    deletedProject: "a deleted project",
    fileItMade: "the file it made",
  },
};
