// The preview raster is the server's arithmetic, done in the browser: the
// still tool is asked for exactly the size the preview video is rendered at.

import { describe, expect, test } from "bun:test";
import { previewRaster } from "./quality";

describe("the preview raster", () => {
  test("is a fraction of the delivery size, as the desktop app draws it", () => {
    expect(previewRaster("1920x1080", "full")).toBe("1920x1080");
    expect(previewRaster("1920x1080", "half")).toBe("960x540");
    expect(previewRaster("1920x1080", "quarter")).toBe("480x270");
    expect(previewRaster("1080x1920", "quarter")).toBe("270x480");
  });

  test("keeps both sides even, as the encoder needs", () => {
    expect(previewRaster("1080x1080", "quarter")).toBe("270x270");
    expect(previewRaster("1284x722", "quarter")).toBe("320x180");
  });
});
