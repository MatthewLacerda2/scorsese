// The three texts of each document have the same shape: the same sections, in
// the same order, with the same blocks (a paragraph or a list of the same
// length) and the same links, so a translation cannot gain or lose a clause
// unseen.

import { expect, test } from "bun:test";
import type { Block, LegalDocument } from "@/legal/document";
import { PRIVACY, TERMS } from "@/legal/documents";

/** `paragraph`, or `list:N` with the links' addresses, per block. */
function blockShape(block: Block): string {
  if (typeof block === "string") return "paragraph";
  const links = block.map((item) => (typeof item === "string" ? "-" : item.link.href));
  return `list:${block.length}:${links.join(",")}`;
}

const shape = (doc: LegalDocument) => ({
  summary: doc.summary.points.length,
  updated: doc.updated,
  sections: doc.sections.map((section) => section.blocks.map(blockShape)),
});

for (const [name, documents] of [
  ["terms", TERMS],
  ["privacy policy", PRIVACY],
] as const) {
  test(`the ${name} has the same structure in Portuguese, English and Spanish`, () => {
    expect(shape(documents.en)).toEqual(shape(documents["pt-BR"]));
    expect(shape(documents.es)).toEqual(shape(documents["pt-BR"]));
  });

  test(`the ${name} opens with a summary, and every vendor link is https`, () => {
    for (const doc of Object.values(documents)) {
      expect(doc.summary.points.length).toBeGreaterThanOrEqual(5);
      for (const section of doc.sections)
        for (const block of section.blocks)
          if (typeof block !== "string")
            for (const item of block)
              if (typeof item !== "string") expect(item.link.href).toStartWith("https://");
    }
  });
}
