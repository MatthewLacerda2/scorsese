// The public pages' footer: the links to pricing, the terms and the privacy
// policy (the header leads only to pricing, #1566), each a thumb's height on a
// phone, and who sells the service, which Decreto 7.962/2013 wants in plain
// view (`seller.ts`). The landing page puts its own line above it.

import type { ReactNode } from "react";
import { Link } from "react-router";
import { useT } from "@/i18n/I18nProvider";
import { SELLER } from "@/legal/seller";

/** A link a thumb can hit on a phone: at least 48 px each way, 8 px from the next row. */
const LINK =
  "hover:underline max-sm:inline-flex max-sm:min-h-12 max-sm:min-w-12 max-sm:items-center";

export function Footer({ children }: { children?: ReactNode }) {
  const t = useT();
  return (
    <footer className="dim mx-auto max-w-6xl border-t border-paper/10 px-4 py-6 text-sm sm:px-6">
      {children}
      <nav className="flex flex-wrap gap-x-5 gap-y-2 text-paper">
        <Link to="/pricing" className={LINK}>
          {t.landing.legal.pricingShort}
        </Link>
        <Link to="/terms" className={LINK}>
          {t.landing.legal.terms}
        </Link>
        <Link to="/privacy" className={LINK}>
          {t.landing.legal.privacy}
        </Link>
        <a href={`mailto:${SELLER.email}`} className={LINK}>
          {SELLER.email}
        </a>
      </nav>
      <p className="mt-4 text-xs">
        {SELLER.name} · CNPJ {SELLER.cnpj} · {SELLER.address}
      </p>
    </footer>
  );
}
