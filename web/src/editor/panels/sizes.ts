// How big the editor's three panels around the preview are (#863): the chat
// on the left, the assets on the right (#943), the timeline along the bottom. Each one
// is dragged by its inner edge, and the preview takes up the difference.
//
// Sizes are in rem, because what a panel holds is text — asset names, the
// chat, track names — and its useful size goes with the font, not the screen.
// Each panel ranges from 0.3× to 1.3× of today's size (the maintainer: 40%
// larger is too much, 30% is small enough). What does differ between a laptop
// and a desktop is how much room that leaves the preview, so the preview's
// minimum is a share of the editor's own width and height: whatever the
// panels want, the preview keeps it, on any screen.
//
// Sizes are a person's screen, not part of the edit, so they are kept per
// browser in `localStorage`, read and written inside try/catch as the frame
// shape is (`../shape.ts`), and never in the project.

export type Panel = "assets" | "chat" | "timeline";

export type Sizes = Record<Panel, number>;

/** The editor's area in rem, once it has been measured. */
export interface Room {
  width: number;
  height: number;
}

/** Today's sizes, in rem: where each panel starts, and where a double-click puts it back. */
export const DEFAULTS: Sizes = { assets: 15, chat: 22, timeline: 15 };

/** A panel's least and most, as multiples of its default. */
export const LEAST = 0.3;
export const MOST = 1.3;

/** The share of the editor's width, and of its height, the preview always keeps. */
export const PREVIEW_SHARE = 0.4;

/** A handle's own strip, in rem: it sits between two panels, over neither. */
export const HANDLE = 0.375;

const within = (value: number, least: number, most: number) =>
  Math.min(Math.max(value, least), most);

const least = (panel: Panel) => DEFAULTS[panel] * LEAST;

/** What the panels sharing `panel`'s direction may take together, leaving the preview its share. */
function budget(panel: Panel, room: Room): number {
  return panel === "timeline"
    ? room.height * (1 - PREVIEW_SHARE) - HANDLE
    : room.width * (1 - PREVIEW_SHARE) - 2 * HANDLE;
}

/**
 * `panel` dragged to `wanted`: held inside its range, and below what leaves
 * the preview its share — the preview's minimum wins over a panel's most,
 * never over its least. The other panels stay where they are.
 */
export function resize(sizes: Sizes, panel: Panel, wanted: number, room: Room | null): Sizes {
  let most = DEFAULTS[panel] * MOST;
  if (room) {
    const beside = panel === "assets" ? sizes.chat : panel === "chat" ? sizes.assets : 0;
    most = Math.min(most, budget(panel, room) - beside);
  }
  return { ...sizes, [panel]: within(wanted, least(panel), Math.max(least(panel), most)) };
}

/**
 * The sizes as drawn in `room`: each held inside its range, and where the
 * room is too small for what they want — a window made smaller — the panels
 * give the preview back its share, each in proportion to how far it is above
 * its least, and none below it.
 */
export function fit(sizes: Sizes, room: Room | null): Sizes {
  const held = { ...sizes };
  for (const panel of Object.keys(DEFAULTS) as Panel[])
    held[panel] = within(sizes[panel], least(panel), DEFAULTS[panel] * MOST);
  if (!room) return held;
  giveBack(["assets", "chat"], held, budget("assets", room));
  giveBack(["timeline"], held, budget("timeline", room));
  return held;
}

/** Takes from `panels` in `sizes`, in place, until they fit in `budget` or are all at their least. */
function giveBack(panels: Panel[], sizes: Sizes, budget: number) {
  const total = (of: (panel: Panel) => number) => panels.reduce((sum, p) => sum + of(p), 0);
  const over = total((panel) => sizes[panel]) - budget;
  const slack = total((panel) => sizes[panel] - least(panel));
  if (over <= 0 || slack <= 0) return;
  const share = Math.min(1, over / slack);
  for (const panel of panels)
    sizes[panel] =
      share === 1 ? least(panel) : sizes[panel] - (sizes[panel] - least(panel)) * share;
}

/** The `localStorage` key: one for the browser, since a screen is not a project. */
export const SIZES_KEY = "scorsese.panels";

/** The sizes last left in this browser; the defaults for any it does not have. */
export function savedSizes(storage: Pick<Storage, "getItem"> | undefined): Sizes {
  try {
    const saved: unknown = JSON.parse(storage?.getItem(SIZES_KEY) ?? "null");
    const sizes = { ...DEFAULTS };
    if (saved && typeof saved === "object")
      for (const panel of Object.keys(DEFAULTS) as Panel[]) {
        const size = (saved as Record<string, unknown>)[panel];
        if (typeof size === "number" && Number.isFinite(size)) sizes[panel] = size;
      }
    return fit(sizes, null);
  } catch {
    return { ...DEFAULTS };
  }
}

export function saveSizes(storage: Pick<Storage, "setItem"> | undefined, sizes: Sizes) {
  try {
    storage?.setItem(SIZES_KEY, JSON.stringify(sizes));
  } catch {
    // A private window: the sizes last as long as the page does.
  }
}
