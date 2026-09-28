// The frame's shape. The project has none of its own — aspect is whatever a
// render's resolution says (docs/project-format.md, *Timeline fps is not
// output fps*) — so the editor keeps the user's choice, per project and per
// browser, and offers delivery sizes in it. The first is what the preview is
// a fraction of (`preview/quality.ts`).

export type Shape = "landscape" | "portrait" | "square";

export const SHAPES: Record<Shape, { label: string; deliver: [string, ...string[]] }> = {
  landscape: { label: "16:9", deliver: ["1920x1080", "1280x720"] },
  portrait: { label: "9:16", deliver: ["1080x1920", "720x1280"] },
  square: { label: "1:1", deliver: ["1080x1080", "720x720"] },
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
