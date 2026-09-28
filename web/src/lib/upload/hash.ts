// A file's SHA-256, computed in the browser before a byte is uploaded: the
// library is stored once per (user, hash), so the hash is what lets the page
// ask `GET /api/library?sha256=` and say "you already have this" without
// sending the file at all (docs/web.md, *Library*).
//
// Streamed a chunk at a time rather than read whole — `crypto.subtle.digest`
// wants the entire file in memory, and a screen recording is gigabytes.

import { sha256 } from "@noble/hashes/sha2.js";
import { bytesToHex } from "@noble/hashes/utils.js";

/**
 * The lowercase hex SHA-256 of `file`. `onProgress` hears the fraction hashed
 * so far, from 0 to 1, so a large file shows it is being read.
 */
export async function hashFile(
  file: Blob,
  onProgress?: (fraction: number) => void,
): Promise<string> {
  const hash = sha256.create();
  const reader = file.stream().getReader();
  let done = 0;
  for (;;) {
    const chunk = await reader.read();
    if (chunk.done) break;
    hash.update(chunk.value);
    done += chunk.value.byteLength;
    onProgress?.(file.size === 0 ? 1 : done / file.size);
  }
  return bytesToHex(hash.digest());
}
