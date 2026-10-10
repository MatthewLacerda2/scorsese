// What a project is started for (#1016): a placement and a kind of video,
// both optional. The menu is the server's (`GET /api/styles`, the same closed
// lists the CLI and MCP offer); these are the plain decisions the dialog and
// the editor's control draw from it.

import type { StyleMenu, StyleView } from "@/api";
import type { Shape } from "@/editor/shape";

/** The styles offered for `platform`, in the menu's order — all of them with none chosen. */
export function stylesFor(menu: StyleMenu, platform: string | null): StyleView[] {
  if (platform === null) return menu.styles;
  return menu.styles.filter((style) => style.platforms.includes(platform));
}

/**
 * The style still chosen after the platform becomes `platform`: kept when it
 * is made for it, dropped otherwise — the dialog never holds a pair the
 * server would refuse.
 */
export function keptStyle(menu: StyleMenu, platform: string | null, style: string | null) {
  if (style === null) return null;
  return stylesFor(menu, platform).some((offered) => offered.id === style) ? style : null;
}

/** The editor's frame shape for a placement: upright, or landscape for YouTube. */
export function shapeOf(menu: StyleMenu, platform: string | null): Shape | null {
  const found = menu.platforms.find((offered) => offered.id === platform);
  if (!found) return null;
  if (found.width === found.height) return "square";
  return found.width > found.height ? "landscape" : "portrait";
}

/** A choice's name as the menu gives it, or `null` for none. */
export function nameOf(menu: StyleMenu, kind: "platforms" | "styles", id: string | null) {
  const list: { id: string; name: string }[] = menu[kind];
  return list.find((item) => item.id === id)?.name ?? null;
}
