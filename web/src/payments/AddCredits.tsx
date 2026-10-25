// "Add credits" (#548): an amount in reais, what it buys shown live as it is
// typed, and Pay — which sends the browser to Stripe's own page. The rate is
// the server's (docs/web.md, *Buying credits*); Pay names the one shown, and
// when it has moved since, the server answers with the quote now, which takes
// the old one's place here so the person sees it before paying again.
//
// Its own chunk: the shell loads it on the first open, so the
// entry chunk carries none of it.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useState } from "react";
import { ApiError, api, type PaymentQuote } from "@/api";
import { queryKeys } from "@/app/queryKeys";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";
import { useSettled } from "@/lib/useSettled";
import {
  formatReais,
  MAXIMUM_CENTAVOS,
  MINIMUM_CENTAVOS,
  parseReais,
  wholeCents,
  withinLimits,
} from "@/payments/reais";

/** How long typing must pause before the quote is asked for. */
const SETTLE_MS = 300;

export function AddCreditsDialog({
  open,
  onOpenChange,
  initial,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The amount in reais the box starts with; 50 when left out. */
  initial?: string;
}) {
  const t = useT();
  const [text, setText] = useState(initial ?? "50");
  const centavos = parseReais(text);
  const asked = useSettled(centavos, SETTLE_MS);
  const valid = asked !== null && withinLimits(asked);
  const quote = useQuery({
    queryKey: queryKeys.paymentQuote(asked),
    queryFn: () => api.payments.quote(asked ?? 0),
    enabled: open && valid,
    retry: false,
  });
  const client = useQueryClient();
  const pay = useMutation({
    mutationFn: (shown: PaymentQuote) => api.payments.start(shown.brl_centavos, shown.brl_per_usd),
    onSuccess: (begun) => window.location.assign(begun.url),
    onError: (failure, shown) => {
      const moved =
        failure instanceof ApiError && failure.status === 409 ? failure.body.quote : null;
      if (moved) client.setQueryData(queryKeys.paymentQuote(shown.brl_centavos), moved);
    },
  });
  const current = quote.data && quote.data.brl_centavos === centavos ? quote.data : null;

  function submit(event: FormEvent) {
    event.preventDefault();
    if (current) pay.mutate(current);
  }

  const error = pay.error ?? (valid ? quote.error : null);
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t.buying.add}</DialogTitle>
          <DialogDescription>{t.buying.description}</DialogDescription>
          <p className="text-muted-foreground text-sm">{t.buying.accepted}</p>
        </DialogHeader>
        <form onSubmit={submit} className="flex flex-col gap-3">
          <Label htmlFor="add-credits-amount">{t.buying.amount}</Label>
          <Input
            id="add-credits-amount"
            inputMode="decimal"
            autoComplete="off"
            value={text}
            onChange={(event) => setText(event.target.value)}
          />
          <p className="text-muted-foreground text-xs">
            {t.buying.limits(formatReais(MINIMUM_CENTAVOS), formatReais(MAXIMUM_CENTAVOS))}
          </p>
          <div aria-live="polite" className="min-h-12">
            {current && (
              <>
                <p className="font-medium">
                  {t.buying.preview(
                    formatReais(current.brl_centavos),
                    formatDollars(wholeCents(current.credit_micros)),
                  )}
                </p>
                {current.bonus_micros > 0 && (
                  <p className="font-medium text-primary">
                    {t.buying.bonus(
                      formatDollars(wholeCents(current.bonus_micros)),
                      current.bonus_percent,
                    )}
                  </p>
                )}
                <p className="text-muted-foreground text-sm">
                  {t.buying.rate(current.brl_per_usd)}
                </p>
              </>
            )}
            {error && <p className="text-destructive text-sm">{error.message}</p>}
          </div>
          <Button type="submit" disabled={!current || pay.isPending}>
            {pay.isPending ? t.buying.paying : t.buying.pay}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  );
}
