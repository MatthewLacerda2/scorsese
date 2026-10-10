// The catalogue in Brazilian Portuguese — the maintainer's language, written as a Brazilian would say it, not word for word.
// Typed against English (`Messages`), so a missing key does not compile.

import type { Messages } from "@/i18n/catalogue";
import { assets } from "@/i18n/pt-BR/assets";
import { chat } from "@/i18n/pt-BR/chat";
import { common } from "@/i18n/pt-BR/common";
import { editor } from "@/i18n/pt-BR/editor";
import { files } from "@/i18n/pt-BR/files";
import { inspector } from "@/i18n/pt-BR/inspector";
import { landing } from "@/i18n/pt-BR/landing";
import { pages } from "@/i18n/pt-BR/pages";

export const ptBR: Messages = { common, files, pages, editor, chat, assets, inspector, landing };
