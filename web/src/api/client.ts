// The one door to the server. Every page reaches `/api` through `request`, so
// there is one place that knows the server's error shape (`{"error": "…"}`,
// sometimes with fields beside it — `item`, `projects`, `pending`) and one
// place that turns a failed response into something a page can show.
//
// Same origin in development (Vite's proxy) and in production (nginx), so the
// session cookie — `Path=/api; HttpOnly; SameSite=Strict` — rides along on
// every call without anything here touching it.

/** A request the server refused or failed, with what it said. */
export class ApiError extends Error {
  /** The HTTP status: 401 means logged out, 409 a conflict, and so on. */
  readonly status: number;
  /** The whole JSON body, when there was one: the fields beside `error`. */
  readonly body: Record<string, unknown>;

  constructor(status: number, message: string, body: Record<string, unknown> = {}) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.body = body;
  }
}

/** What `fetch` looks like to this module; tests hand in their own. */
export type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

let fetcher: Fetch = (input, init) => fetch(input, init);

/** Swap the transport — for tests only. Returns the previous one. */
export function setFetch(next: Fetch): Fetch {
  const previous = fetcher;
  fetcher = next;
  return previous;
}

type Method = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

/**
 * Call the API: `path` is under `/api` (`"/library"`), `body` is sent as JSON.
 * Resolves to the parsed JSON body, or `undefined` for an empty one (`204`).
 * Rejects with an {@link ApiError} for any status that is not 2xx.
 */
export async function request<T>(method: Method, path: string, body?: unknown): Promise<T> {
  const init: RequestInit = { method, headers: { Accept: "application/json" } };
  if (body !== undefined) {
    init.headers = { ...init.headers, "Content-Type": "application/json" };
    init.body = JSON.stringify(body);
  }
  let response: Response;
  try {
    response = await fetcher(`/api${path}`, init);
  } catch {
    throw new ApiError(0, "the server could not be reached — check your connection");
  }
  const parsed = await readJson(response);
  if (!response.ok) {
    throw new ApiError(response.status, errorMessage(response.status, parsed), parsed ?? {});
  }
  return parsed as T;
}

/** The body as JSON, or `undefined` when it is empty or not JSON. */
async function readJson(response: Response): Promise<Record<string, unknown> | undefined> {
  const text = await response.text();
  if (!text) return undefined;
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
}

/** The server's own words when it gave some; a plain fallback when not. */
function errorMessage(status: number, body: Record<string, unknown> | undefined): string {
  if (body && typeof body.error === "string") return body.error;
  if (status === 401) return "you are not logged in";
  if (status >= 500) return "something went wrong on the server";
  return `the server answered ${status}`;
}

/**
 * A query string from the fields that are set: `undefined`, `null` and `""`
 * are left out, so a cleared filter is simply absent. Empty gives `""`.
 */
export function query(fields: Record<string, string | number | null | undefined>): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(fields)) {
    if (value === undefined || value === null || value === "") continue;
    params.set(key, String(value));
  }
  const text = params.toString();
  return text ? `?${text}` : "";
}
