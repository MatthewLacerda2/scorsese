// The pricing page's cards: the free credit, then the three purchase sizes,
// side by side on a wide screen and stacked on a phone. The one with the most
// bonus is the highlighted one. What the buttons do is the page's: the free one
// starts, a buy one is told its amount in reais.

import { CheckIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";
import { FREE, PLANS } from "@/pricing/examples";

const CARD = "card flex flex-col gap-5 p-6 sm:p-7";

function Line({ children }: { children: string }) {
  return (
    <li className="dim flex gap-2 text-sm leading-snug">
      <CheckIcon className="gold mt-0.5 size-4 shrink-0" />
      {children}
    </li>
  );
}

export function PlanCards({
  onStart,
  onBuy,
}: {
  onStart: () => void;
  onBuy: (reais: number) => void;
}) {
  const t = useT().pricing;
  return (
    <div className="mt-12 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
      <div className={CARD}>
        <h2 className="display text-xl font-medium">{t.free.name}</h2>
        <p className="display text-3xl font-semibold">
          {t.free.price(formatDollars(FREE.dailyMicros))}
        </p>
        <ul className="flex flex-1 flex-col gap-2">
          {t.free.lines(formatDollars(FREE.monthlyMicros)).map((line) => (
            <Line key={line}>{line}</Line>
          ))}
        </ul>
        <Button type="button" variant="ghost" className="ghost h-auto py-2.5" onClick={onStart}>
          {t.free.button}
        </Button>
      </div>
      {PLANS.map((plan) => (
        <div
          key={plan.reais}
          className={`${CARD} ${plan.best ? "ring-2 ring-gold" : ""}`}
          data-best={plan.best}
        >
          <div className="flex items-center justify-between gap-2">
            <h2 className="display text-xl font-medium">{t.plan.reais(plan.reais)}</h2>
            {plan.best && (
              <span className="rounded-full bg-gold px-2.5 py-0.5 text-xs font-semibold text-screen">
                {t.plan.best}
              </span>
            )}
          </div>
          <p className="display text-3xl font-semibold">
            {plan.bonusPercent > 0 ? t.plan.bonus(plan.bonusPercent) : " "}
          </p>
          <ul className="flex flex-1 flex-col gap-2">
            {t.plan.lines.map((line) => (
              <Line key={line}>{line}</Line>
            ))}
          </ul>
          <Button
            type="button"
            className={`h-auto py-2.5 ${plan.best ? "cta" : "ghost"}`}
            variant={plan.best ? "default" : "ghost"}
            onClick={() => onBuy(plan.reais)}
          >
            {t.plan.button}
          </Button>
        </div>
      ))}
    </div>
  );
}
