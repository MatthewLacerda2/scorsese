// Whether changing the project's model deserves a warning first (#705).
//
// A change of model reads the whole conversation uncached on its next turn —
// the prompt cache belongs to the model that wrote it — so the panel asks
// before switching while that cache may still be warm. It is not warm when the
// conversation is empty, or when the last answer is older than the cache of
// the model being left lasts (`cache_seconds`, an hour for every model today).
// The time is the server's `finished_at`, compared with the browser's clock:
// it survives a reload, and a few seconds of drift do not matter against an
// hour. The warning's words are the catalogue's (`chat.model.switchWarning`).

import type { ModelChoice, TurnView } from "@/api/chat";

/** Whether leaving `leaving` now could cost a cache miss, given the conversation's `turns`. */
export function warnsBeforeSwitching(
  turns: TurnView[],
  leaving: ModelChoice | undefined,
  nowSeconds: number,
): boolean {
  const last = turns.at(-1);
  if (last === undefined) return false;
  // Still running: whatever it has written is as warm as a cache gets.
  if (last.finished_at === null) return true;
  const lifetime = leaving?.cache_seconds ?? 3600;
  return nowSeconds - last.finished_at <= lifetime;
}
