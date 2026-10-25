// The editor's strings — the page, rendering, removals, the preview
// and the timeline — in English, the source the other catalogues are typed
// against. The inspector has its own area (`inspector.ts`).

/** `“a”, “b” and 3 more`: the clips a removal takes with it. */
const more = (named: string, rest: number) => `${named} and ${rest} more`;

export const editor = {
  page: {
    opening: "Opening the project…",
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
    file: "File",
    video: (extension: string) => `Video — .${extension}`,
    sound: (extension: string) => `Sound only — .${extension}`,
    rate: "Frame rate",
    ownRate: (fps: string) => `Same as the edit (${fps} fps)`,
    fps: (fps: string) => `${fps} fps`,
    loudness: "Loudness",
    loudnesses: {
      mixed: "As mixed",
      streaming: "Streaming and social (−14 LUFS)",
      podcast: "Podcast (−16 LUFS)",
    },
    render: "Render",
    stop: "Stop",
    download: "Download",
    audio: "audio",
    none: "No renders kept for this project yet.",
    note: "Renders are kept for a while and made again on request; they cost no credits.",
    ready: "Your video is ready",
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
    capturing: (percent: string, done: number, of: number) =>
      `${percent} · capturing the web pages, frame ${done} of ${of}…`,
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
    empty: "Drag something here from the right.",
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
    failed: "The preview could not be made; try again.",
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
    maximize: "Full screen",
    minimize: "Leave full screen",
    quality: "Preview quality",
    qualities: {
      full: { label: "Full", says: "full quality: the render's own picture" },
      half: { label: "1/2", says: "half quality: fewer pixels, proxies where made" },
      quarter: { label: "1/4", says: "quarter quality: fastest, proxies where made" },
    },
  },
};
