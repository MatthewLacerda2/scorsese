// The pricing page's cards: the free credit, then the three purchase sizes,
// side by side on a wide screen and stacked on a phone. The one with the most
// bonus is the highlighted one. Every card has the same rows — a name, the
// price (the card's heading, so a screen reader's list of them tells the
// cards apart), a bonus line (kept at its height when there is no bonus), what it
// includes, a button — so the rows line up across the cards. What the buttons
// do is the page's: the free one starts, a buy one is told its amount in reais.

import { CheckIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";
import { FREE, PLANS } from "@/pricing/examples";

function Card(props: {
  name: string;
  badge?: string;
  price: string;
  bonus?: string;
  lines: readonly string[];
  best?: boolean;
  children: ReactNode;
}) {
  return (
    <div
      className={`card relative flex flex-col p-6 sm:p-7 ${props.best ? "ring-2 ring-gold" : ""}`}
      data-best={props.best}
    >
      {/* On the card's top edge, so a long badge in any language never pushes the rows down. */}
      {props.badge && (
        <span className="absolute -top-3 left-1/2 -translate-x-1/2 rounded-full bg-gold px-3 py-0.5 text-xs font-semibold whitespace-nowrap text-screen">
          {props.badge}
        </span>
      )}
      <p className="dim font-medium">{props.name}</p>
      <h2 className="display mt-4 text-3xl font-semibold">{props.price}</h2>
      <p className="gold mt-1 min-h-5 text-sm font-medium">{props.bonus}</p>
      <ul className="mt-6 flex flex-1 flex-col gap-2">
        {props.lines.map((line) => (
          <li key={line} className="dim flex gap-2 text-sm leading-snug">
            <CheckIcon className="gold mt-0.5 size-4 shrink-0" aria-hidden />
            {line}
          </li>
        ))}
      </ul>
      <div className="mt-6">{props.children}</div>
    </div>
  );
}

const BUTTON = "h-11 w-full";

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
      <Card
        name={t.free.name}
        price={t.free.price(formatDollars(FREE.dailyMicros))}
        lines={t.free.lines(formatDollars(FREE.monthlyMicros))}
      >
        <Button type="button" variant="ghost" className={`ghost ${BUTTON}`} onClick={onStart}>
          {t.free.button}
        </Button>
      </Card>
      {PLANS.map((plan) => (
        <Card
          key={plan.reais}
          name={t.plan.name}
          badge={plan.best ? t.plan.best : undefined}
          price={t.plan.reais(plan.reais)}
          bonus={plan.bonusPercent > 0 ? t.plan.bonus(plan.bonusPercent) : undefined}
          lines={t.plan.lines}
          best={plan.best}
        >
          <Button
            type="button"
            className={`${BUTTON} ${plan.best ? "cta" : "ghost"}`}
            variant={plan.best ? "default" : "ghost"}
            onClick={() => onBuy(plan.reais)}
          >
            {t.plan.button}
          </Button>
        </Card>
      ))}
    </div>
  );
}
