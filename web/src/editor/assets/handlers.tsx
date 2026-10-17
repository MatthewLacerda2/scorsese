// What the tile tests take apart a tile with: an element tree walked for the
// handler a test has to call, with no DOM to click.

import type { ReactElement, ReactNode } from "react";
import type { DocumentAsset } from "@/api";
import { en } from "@/i18n/en";
import { look } from "./look";
import { TileView } from "./TileView";

export const shot: DocumentAsset = {
  id: "beach",
  kind: "video",
  sha256: "aa",
  path: "assets/aa.mp4",
};

/** Every element in a tree, depth first — to find a handler and call it. */
function* elements(node: ReactNode): Generator<ReactElement<Record<string, unknown>>> {
  if (Array.isArray(node)) for (const child of node) yield* elements(child);
  else if (node && typeof node === "object" && "props" in node) {
    const element = node as ReactElement<Record<string, unknown>>;
    yield element;
    yield* elements(element.props.children as ReactNode);
  }
}

/** The handler `key` of the first element `match` picks — failing loudly when none does. */
export function handler<F>(
  view: ReactNode,
  match: (props: Record<string, unknown>) => boolean,
  key: string,
) {
  const found = [...elements(view)].find((element) => match(element.props));
  if (!found) throw new Error(`no element has ${key}`);
  return found.props[key] as F;
}

export function tile(asset: DocumentAsset, onRemove = () => {}, onOpen = () => {}) {
  const props = { asset, stills: 0, open: false, onFold: () => {}, pending: false };
  return TileView({
    ...props,
    onRemove,
    onOpen,
    shown: look(asset),
    src: undefined,
    words: en.assets,
  });
}
