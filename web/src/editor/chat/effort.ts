// How hard the assistant thinks (#769, #1443): low, medium, high or extra —
// `xhigh` to the server, which is the API's name for it. Kept on the project
// on the server, beside its model, and chosen beside the model picker; the
// server's answer to `GET /projects/{id}/chat` says which it is.

/** The levels, cheapest first, as the server names them. */
export const EFFORTS = ["low", "medium", "high", "xhigh"] as const;

export type Effort = (typeof EFFORTS)[number];

/** `name` when the server sent one of the levels, else `high`, a new project's. */
export function effortOf(name: string | undefined): Effort {
  return EFFORTS.find((level) => level === name) ?? "high";
}
