// The frame's shape. The project has none of its own — aspect is whatever a
// render's resolution says (docs/project-format.md, *Timeline fps is not
// output fps*) — so the editor keeps the user's choice, per project and per
// browser, and draws previews and offers delivery sizes in it.

export type Shape = "landscape" | "portrait" | "square";

export const SHAPES: Record<Shape, { label: string; preview: string; deliver: string[] }> = {
  landscape: { label: "16:9", preview: "640x360", deliver: ["1920x1080", "1280x720"] },
  portrait: { label: "9:16", preview: "360x640", deliver: ["1080x1920", "720x1280"] },
  square: { label: "1:1", preview: "480x480", deliver: ["1080x1080", "720x720"] },
};

const key = (project: number) => `scorsese.shape.${project}`;

/** The shape last chosen for `project` in this browser; landscape otherwise. */
export function savedShape(project: number): Shape {
  try {
    const saved = localStorage.getItem(key(project));
    return saved && saved in SHAPES ? (saved as Shape) : "landscape";
  } catch {
    return "landscape";
  }
}

export function saveShape(project: number, shape: Shape) {
  try {
    localStorage.setItem(key(project), shape);
  } catch {
    // A private window: the choice lasts as long as the page does.
  }
}
