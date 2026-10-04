// The library's file browser, a file's details and the uploads tray, in
// English — the source the other catalogues are typed against. File names and
// what the server says (a refusal, a duplicate's sentence) stay as they come.

export const files = {
  kinds: { video: "Video", image: "Image", audio: "Audio", midi: "MIDI" },
  sorts: {
    newest: "Newest first",
    oldest: "Oldest first",
    name: "Name",
    largest: "Largest first",
  },
  browser: {
    label: "Files",
    search: "Search by name",
    kind: "Kind",
    allKinds: "All kinds",
    sort: "Sort",
    upload: "Upload",
    noMatch: "No files match.",
    emptyLibrary:
      "Your library is empty. Upload videos, pictures, sounds and MIDI files — or drop them here — to use in any project.",
    emptyProject: "This project uses no files from your library yet.",
    add: (name: string) => `Add ${name}`,
  },
  /** The button that turns a piece of text into a field; `label` is the field's name. */
  edit: (label: string) => `Edit ${label.toLowerCase()}`,
  viewer: {
    midi: "A MIDI file is notes, not sound. Ask the assistant to score your video with it.",
  },
  download: {
    button: "Download",
    file: (name: string) => `Download ${name}`,
  },
  details: {
    notFound: "Not found",
    name: "Name",
    generated: "generated",
    view: "View",
    open: "Open",
    play: "Play",
    size: "Size",
    dimensions: "Dimensions",
    duration: "Duration",
    added: "Added",
    description: "Description",
    descriptionPlaceholder: "Words the assistant reads when choosing a file",
    usedIn: "Used in",
    unused: "No project uses it yet.",
    templates: (names: string) => `Templates: ${names}`,
    delete: "Delete",
    deleteTitle: (name: string) => `Delete “${name}”?`,
    deleteDescription: "The file is removed from your library for good.",
  },
  generation: {
    title: {
      veo_shot: "Generated video",
      still_image: "Generated still",
      spoken_line: "Generated speech",
      voice_design: "Voice design sample",
    },
    model: "Model",
    made: "Made",
    cost: "Cost",
    free: "Nothing was charged",
    estimate: "Estimate",
    listedPrice: (amount: string) => `${amount} at the provider's listed price`,
    resolution: "Resolution",
    length: "Length",
    seconds: (seconds: string) => `${seconds} s`,
    aspect: "Aspect",
    references: "References",
    seed: "Seed",
    voice: "Voice",
  },
  uploads: {
    title: "Uploads",
    clear: "Clear finished",
    phase: {
      hashing: "Checking…",
      uploading: "Uploading…",
      done: "Uploaded",
      duplicate: "Already in your library",
      failed: "Failed",
    },
    alreadyHave: (name: string) => `you already have this as “${name}”`,
  },
};
