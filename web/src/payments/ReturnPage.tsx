// Where Stripe sends a buyer back to (#548): `/credits/return?session=…`.
// It asks the server to confirm the payment — which credits it, if Stripe
// says it is paid and the webhook has not arrived yet — and asks again every
// few seconds while it is still open. Once it is paid the balance in the
// header is refreshed, so the person sees it move.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { Link, useSearchParams } from "react-router";
import { api } from "@/api";
import { queryKeys } from "@/app/queryKeys";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";
import { wholeCents } from "@/payments/reais";

/** How often an open payment is asked about again. */
const POLL_MS = 3_000;

export function CreditsReturnPage() {
  const t = useT();
  const [params] = useSearchParams();
  const session = params.get("session") ?? "";
  const client = useQueryClient();
  const payment = useQuery({
    queryKey: queryKeys.paymentConfirm(session),
    queryFn: () => api.payments.confirm(session),
    enabled: session !== "",
    retry: false,
    refetchInterval: (query) => (query.state.data?.state === "open" ? POLL_MS : false),
  });
  const state = payment.data?.state;
  useEffect(() => {
    if (state === "paid") client.invalidateQueries({ queryKey: queryKeys.credits });
  }, [state, client]);

  let message: string = t.buying.returned.checking;
  if (!session || payment.isError) message = payment.error?.message ?? t.buying.returned.missing;
  else if (payment.data?.state === "paid")
    message = t.buying.returned.paid(
      formatDollars(wholeCents(payment.data.credit_micros + payment.data.bonus_micros)),
    );
  else if (state === "open") message = t.buying.returned.open;
  else if (state === "expired") message = t.buying.returned.expired;

  return (
    <div className="mx-auto flex max-w-md flex-col gap-4 py-8">
      <h1 className="font-semibold text-xl">{t.buying.returned.title}</h1>
      <p aria-live="polite">{message}</p>
      <div className="flex gap-2">
        <Button asChild>
          <Link to="/">{t.buying.returned.projects}</Link>
        </Button>
        <Button asChild variant="outline">
          <Link to="/spending">{t.buying.returned.history}</Link>
        </Button>
      </div>
    </div>
  );
}
