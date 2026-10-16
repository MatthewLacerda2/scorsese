// How hard the assistant thinks on a message (#769), in the user's words:
// Quick, Balanced or Thorough — `low`, `medium` and `high` to the server,
// never "effort" on screen. Chosen beside Send, sent with each message, and
// remembered per project in this browser: a project of nudges stays on Quick,
// one being built stays on Thorough.
//
// Stored through `lib/storage.ts`, which catches a storage that throws — a
// private window, storage disabled — so a failure reads as "not chosen", which
// is Thorough, today's behaviour.

import { KEYS, oneOf, read, write } from "@/lib/storage";

/** The levels, cheapest first, as the server names them. */
export const EFFORTS = ["low", "medium", "high"] as const;

export type Effort = (typeof EFFORTS)[number];

/** What a message is sent at until the user picks otherwise. */
export const DEFAULT_EFFORT: Effort = "high";

const CODEC = oneOf<Effort>(EFFORTS, DEFAULT_EFFORT);

/** `project`'s remembered level, or the default. */
export function storedEffort(
  storage: Pick<Storage, "getItem"> | undefined,
  project: number,
): Effort {
  return read(storage, KEYS.effort(project), CODEC);
}

/** Remember `project`'s level. A storage that refuses only means it is not remembered. */
export function saveEffort(
  storage: Pick<Storage, "setItem"> | undefined,
  project: number,
  effort: Effort,
): void {
  write(storage, KEYS.effort(project), CODEC, effort);
}
