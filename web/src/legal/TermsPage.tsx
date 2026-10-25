// `/terms`: the terms of use, in the site's language.

import { useLanguage } from "@/i18n/I18nProvider";
import { TERMS } from "@/legal/documents";
import { LegalPage } from "@/legal/LegalPage";

export function TermsPage() {
  const { language } = useLanguage();
  return <LegalPage documents={TERMS} language={language} />;
}
