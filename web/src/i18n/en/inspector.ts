// The inspector's strings — a clip's plain values, an image sequence's hold
// and loop, and a generated clip's brief — in English, the source the other
// catalogues are typed against.

export const inspector = {
  notInAssets: "not in the assets table",
  onTrack: (track: string) => `on track ${track}`,
  start: "Start",
  duration: "Duration",
  speed: "Speed",
  fit: "Fit",
  fits: {
    fit: "Fit — whole picture",
    fill: "Fill — no bars",
    native: "Native — own pixel size",
  },
  position: "Position",
  positionTitle: "How far from where it naturally sits: right and down, in % of the frame",
  rotation: "Rotation",
  scale: "Scale",
  animated: "Animated",
  points: (n: number) => (n === 1 ? "1 point" : `${n} points`),
  byHand: "by hand",
  animationNote: "Changing an animation is a sentence to the assistant.",
  animatedValue: "animated",
  animatedTitle: "This changes over the clip — nothing here will flatten it",
  stretched: (wide: number, tall: number) => `${wide}% wide, ${tall}% tall`,
  stretchedTitle: "Stretched on purpose — ask the assistant for a single value",
  sequence: {
    title: "Sequence",
    count: (stills: number, hold: number, frames: number) =>
      `${stills} ${stills === 1 ? "still" : "stills"} × ${hold} ${hold === 1 ? "frame" : "frames"} = ${frames} frames`,
    hold: "Hold",
    holdTitle: "How many frames each still stays on screen",
    loop: "Loop",
    loopTitle: "Start again from the first still, instead of holding the last",
  },
  brief: {
    title: "Brief",
    noneYet: "none yet",
    note: "Changing the brief, or making it, is a sentence to the assistant.",
    labels: { prompt: "Prompt", line: "Line", recipe: "Recipe" },
    choices: {
      model: "Model",
      size: "Size",
      length: "Length",
      aspect: "Aspect",
      voice: "Voice",
      language: "Language",
    },
    /** What each lifecycle state means to somebody about to press GO. */
    states: {
      sketch: "not made yet — previews as a slug card, at no cost",
      queued: "being made",
      generated: "made",
      stale: "the brief changed since it was made — previews as a slug card until it is made again",
    } as Record<string, string>,
  },
};
