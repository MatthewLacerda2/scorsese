import { ApiError } from "./client";

/**
 * The words to show for a failed request. A refusal the server worded itself
 * keeps its words unless the caller has better ones for that status in
 * `byStatus`; anything that is not a refusal (a thrown `Error`, or a value
 * thrown bare) shows `unknown` when the caller has one, else its own message.
 */
export function describeError(
  error: unknown,
  byStatus: Partial<Record<number, string>> = {},
  unknown?: string,
): string {
  if (error instanceof ApiError) return byStatus[error.status] ?? error.message;
  if (unknown !== undefined) return unknown;
  return error instanceof Error ? error.message : String(error);
}
