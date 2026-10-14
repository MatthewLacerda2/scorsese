// `/privacy`: the privacy policy (#547).

import { LegalPage } from "@/legal/LegalPage";
import { PRIVACY } from "@/legal/privacy";

export function PrivacyPage() {
  return <LegalPage document={PRIVACY} />;
}
