// `/privacy`: the privacy policy, in the site's language.

import { useLanguage } from "@/i18n/I18nProvider";
import { PRIVACY } from "@/legal/documents";
import { LegalPage } from "@/legal/LegalPage";

export function PrivacyPage() {
  const { language } = useLanguage();
  return <LegalPage documents={PRIVACY} language={language} />;
}
