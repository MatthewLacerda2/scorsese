// Where an attached file is: held in the page (signed out, nothing is
// uploaded), on its way into the library, ready as a library item, or
// refused. Signed in, it is the upload row's own state (`files/uploads.tsx`),
// so the tray and the uploads corner never disagree.

import type { Upload } from "@/files/uploads";

export interface Status {
  phase: "held" | "uploading" | "ready" | "failed";
  /** 0–1 of the whole way, hashing included. */
  progress: number;
  /** The library item it is, once ready. */
  item?: number;
  /** Why it was refused. */
  message?: string;
}

/**
 * The state of the file whose upload row is `key` — `undefined` when it is only
 * held. `settled` is what it came to before, kept because the uploads corner's
 * "Clear finished" forgets the row.
 */
export function statusOf(
  key: string | undefined,
  rows: readonly Upload[],
  settled?: Status,
): Status {
  if (settled) return settled;
  if (key === undefined) return { phase: "held", progress: 1 };
  const row = rows.find((found) => found.key === key);
  if (!row) return { phase: "uploading", progress: 0 };
  switch (row.phase) {
    case "hashing":
      return { phase: "uploading", progress: row.progress / 2 };
    case "uploading":
      return { phase: "uploading", progress: 0.5 + row.progress / 2 };
    case "failed":
      return { phase: "failed", progress: 0, message: row.message };
    default:
      // Uploaded, or already in the library: either way, an item.
      return row.item === undefined
        ? { phase: "failed", progress: 0, message: row.message }
        : { phase: "ready", progress: 1, item: row.item };
  }
}

/** Whether a status is final: ready or refused, never to change again. */
export function isSettled(status: Status): boolean {
  return status.phase === "ready" || status.phase === "failed";
}

/** Whether sending must wait: a file is still on its way. */
export function busy(statuses: readonly Status[]): boolean {
  return statuses.some((status) => status.phase === "uploading");
}

/** The library items that go with the message. */
export function items(statuses: readonly Status[]): number[] {
  return statuses.flatMap((status) => (status.item === undefined ? [] : [status.item]));
}
