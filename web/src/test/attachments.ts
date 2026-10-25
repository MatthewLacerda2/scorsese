// A tray with nothing in it, for a test that mounts what takes one but is not about attaching.

import type { Attachments } from "@/prompt/attach/useAttachments";

export const noAttachments: Attachments = {
  list: [],
  add: () => {},
  remove: () => {},
  clear: () => {},
  refused: [],
  busy: false,
  items: [],
  accept: undefined,
  prepare: () => {},
};
