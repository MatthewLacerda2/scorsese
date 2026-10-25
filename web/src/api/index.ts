// Every route the web app calls, typed, in one place: docs/web.md's route
// tables as functions. A page imports `api` and never builds a URL itself, so
// when a route changes there is one file to change with it.

import { ApiError, query, request } from "./client";
import type { JobView } from "./events";
import type {
  AbBatch,
  Account,
  Balance,
  ClientError,
  EditorProject,
  FileKind,
  Formats,
  History,
  HistoryKind,
  LibraryItem,
  LibraryTile,
  NewProject,
  Payment,
  PaymentBegun,
  PaymentQuote,
  Prices,
  ProjectSummary,
  RenderAsk,
  RenderView,
  StoredProject,
  ToolAnswer,
} from "./types";

export { ApiError } from "./client";
export { describeError } from "./describe";
export * from "./document";
export * from "./types";

/**
 * The tools the editor calls — the server's allowlist (`http::editor`): the
 * hand-edits (a lane, a placement, a trim, a value, a move to another lane, a
 * delete, and removing a lane or an asset with the clips that go named — #396),
 * bringing a library file in, looking at a frame, and reading and rewriting
 * the script from the Script button (#1445).
 */
export type EditorTool =
  | "track_new"
  | "track_remove"
  | "asset_remove"
  | "place_clip"
  | "clip_set"
  | "clip_move"
  | "clip_remove"
  | "sequence"
  | "import"
  | "still"
  | "script_read"
  | "script_write";

/** What `GET /api/library` narrows by; every field optional. */
export interface LibraryFilter {
  kind?: FileKind;
  search?: string;
  sha256?: string;
  /** Only the files this project uses. */
  project?: number;
}

/** What `GET /api/credits/history` narrows by; days are UTC `YYYY-MM-DD`. */
export interface HistoryFilter {
  project?: number;
  kind?: HistoryKind;
  since?: string;
  until?: string;
  /** The next page: rows older than this row id. */
  before?: number;
  limit?: number;
}

export const api = {
  /**
   * The landing's A/B tests (#1553): what a signed-out visitor was shown and
   * did, sent as a beacon so it survives the page leaving. Always `204`.
   */
  ab: {
    send: (batch: AbBatch) => {
      const body = new Blob([JSON.stringify(batch)], { type: "application/json" });
      navigator.sendBeacon("/api/ab/events", body);
    },
  },
  /**
   * Tell the server the page crashed (#1428): it writes a line to its log and
   * answers `204`, signed in or not.
   */
  crashes: {
    report: (report: ClientError) => request<void>("POST", "/client-errors", report),
  },
  account: {
    logout: () => request<void>("POST", "/logout"),
    me: () => request<Account>("GET", "/me"),
    /**
     * Where the sign-in button sends the browser: the server's start of the
     * Google flow, a run of full-page redirects rather than a call. `next` is
     * the path to come back to, already made safe by the caller.
     */
    googleSignIn: (next: string | null) =>
      next === null ? "/api/auth/google" : `/api/auth/google?next=${encodeURIComponent(next)}`,
  },
  /** The live stream (`src/app/events.ts` holds the one `EventSource` on it). */
  events: {
    url: "/api/events",
  },
  /** Where tus sends a file (`src/lib/upload/uploader.ts`); chunks follow its `Location`. */
  uploads: {
    endpoint: "/api/uploads",
  },
  projects: {
    list: () => request<ProjectSummary[]>("GET", "/projects"),
    /** A project from a name; the assistant reads what it is for from the first prompt (#1531). */
    create: (project: NewProject) => request<StoredProject>("POST", "/projects", project),
    open: (id: number) => request<StoredProject>("GET", `/projects/${id}`),
    rename: (id: number, name: string) =>
      request<{ revision: number }>("PATCH", `/projects/${id}`, { name }),
    remove: (id: number) => request<void>("DELETE", `/projects/${id}`),
  },
  library: {
    list: (filter: LibraryFilter = {}) =>
      request<LibraryTile[]>("GET", `/library${query({ ...filter })}`),
    /** The extensions each kind is uploaded under (#1534); signed out too. */
    formats: () => request<Formats>("GET", "/library/formats"),
    details: (id: number) => request<LibraryItem>("GET", `/library/${id}`),
    update: (id: number, change: { name?: string; description?: string }) =>
      request<LibraryItem>("PATCH", `/library/${id}`, change),
    remove: (id: number) => request<void>("DELETE", `/library/${id}`),
    /** Where the file itself streams from — for `<img>`, `<video>`, `<audio>`. */
    fileUrl: (id: number) => `/api/library/${id}/file`,
    /** The same file sent to be saved under its own name and extension (#711). */
    downloadUrl: (id: number) => `/api/library/${id}/file?download=1`,
    /** A sound's peaks (#1572) at the URL its tile names; rejects with the
     * status, `404` while they are being read. */
    peaks: async (url: string): Promise<ArrayBuffer> => {
      const response = await fetch(url);
      if (!response.ok) throw new ApiError(response.status, `peaks: ${response.status}`);
      return response.arrayBuffer();
    },
    /** The file as text — a page's source, shown and never run (#1300). */
    fileText: async (id: number): Promise<string> => {
      const response = await fetch(`/api/library/${id}/file`);
      if (!response.ok) throw new Error(`library file ${id}: ${response.status}`);
      return response.text();
    },
  },
  editor: {
    /** The project with its document typed for drawing the timeline. */
    open: (id: number) => request<EditorProject>("GET", `/projects/${id}`),
    /**
     * One of the editor's tools on project `id` — every hand-edit is one, so
     * the edit is `core`'s. An edit names the `revision` it was worked out on;
     * a `409` means the project moved on and nothing was written.
     */
    tool: (id: number, name: EditorTool, args: Record<string, unknown>, revision?: number) =>
      request<ToolAnswer>("POST", `/projects/${id}/tools/${name}`, {
        arguments: args,
        revision,
      }),
  },
  jobs: {
    /**
     * Stop a render or a preview (#660): a waiting one comes back `cancelled`,
     * a running one `running`, turning `cancelled` on the event stream once it
     * has stopped. `409` for a kind that is never stopped.
     */
    cancel: (id: number) => request<JobView>("POST", `/jobs/${id}/cancel`),
  },
  renders: {
    /** `200 {render}` when it is already made, `202 {job}` while it is on its way. */
    request: (id: number, settings: RenderAsk = {}) =>
      request<{ render: RenderView | null; job: JobView | null }>(
        "POST",
        `/projects/${id}/renders`,
        settings,
      ),
    list: (id: number) => request<RenderView[]>("GET", `/projects/${id}/renders`),
    /**
     * The editor's preview video (#542): the cut at a preview quality of the
     * delivery size, answered as a render is — made, or on its way.
     */
    preview: (id: number, ask: { resolution: string; quality: string }) =>
      request<{ render: RenderView | null; job: JobView | null }>(
        "POST",
        `/projects/${id}/previews`,
        ask,
      ),
  },
  credits: {
    balance: () => request<Balance>("GET", "/credits"),
    history: (filter: HistoryFilter = {}) =>
      request<History>("GET", `/credits/history${query({ ...filter })}`),
  },
  /** What the public pricing page shows, from the tables that charge. */
  prices: () => request<Prices>("GET", "/prices"),
  payments: {
    /** What `centavos` buy at the rate now. */
    quote: (centavos: number) =>
      request<PaymentQuote>("GET", `/payments/quote${query({ centavos })}`),
    /**
     * A Checkout Session at the rate the person was shown; send the browser to
     * its `url`. A `409` carries the quote at the rate now, when it moved.
     */
    start: (centavos: number, brl_per_usd: string) =>
      request<PaymentBegun>("POST", "/payments", { centavos, brl_per_usd }),
    /** The purchase, credited first if Stripe says it is paid. */
    confirm: (session: string) =>
      request<Payment>("POST", `/payments/${encodeURIComponent(session)}/confirm`),
  },
};
