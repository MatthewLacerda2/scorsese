// What the new-project dialog holds until Create (#1016), and the plain
// decisions about it: the request it makes, a file picked or unpicked, a
// platform chosen. `Wizard.tsx` is the dialog's steps.

import type { NewProject, StyleMenu } from "@/api";
import { keptStyle } from "@/start/menu";

export const STEPS = ["name", "assets", "platform", "style"] as const;

/** What the dialog holds until Create. */
export interface Draft {
  name: string;
  assets: number[];
  platform: string | null;
  style: string | null;
}

export const EMPTY: Draft = { name: "", assets: [], platform: null, style: null };

/** The request a draft makes — or `null` when it has no name yet, and creates nothing. */
export function bodyOf(draft: Draft): NewProject | null {
  const name = draft.name.trim();
  if (!name) return null;
  return { name, platform: draft.platform, style: draft.style, assets: draft.assets };
}

/** The draft with `id` picked, or unpicked when it already was. */
export function toggled(draft: Draft, id: number): Draft {
  const assets = draft.assets.includes(id)
    ? draft.assets.filter((picked) => picked !== id)
    : [...draft.assets, id];
  return { ...draft, assets };
}

/** The draft on `platform`, keeping its style only when it is made for that. */
export function onPlatform(draft: Draft, menu: StyleMenu, platform: string | null): Draft {
  return { ...draft, platform, style: keptStyle(menu, platform, draft.style) };
}
