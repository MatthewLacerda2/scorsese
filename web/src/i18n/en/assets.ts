// The editor's assets sidebar and its library modal, in English — the source
// the other catalogues are typed against. Asset ids and file names are the
// user's own and stay as written.

export const assets = {
  heading: "Assets",
  empty: "Nothing yet. Add files from your library, then drag them onto a track.",
  alsoAlone: "also used on its own",
  fold: (id: string) => `Fold ${id}'s photos away`,
  unfold: (id: string) => `Show ${id}'s photos`,
  drag: "Drag onto a track to place it",
  photos: (name: string, n: number) => `${name} — ${n === 1 ? "1 photo" : `${n} photos`}`,
  remove: (id: string) => `Remove ${id}`,
  removeTitle: "Remove from the project — the clips using it go too",
  state: { sketch: "sketch", queued: "queued", generated: "generated", stale: "stale" },
  library: {
    button: "Library",
    title: "Add from your library",
    description:
      "Pick a file to add it to this project's assets, then drag it from there onto a track. " +
      "New files can be uploaded here, or dropped on this window.",
    inProject: "in this project",
  },
  kinds: {
    video: "video",
    image: "image",
    audio: "audio",
    text: "text",
    color: "colour",
    shape: "shape",
    icon: "icon",
    group: "group",
    image_sequence: "image sequence",
    html: "page",
    generated_video: "generated video",
    generated_image: "generated still",
    generated_audio: "generated speech",
    synth_audio: "synthesised audio",
  },
};
