// Where a file came from (#1237): a stock file's card names its library,
// author, licence and page; a generated file's record names its provider.

import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import type { GenerationRecord as Record, StockSource } from "@/api";
import { GenerationRecord } from "./GenerationRecord";
import { StockSourceCard } from "./StockSourceCard";

const source: StockSource = {
  library: "pixabay",
  id: "39009",
  author: "someone",
  page_url: "https://pixabay.com/videos/id-39009/",
  licence: "Pixabay Content License",
};

test("a stock file's card says its library, author, licence and page", () => {
  const html = renderToStaticMarkup(<StockSourceCard source={source} />);
  expect(html).toContain("From Pixabay");
  expect(html).toContain("someone");
  expect(html).toContain("Pixabay Content License");
  expect(html).toContain('href="https://pixabay.com/videos/id-39009/"');
  expect(html).toContain('rel="noopener noreferrer"');
  expect(html).toContain("See it on Pixabay");
});

test("a card leaves out what the library did not say", () => {
  const html = renderToStaticMarkup(<StockSourceCard source={{ library: "pixabay", id: "7" }} />);
  expect(html).toContain("From Pixabay");
  expect(html).not.toContain("<a");
  expect(html).not.toContain("Licence");
});

const shot: Record = {
  kind: "veo_shot",
  id: 1,
  model: "veo-3.1-fast",
  created_at: 1_760_000_000,
  project_id: null,
  estimated_cost_micros: 1_200_000,
  charged_micros: 1_320_000,
  prompt: "rain on a window",
};

test("a generated file's record names its provider from its kind", () => {
  const named = (record: Record) => renderToStaticMarkup(<GenerationRecord record={record} />);
  expect(named(shot)).toContain("Veo, by Google");
  expect(named({ ...shot, kind: "still_image" })).toContain("Gemini, by Google");
  expect(named({ ...shot, kind: "spoken_line", text: "hello" })).toContain("ElevenLabs");
  expect(named({ ...shot, kind: "voice_design" })).toContain("ElevenLabs");
});
