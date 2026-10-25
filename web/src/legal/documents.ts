// The terms and the privacy policy in the site's language. Portuguese is the
// binding text, and the other two are translations of it (`document.ts`).

import type { Language } from "@/i18n/language";
import type { LegalDocument } from "@/legal/document";
import { PRIVACY_EN } from "@/legal/en/privacy";
import { TERMS_EN } from "@/legal/en/terms";
import { PRIVACY_ES } from "@/legal/es/privacy";
import { TERMS_ES } from "@/legal/es/terms";
import { PRIVACY_PT } from "@/legal/pt-BR/privacy";
import { TERMS_PT } from "@/legal/pt-BR/terms";

export const TERMS: Record<Language, LegalDocument> = {
  "pt-BR": TERMS_PT,
  en: TERMS_EN,
  es: TERMS_ES,
};

export const PRIVACY: Record<Language, LegalDocument> = {
  "pt-BR": PRIVACY_PT,
  en: PRIVACY_EN,
  es: PRIVACY_ES,
};
