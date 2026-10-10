// Starting a project (#1016): the new-project dialog and the editor's
// platform-and-style control. Platform and style names come from the server
// and stay as they arrive.

export const start = {
  title: "New project",
  description: "Name it, and say what it is for if you know. Everything but the name can wait.",
  steps: {
    name: "Name",
    assets: "Files",
    platform: "Platform",
    style: "Style",
  },
  stepOf: (step: number, total: number) => `Step ${step} of ${total}`,
  name: "Project's name",
  nameMissing: "Write the name of the project",
  assetsHint: "Pick files from your library to bring into the project. You can add more later.",
  picked: (n: number) => (n === 1 ? "1 file picked" : `${n} files picked`),
  platformHint: "You can choose or change the platform later.",
  styleHint: "You can choose or change the style later.",
  noPlatform: "No platform",
  noStyle: "No style",
  everyStyle: "Every style is shown: choose a platform to see the ones made for it.",
  noPreview: "Preview coming soon",
  back: "Back",
  next: "Next",
  create: "Create",
  // The editor's control.
  button: "Platform and style",
  changeTitle: "Platform and style",
  changeDescription:
    "The assistant updates the script to match and tells you what changes. Nothing in the edit changes until you answer.",
  save: "Save",
  changedMessage: "I changed the platform or the style of this video.",
  notTold: (why: string) => `Saved, but the assistant could not be told: ${why}`,
};
