// What the server answers, spelled as `crates/server` serialises it. Times are
// seconds since the Unix epoch; money is integer micro-dollars (`*_micros`) and,
// when the operator has set a display rate, integer centavos (`*_centavos`).
// docs/web.md has each route; the Rust type each mirrors is named beside it.

/** `accounts::Account` — who is logged in. */
export interface Account {
  id: number;
  email: string;
  created_at: number;
}

/** `projects::Summary` — a project in a list, without its document. */
export interface ProjectSummary {
  id: number;
  name: string;
  revision: number;
  created_at: number;
  updated_at: number;
}

/**
 * `projects::Stored` — a project opened: the summary and its `project.json`.
 * The document is the format docs/project-format.md describes; it is typed
 * loosely here because the editor (#545), not this client, reads inside it.
 */
export interface StoredProject extends ProjectSummary {
  document: Record<string, unknown>;
}

/** The kinds a library file can be — `library::Kind`. */
export const FILE_KINDS = ["video", "image", "audio"] as const;
export type FileKind = (typeof FILE_KINDS)[number];

/** `http::library::Tile` — a file in a list: enough to draw a tile. */
export interface LibraryTile {
  id: number;
  name: string;
  kind: FileKind;
  size_bytes: number;
  /** Where its thumbnail is; `404 {"pending": true}` there until it is drawn. */
  thumbnail: string;
}

/** `scorsese_core::MediaMetadata` — what probing the file found. */
export interface MediaMetadata {
  duration_seconds?: number;
  width?: number;
  height?: number;
  has_alpha?: boolean;
  audio_channels?: number;
  sample_rate?: number;
}

/** A project, as the reason a file cannot be deleted — `library::UsedBy`. */
export interface ProjectRef {
  id: number;
  name: string;
}

/**
 * The record of the paid generation that made a file (`library/generation.sql`):
 * a Veo shot or a spoken line, what it was asked, and what it cost.
 */
export interface GenerationRecord {
  kind: "veo_shot" | "spoken_line";
  id: number;
  model: string;
  created_at: number;
  project_id: number | null;
  /** scorsese's own estimate from its price table — never the provider's bill. */
  estimated_cost_micros: number;
  /** What the ledger charged: `0` when the provider failed, which is free. */
  charged_micros: number;
  // A Veo shot's brief.
  prompt?: string;
  resolution?: string;
  seconds?: number;
  aspect?: string;
  // A spoken line's brief.
  voice?: string;
  text?: string;
  settings?: Record<string, unknown>;
}

/** `http::library::Details` — everything known about one file. */
export interface LibraryItem {
  id: number;
  name: string;
  kind: FileKind;
  sha256: string;
  extension: string;
  size_bytes: number;
  media: MediaMetadata;
  description: string | null;
  brief_hash: string | null;
  created_at: number;
  last_used_at: number | null;
  generated: boolean;
  used_by: ProjectRef[];
  generation: GenerationRecord | null;
}

/** `credits::rates::DisplayRate` — reais per dollar, in ten-thousandths. */
export interface DisplayRate {
  brl_per_usd_e4: number;
  set_at: number;
}

/** `http::credits::Balance` — `GET /api/credits`. */
export interface Balance {
  balance_micros: number;
  balance_centavos: number | null;
  rate: DisplayRate | null;
}

/** What a history row is about — `credits::history::KINDS`. */
export const HISTORY_KINDS = [
  "veo_shot",
  "spoken_line",
  "assistant",
  "top_up",
  "monthly_fee",
  "refund",
] as const;
export type HistoryKind = (typeof HISTORY_KINDS)[number];

/** `credits::history::Row` — one thing that moved the balance. */
export interface HistoryRow {
  id: number;
  at: number;
  when: string;
  kind: HistoryKind;
  status: "charged" | "free" | "pending" | "credited";
  project_id: number | null;
  project_name: string | null;
  memo: string;
  amount_micros: number;
  balance_after_micros: number;
  /** What set its price; a generation's carries `library_item_id` and `error`. */
  detail: Record<string, unknown>;
  amount_centavos: number | null;
}

/** `credits::history::History` — a page of history and the filter's total. */
export interface History {
  balance_micros: number;
  balance_centavos: number | null;
  rate: DisplayRate | null;
  matched: number;
  total_micros: number;
  total_centavos: number | null;
  rows: HistoryRow[];
}
