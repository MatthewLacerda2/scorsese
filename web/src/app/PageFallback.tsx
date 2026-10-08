// What a page's place shows while its chunk downloads (#896): nothing at
// first, and the same quiet "Loading…" the pages use once the wait is long
// enough to notice. A fast connection never sees it, so it never flashes.

import { useEffect, useState } from "react";
import { useT } from "@/i18n/I18nProvider";

/** How long a chunk may take before saying so: under it, a blank reads as instant. */
const QUIET_MS = 300;

export function PageFallback() {
  const t = useT();
  const [shown, setShown] = useState(false);
  useEffect(() => {
    const timer = setTimeout(() => setShown(true), QUIET_MS);
    return () => clearTimeout(timer);
  }, []);
  return shown ? <p className="p-6 text-muted-foreground">{t.common.loading}</p> : null;
}
