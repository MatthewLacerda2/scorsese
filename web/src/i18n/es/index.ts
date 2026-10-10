// The catalogue in Spanish, written to read naturally rather than word for word.
// Typed against English (`Messages`), so a missing key does not compile.

import type { Messages } from "@/i18n/catalogue";
import { assets } from "@/i18n/es/assets";
import { chat } from "@/i18n/es/chat";
import { common } from "@/i18n/es/common";
import { editor } from "@/i18n/es/editor";
import { files } from "@/i18n/es/files";
import { inspector } from "@/i18n/es/inspector";
import { landing } from "@/i18n/es/landing";
import { menu } from "@/i18n/es/menu";
import { pages } from "@/i18n/es/pages";
import { start } from "@/i18n/es/start";

export const es: Messages = {
  common,
  files,
  pages,
  editor,
  chat,
  assets,
  inspector,
  landing,
  start,
  menu,
};
