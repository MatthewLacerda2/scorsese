// `/terms`: the terms of use (#547).

import { LegalPage } from "@/legal/LegalPage";
import { TERMS } from "@/legal/terms";

export function TermsPage() {
  return <LegalPage document={TERMS} />;
}
