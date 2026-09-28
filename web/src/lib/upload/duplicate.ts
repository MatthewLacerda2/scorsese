// "You already have this as X": the one refusal an upload must not retry.
//
// A tus client retries a `409` by default, which is right for a chunk sent at
// the wrong offset — it asks `HEAD` and carries on. But the server also answers
// `409` for a file whose bytes are already in the library, with the item's id
// in the body (`{"error": "you already have this as “X”", "item": 7}`), and
// retrying that only asks the same question again. `shouldRetry` is what the
// Uppy `onShouldRetry` hook calls to tell the two apart (docs/web.md, *Library*).

/** An upload refused because the library already holds its bytes. */
export interface Duplicate {
  /** The library item that has them. */
  item: number;
  /** The server's sentence, naming the item. */
  message: string;
}

/** The duplicate a response describes, or `null` when it is anything else. */
export function duplicateOf(
  status: number | undefined,
  body: string | undefined,
): Duplicate | null {
  if (status !== 409 || !body) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(body);
  } catch {
    return null;
  }
  if (typeof parsed !== "object" || parsed === null) return null;
  const { item, error } = parsed as { item?: unknown; error?: unknown };
  if (typeof item !== "number") return null;
  const message = typeof error === "string" ? error : "you already have this file";
  return { item, message };
}

/** The part of a tus response `shouldRetry` reads. */
export interface TusResponse {
  getStatus(): number;
  getBody(): string;
}

/**
 * Whether tus should retry after `response`: never for a duplicate, and
 * whatever the default (`fallback`) decides for everything else.
 */
export function shouldRetry(response: TusResponse | null, fallback: () => boolean): boolean {
  if (response && duplicateOf(response.getStatus(), response.getBody())) return false;
  return fallback();
}

/** The sentence for a duplicate caught before uploading, by its hash. */
export function alreadyHave(item: { id: number; name: string }): Duplicate {
  return { item: item.id, message: `you already have this as “${item.name}”` };
}
