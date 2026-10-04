// The inspector shows a generated clip's brief: what will be made, and how far
// along it is. Rendered through `react-dom/server`, so no DOM.

import { describe, expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import type { DocumentAsset, Track } from "@/api";
import { en } from "@/i18n/en";
import { briefOf } from "./brief";
import { Inspector } from "./Inspector";

const clip = { id: "c1", asset: "s", start: 0, duration: 30 };
const track: Track = { id: "v1", kind: "video", clips: [clip] };

function inspect(asset: DocumentAsset): string {
  return renderToString(
    <Inspector
      clip={clip}
      track={track}
      asset={asset}
      fps={{ num: 30, den: 1 }}
      onChange={() => {}}
    />,
  );
}

describe("the inspector", () => {
  test("shows a sketch's prompt, state and choices", () => {
    const html = inspect({
      id: "s",
      kind: "generated_video",
      state: "sketch",
      prompt: "a lighthouse at dusk",
      video: { model: "fast", seconds: 6, resolution: "720p" },
    });
    expect(html).toContain("Brief");
    expect(html).toContain("a lighthouse at dusk");
    expect(html).toContain('data-state="sketch"');
    expect(html).toContain("6s");
    expect(html).toContain("720p");
  });

  test("has no brief for imported footage", () => {
    const html = inspect({ id: "s", kind: "video", path: "assets/a.mp4" });
    expect(html).not.toContain("Brief");
  });
});

describe("briefOf", () => {
  test("calls speech a line and synthesis by its recipe", () => {
    const line = briefOf(
      { id: "n", kind: "generated_audio", prompt: "Hello.", state: "stale" },
      en.inspector.brief,
    );
    expect(line).toMatchObject({ label: "Line", text: "Hello.", state: "stale" });
    const score = briefOf(
      { id: "m", kind: "synth_audio", recipe: "recipes/rain.json" },
      en.inspector.brief,
    );
    expect(score).toMatchObject({ label: "Recipe", text: "rain" });
  });

  test("states only the choices the document makes", () => {
    const still = briefOf(
      { id: "i", kind: "generated_image", image: { aspect: "1:1" } },
      en.inspector.brief,
    );
    expect(still?.choices).toEqual([["Aspect", "1:1"]]);
  });
});
