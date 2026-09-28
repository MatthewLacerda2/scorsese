// Preview quality (#542): how much of the delivery raster the preview draws —
// the same three steps, with the same arithmetic, as the desktop app and the
// server (`scorsese_render::Quality`). Full is the render's own picture;
// half and quarter draw fewer pixels and read proxies of heavy videos, so the
// cut plays sooner after an edit.
//
// Kept per browser, not per project and not in project.json: it is about this
// device and this person's patience, not about the edit.

export type Quality = "full" | "half" | "quarter";

export const QUALITIES: Record<Quality, { label: string; divisor: number; says: string }> = {
  full: { label: "Full", divisor: 1, says: "full quality: the render's own picture" },
  half: { label: "1/2", divisor: 2, says: "half quality: fewer pixels, proxies where made" },
  quarter: { label: "1/4", divisor: 4, says: "quarter quality: fastest, proxies where made" },
};

export const DEFAULT_QUALITY: Quality = "half";

/**
 * The raster a preview of a film delivered at `deliver` (`WIDTHxHEIGHT`) is
 * drawn at: each side divided and kept even, as `Quality::raster` does.
 */
export function previewRaster(deliver: string, quality: Quality): string {
  const { divisor } = QUALITIES[quality];
  const side = (pixels: number) => Math.max(2, Math.floor(pixels / divisor) & ~1);
  const [width = 0, height = 0] = deliver.split("x").map(Number);
  return `${side(width)}x${side(height)}`;
}

const KEY = "scorsese.preview-quality";

/** The quality last chosen in this browser; half otherwise. */
export function savedQuality(): Quality {
  try {
    const saved = localStorage.getItem(KEY);
    return saved && saved in QUALITIES ? (saved as Quality) : DEFAULT_QUALITY;
  } catch {
    return DEFAULT_QUALITY;
  }
}

export function saveQuality(quality: Quality) {
  try {
    localStorage.setItem(KEY, quality);
  } catch {
    // A private window: the choice lasts as long as the page does.
  }
}
