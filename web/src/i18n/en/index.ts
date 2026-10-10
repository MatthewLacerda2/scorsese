// The English catalogue: the source every other language is typed against,
// and the fallback for anything one lacks. One file per area of the app, so
// no catalogue outgrows the size gate.

import { assets } from "@/i18n/en/assets";
import { chat } from "@/i18n/en/chat";
import { common } from "@/i18n/en/common";
import { editor } from "@/i18n/en/editor";
import { files } from "@/i18n/en/files";
import { inspector } from "@/i18n/en/inspector";
import { landing } from "@/i18n/en/landing";
import { menu } from "@/i18n/en/menu";
import { pages } from "@/i18n/en/pages";
import { start } from "@/i18n/en/start";

export const en = { common, files, pages, editor, chat, assets, inspector, landing, start, menu };
