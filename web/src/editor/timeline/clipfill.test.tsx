// A clip's fill (#1571): a video or an image tiles the library's thumbnail
// along the clip, taken from the cache the assets panel fills; a sound draws
// the slice of its peaks it plays (#1572); anything else stays its kind's
// solid hue.

import { describe, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import type { DocumentAsset } from "@/api";
import { createQueryClient } from "@/app/queryClient";
import { ClipFill } from "./ClipFill";

function fill(asset: DocumentAsset, cached = true) {
  const client = createQueryClient();
  if (cached) {
    client.setQueryData(
      ["library", "list", { sha256: "aa" }],
      [{ id: 7, name: "f", kind: "image", size_bytes: 1, thumbnail: "/t/7.jpg" }],
    );
  }
  return render(
    <QueryClientProvider client={client}>
      <ClipFill asset={asset} />
    </QueryClientProvider>,
  ).container.firstElementChild as HTMLElement;
}

describe("a clip's fill", () => {
  test("a video tiles its thumbnail", () => {
    const el = fill({ id: "v", kind: "video", sha256: "aa", path: "v.mp4" });
    expect(el.style.backgroundImage).toContain("/t/7.jpg");
    expect(el.style.backgroundRepeat).toBe("repeat-x");
  });

  test("an image tiles its thumbnail", () => {
    const el = fill({ id: "i", kind: "image", sha256: "aa", path: "i.png" });
    expect(el.style.backgroundImage).toContain("/t/7.jpg");
  });

  test("a picture still unknown stays the kind's hue", () => {
    const el = fill({ id: "v", kind: "video", sha256: "aa", path: "v.mp4" }, false);
    expect(el.style.backgroundImage).toBe("");
    expect(el.className).toContain("bg-kind-video");
  });

  test("a caption stays solid", () => {
    expect(fill({ id: "t", kind: "text", text: "Hi" }).style.backgroundImage).toBe("");
    expect(fill({ id: "t", kind: "text", text: "Hi" }).className).toContain("bg-kind-text");
  });
});

describe("a sound's fill (#1572)", () => {
  function sounding(span: { from: number; length: number; width: number }) {
    const client = createQueryClient();
    client.setQueryData(
      ["library", "list", { sha256: "bb" }],
      [{ id: 9, name: "s", kind: "audio", size_bytes: 1, thumbnail: "/t/9", peaks: "/p/9" }],
    );
    client.setQueryData(["library", "peaks", "/p/9"], {
      perSecond: 4,
      levels: Uint8Array.from([0, 51, 255, 102, 0, 0, 204, 0]),
    });
    const { container } = render(
      <QueryClientProvider client={client}>
        <ClipFill asset={{ id: "a", kind: "audio", sha256: "bb", path: "a.wav" }} span={span} />
      </QueryClientProvider>,
    );
    return container;
  }

  test("draws a bar every three pixels of the slice the clip plays", () => {
    const bars = sounding({ from: 0, length: 2, width: 6 }).querySelector("[data-bars]");
    expect(bars?.getAttribute("data-bars")).toBe("1 0.8");
  });

  test("a long clip close up is still one path, not a node a bar", () => {
    const paths = sounding({ from: 0, length: 2, width: 192_000 }).querySelectorAll("path");
    expect(paths.length).toBe(1);
    expect(paths[0]?.getAttribute("data-bars")).toBe("0 0.2 1 0.4 0 0 0.8 0");
  });

  test("trimmed, it draws the slice from its in point", () => {
    const bars = sounding({ from: 0.25, length: 0.5, width: 6 }).querySelector("[data-bars]");
    expect(bars?.getAttribute("data-bars")).toBe("0.2 1");
  });

  test("peaks not known yet leave the kind's hue", () => {
    const client = createQueryClient();
    const { container } = render(
      <QueryClientProvider client={client}>
        <ClipFill
          asset={{ id: "a", kind: "synth_audio", sha256: "cc", path: "a.wav" }}
          span={{ from: 0, length: 1, width: 30 }}
        />
      </QueryClientProvider>,
    );
    expect(container.querySelector("[data-bars]")).toBeNull();
    expect((container.firstElementChild as HTMLElement).className).toContain("bg-kind-synth-audio");
  });
});
