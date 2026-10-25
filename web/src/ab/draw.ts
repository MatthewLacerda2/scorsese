// Which version of a test a visitor sees (#1553): a hash of the visitor code
// and the test's id, so the same browser sees the same version on every visit
// with nothing stored but the code, and one test's draw says nothing of
// another's. FNV-1a over the text, mixed by MurmurHash3's finaliser (FNV's
// low bits alone are the text's parity, the same for every test), then the
// versions in equal shares.

import type { AbTest } from "./tests";

/** FNV-1a, 32 bits, of `text`'s UTF-16 code units, every bit mixed. */
function hash(text: string): number {
  let value = 0x811c9dc5;
  for (let at = 0; at < text.length; at += 1) {
    value ^= text.charCodeAt(at);
    value = Math.imul(value, 0x01000193);
  }
  value ^= value >>> 16;
  value = Math.imul(value, 0x85ebca6b);
  value ^= value >>> 13;
  value = Math.imul(value, 0xc2b2ae35);
  value ^= value >>> 16;
  return value >>> 0;
}

/** The version of `test` that the visitor `code` sees. */
export function draw(code: string, test: AbTest): string {
  const versions = test.versions;
  return versions[hash(`${code}:${test.id}`) % versions.length] ?? versions[0] ?? "";
}
