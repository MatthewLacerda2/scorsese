// The line under the first prompt box that answers a visitor's first doubt:
// starting costs nothing (#1566). The day's free allowance is read from the
// server, the pricing page's own source (`GET /api/prices`); while that is
// asked, or if it fails, the line stands without a figure rather than a wrong one.

import { useT } from "@/i18n/I18nProvider";
import { dollarAmount } from "@/lib/money";
import { usePrices } from "@/pricing/usePrices";

export function FreeLine() {
  const t = useT().landing.hero;
  const daily = usePrices().data?.free.daily_micros;
  return <p className="dim mt-3 text-sm">{t.free(daily ? dollarAmount(daily) : undefined)}</p>;
}
