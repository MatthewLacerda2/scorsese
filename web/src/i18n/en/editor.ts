// The editor's strings — the page, rendering, removals, templates, the preview
// and the timeline — in English, the source the other catalogues are typed
// against. The inspector has its own area (`inspector.ts`).

/** `“a”, “b” and 3 more`: the clips a removal takes with it. */
const more = (named: string, rest: number) => `${named} and ${rest} more`;

export const editor = {
  page: {
    opening: "Opening the project…",
    frameShape: "Frame shape",
    render: "Render",
    renderTitle: "Render the video",
    renderDescription: "The whole cut as it is now, as an MP4 to download.",
    dismiss: "Dismiss",
    resizeAssets: "Resize the assets panel",
    resizeChat: "Resize the chat panel",
    resizeTimeline: "Resize the timeline",
  },
  /** An edit the server would not make. */
  refusal: {
    failed: "the edit could not be made",
    conflict:
      "The project changed while you were editing (the assistant, or another tab), so that edit was not made. Here is what is there now.",
  },
  render: {
    resolution: "Resolution",
    render: "Render",
    stop: "Stop",
    download: "Download",
    audio: "audio",
    none: "No renders kept for this project yet.",
    note: "Renders are kept for a while and made again on request; they cost no credits.",
    jobs: {
      waiting: "Queued behind other renders…",
      running: "Rendering…",
      done: "Ready — below.",
      failed: "The render failed",
      stuck: "The render is stuck",
      cancelled: "Stopped — nothing was kept",
    } as Record<string, string>,
  },
  /** The phase of a running job, after its percentage. */
  progress: {
    preparing: (percent: string) => `${percent} · preparing…`,
    mixing: (percent: string) => `${percent} · mixing the sound…`,
    finishing: (percent: string) => `${percent} · finishing the file…`,
  },
  removal: {
    more,
    asset: (asset: string) => `Remove “${asset}” from the project?`,
    assetClips: (asset: string, count: number, clips: string) =>
      `Remove “${asset}” from the project? ${count === 1 ? "This clip uses it and will be deleted too" : `These ${count} clips use it and will be deleted too`}: ${clips}.`,
    track: (track: string) => `Remove the track “${track}”?`,
    trackClips: (track: string, count: number, clips: string) =>
      `Remove the track “${track}”? ${count === 1 ? "The clip on it will be deleted too" : `The ${count} clips on it will be deleted too`}: ${clips}.`,
  },
  /** Where a generated asset is in its lifecycle, as one word. */
  states: {
    sketch: "sketch",
    queued: "queued",
    generated: "generated",
    stale: "stale",
  } as Record<string, string>,
  trackKinds: { video: "video", audio: "audio" } as Record<string, string>,
  timeline: {
    empty: "Drag something here from the left.",
    newTrack: "Drop here for a new track",
    removeTrack: (track: string) => `Remove the track ${track}`,
    removeTrackTitle: "Remove the track — the clips on it go too",
    clipTitle: (clip: string) =>
      `${clip} — drag to move (onto another lane too), drag an edge to trim, Shift-click to select several, Delete to remove`,
  },
  preview: {
    queued: "Preview queued…",
    rendering: "Rendering the preview…",
    ready: "Preview ready",
    failed: "The preview could not be rendered.",
    failedWhy: (why: string) => `The preview could not be rendered: ${why}`,
    empty: "Nothing on the timeline yet.",
    frame: "The frame under the playhead",
    start: "To the start",
    back: "One frame back",
    play: "Play",
    pause: "Pause",
    forward: "One frame forward",
    end: "To the end",
    scrub: "Scrub",
    quality: "Preview quality",
    qualities: {
      full: { label: "Full", says: "full quality: the render's own picture" },
      half: { label: "1/2", says: "half quality: fewer pixels, proxies where made" },
      quarter: { label: "1/4", says: "quarter quality: fastest, proxies where made" },
    },
  },
  templates: {
    title: "Templates",
    none: "None yet. Select clips on the timeline and save them as a template.",
    summary: (seconds: string, clips: number) =>
      `${seconds}s · ${clips === 1 ? "1 clip" : `${clips} clips`}`,
    insert: (name: string) => `Insert ${name} at the playhead`,
    insertTitle: (seconds: string) => `Insert at the playhead (${seconds}s)`,
    remove: (name: string) => `Delete ${name}`,
    removeTitle: "Delete the template — videos it went into keep their copies",
    confirmRemove: (name: string) => `Delete the template “${name}”?`,
    save: "Save as template",
    saveTitle: "Select clips on the timeline (Shift-click for several), then save them to reuse",
    saveDescription: (count: number) =>
      `The ${count === 1 ? "1 clip" : `${count} clips`} selected, to put into any of your projects later. Changing the template never changes a video it was used in.`,
    name: "Template name",
    placeholder: "Intro, outro, lower third…",
    replace: "Replace the template that has this name",
  },
};
