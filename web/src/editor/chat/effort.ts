// How hard the assistant thinks on a message (#769), in the user's words:
// Quick, Balanced or Thorough — `low`, `medium` and `high` to the server,
// never "effort" on screen. Chosen beside Send, sent with each message, and
// remembered per project in this browser: a project of nudges stays on Quick,
// one being built stays on Thorough.
//
// Wrapped as `i18n/language.ts` wraps the language: `localStorage` can throw —
// a private window, storage disabled — so every access is caught, and a
// failure reads as "not chosen", which is Thorough, today's behaviour.

/** The levels, cheapest first, as the server names them. */
export const EFFORTS = ["low", "medium", "high"] as const;

export type Effort = (typeof EFFORTS)[number];

/** What a message is sent at until the user picks otherwise. */
export const DEFAULT_EFFORT: Effort = "high";

/** The `localStorage` key for `project`'s choice. */
export function effortKey(project: number): string {
  return `scorsese-effort-${project}`;
}

function isEffort(value: unknown): value is Effort {
  return EFFORTS.some((effort) => effort === value);
}

/** `project`'s remembered level, or the default. */
export function storedEffort(
  storage: Pick<Storage, "getItem"> | undefined,
  project: number,
): Effort {
  try {
    const value = storage?.getItem(effortKey(project));
    return isEffort(value) ? value : DEFAULT_EFFORT;
  } catch {
    return DEFAULT_EFFORT;
  }
}

/** Remember `project`'s level. A storage that refuses only means it is not remembered. */
export function saveEffort(
  storage: Pick<Storage, "setItem"> | undefined,
  project: number,
  effort: Effort,
): void {
  try {
    storage?.setItem(effortKey(project), effort);
  } catch {
    // Private window or storage disabled: the choice lasts this page only.
  }
}
