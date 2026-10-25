// A clip's fill (#1571): a video or an image tiles the library's thumbnail
// along the clip, taken from the cache the assets panel fills; anything else
// stays its kind's solid hue.

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

  test("a caption and a sound stay solid", () => {
    expect(fill({ id: "t", kind: "text", text: "Hi" }).style.backgroundImage).toBe("");
    expect(fill({ id: "a", kind: "audio", path: "a.wav" }).className).toContain("bg-kind-audio");
  });
});
