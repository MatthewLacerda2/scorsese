// What the Render dialog asks for (#1070): the plain choices the desktop app's
// dialog offers — the kind of file, its size, its frame rate — and the
// soundtrack's loudness, which the server already takes. Each starts at the
// default the server would pick anyway, so pressing Render without touching
// anything is the request it always was; codecs stay on the container's own.

import type { RenderAsk } from "@/api";

/** Every container `docs/output-formats.md` delivers in, the default first. */
export const CONTAINERS = ["mp4", "mkv", "avi", "wmv", "mp3", "wav", "m4a"] as const;
export type Container = (typeof CONTAINERS)[number];

/** The containers that carry the soundtrack alone, with no picture to size or time. */
const SOUND_ONLY: ReadonlySet<Container> = new Set(["mp3", "wav", "m4a"]);

/** The rates offered besides the edit's own — the desktop app's list. */
export const RATES = ["24", "25", "30", "60"] as const;

/**
 * Where the soundtrack is headed, and the integrated loudness in LUFS that
 * suits it: `null` delivers the mix as balanced. Streaming platforms play
 * back at about −14; podcast apps at about −16.
 */
export const LOUDNESS = { mixed: null, streaming: -14, podcast: -16 } as const;
export type Loudness = keyof typeof LOUDNESS;

export interface Delivery {
  container: Container;
  /** `WIDTHxHEIGHT`, one of the project's delivery sizes. */
  resolution: string;
  /** One of [`RATES`], or `null` for the edit's own. */
  fps: string | null;
  loudness: Loudness;
}

/** Whether `container` is the soundtrack alone. */
export function soundOnly(container: Container): boolean {
  return SOUND_ONLY.has(container);
}

/**
 * The body `POST /api/projects/{id}/renders` gets for `delivery`. A sound-only
 * file says nothing about a picture — the server refuses a size or a rate on
 * one rather than ignore it — and every default is left unsaid.
 */
export function renderAsk(delivery: Delivery): RenderAsk {
  const ask: RenderAsk = {};
  if (delivery.container !== CONTAINERS[0]) ask.container = delivery.container;
  if (!soundOnly(delivery.container)) {
    ask.resolution = delivery.resolution;
    if (delivery.fps) ask.fps = delivery.fps;
  }
  const loudness = LOUDNESS[delivery.loudness];
  if (loudness !== null) ask.loudness = loudness;
  return ask;
}
