// Every route the web app calls, typed, in one place: docs/web.md's route
// tables as functions. A page imports `api` and never builds a URL itself, so
// when a route changes there is one file to change with it.

import { query, request } from "./client";
import type { JobView } from "./events";
import type {
  Account,
  Balance,
  EditorProject,
  FileKind,
  History,
  HistoryKind,
  LibraryItem,
  LibraryTile,
  ProjectSummary,
  RenderView,
  StoredProject,
  TemplateSummary,
  ToolAnswer,
} from "./types";

export { ApiError } from "./client";
export * from "./document";
export * from "./types";

/**
 * The tools the editor calls — the server's allowlist (`http::editor`): the
 * hand-edits (a lane, a placement, a trim, a value, a move to another lane, a
 * delete), bringing a library file in, looking at a frame, and saving the
 * selection as a template or putting one in (#546).
 */
export type EditorTool =
  | "track_new"
  | "place_clip"
  | "trim_clip"
  | "clip_set"
  | "clip_move"
  | "clip_remove"
  | "import"
  | "still"
  | "template_save"
  | "template_insert";

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
  account: {
    login: (email: string, password: string) =>
      request<Account>("POST", "/login", { email, password }),
    logout: () => request<void>("POST", "/logout"),
    me: () => request<Account>("GET", "/me"),
    changePassword: (current: string, next: string) =>
      request<void>("POST", "/me/password", { current, new: next }),
  },
  projects: {
    list: () => request<ProjectSummary[]>("GET", "/projects"),
    create: (name: string) => request<StoredProject>("POST", "/projects", { name }),
    open: (id: number) => request<StoredProject>("GET", `/projects/${id}`),
    rename: (id: number, name: string) =>
      request<{ revision: number }>("PATCH", `/projects/${id}`, { name }),
    remove: (id: number) => request<void>("DELETE", `/projects/${id}`),
  },
  library: {
    list: (filter: LibraryFilter = {}) =>
      request<LibraryTile[]>("GET", `/library${query({ ...filter })}`),
    details: (id: number) => request<LibraryItem>("GET", `/library/${id}`),
    update: (id: number, change: { name?: string; description?: string }) =>
      request<LibraryItem>("PATCH", `/library/${id}`, change),
    remove: (id: number) => request<void>("DELETE", `/library/${id}`),
    /** Where the file itself streams from — for `<img>`, `<video>`, `<audio>`. */
    fileUrl: (id: number) => `/api/library/${id}/file`,
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
    request: (id: number, settings: { resolution?: string } = {}) =>
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
  templates: {
    /** The user's templates, by name — saved and inserted through the tools. */
    list: () => request<TemplateSummary[]>("GET", "/templates"),
    /** The videos it went into keep their copies. */
    remove: (id: number) => request<void>("DELETE", `/templates/${id}`),
  },
  credits: {
    balance: () => request<Balance>("GET", "/credits"),
    history: (filter: HistoryFilter = {}) =>
      request<History>("GET", `/credits/history${query({ ...filter })}`),
  },
};
