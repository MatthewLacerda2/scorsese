// `/pricing`: public and crawlable, in the landing's look and theme, with its
// header and footer. Its figures are the server's (`usePrices`, #1538).
// Signed out, every button opens the sign-in popup; signed in, the free one
// goes home and a buy one goes straight to Stripe's Checkout at the rate its
// card shows — or, when the rate moved, shows the card's new figure and why,
// for a second click. A card with no rate to show opens Add credits with its
// amount instead.

import "@/landing/landing.css";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useNavigate } from "react-router";
import { ApiError, api, type PaymentQuote, type Prices, type RechargeCard } from "@/api";
import { AddCreditsProvider, useAddCredits } from "@/app/addCredits";
import { queryKeys } from "@/app/queryKeys";
import { useT } from "@/i18n/I18nProvider";
import { Header } from "@/landing/Header";
import { SignInDialog } from "@/landing/SignInDialog";
import { Footer } from "@/legal/Footer";
import { useMeta } from "@/lib/useMeta";
import { Costs } from "@/pricing/Costs";
import { Faq } from "@/pricing/Faq";
import { PlanCards } from "@/pricing/PlanCards";
import { usePrices } from "@/pricing/usePrices";
import { useAccount } from "@/session/session";

/** `prices` with `card`'s quote replaced by `quote`, the one a `409` brought. */
function requoted(prices: Prices, quote: PaymentQuote): Prices {
  if (!prices.recharge) return prices;
  const cards = prices.recharge.cards.map((card) =>
    card.brl_centavos === quote.brl_centavos ? { ...card, quote } : card,
  );
  return { ...prices, recharge: { ...prices.recharge, cards } };
}

function Content() {
  const t = useT().pricing;
  const account = useAccount();
  const navigate = useNavigate();
  const addCredits = useAddCredits();
  const prices = usePrices();
  const client = useQueryClient();
  const [signIn, setSignIn] = useState(false);
  const [notes, setNotes] = useState<Record<number, string>>({});
  const signedIn = Boolean(account.data);

  const buy = useMutation({
    mutationFn: (card: RechargeCard & { quote: PaymentQuote }) =>
      api.payments.start(card.brl_centavos, card.quote.brl_per_usd),
    onMutate: (card) => setNotes(({ [card.brl_centavos]: _, ...rest }) => rest),
    onSuccess: (begun) => window.location.assign(begun.url),
    onError: (failure, card) => {
      const moved =
        failure instanceof ApiError && failure.status === 409
          ? (failure.body.quote as PaymentQuote | undefined)
          : undefined;
      if (moved)
        client.setQueryData<Prices>(queryKeys.prices, (now) => now && requoted(now, moved));
      setNotes((now) => ({ ...now, [card.brl_centavos]: failure.message }));
    },
  });

  function onBuy(card: RechargeCard) {
    if (!signedIn) setSignIn(true);
    else if (card.quote) buy.mutate({ ...card, quote: card.quote });
    else addCredits(card.brl_centavos / 100);
  }

  useMeta(t.meta.title, t.meta.description);

  return (
    <div className="landing">
      <div className="grain" aria-hidden />
      <Header signedIn={signedIn} onSignIn={() => setSignIn(true)} />
      <main className="mx-auto max-w-6xl px-4 pt-32 pb-8 sm:px-6">
        <h1 className="display text-5xl font-semibold text-balance sm:text-6xl">{t.title}</h1>
        <p className="dim mt-5 max-w-2xl text-lg leading-relaxed">{t.lead}</p>
        {prices.data && (
          <PlanCards
            free={prices.data.free}
            cards={prices.data.recharge?.cards ?? []}
            onStart={() => (signedIn ? navigate("/") : setSignIn(true))}
            onBuy={onBuy}
            buying={buy.isPending ? buy.variables?.brl_centavos : undefined}
            notes={notes}
          />
        )}
        {prices.error && <p className="mt-12 text-destructive">{prices.error.message}</p>}
        {prices.data && <Costs examples={prices.data.examples} />}
        <Faq />
      </main>
      <Footer />
      <SignInDialog open={signIn} onOpenChange={setSignIn} />
    </div>
  );
}

export function PricingPage() {
  return (
    <AddCreditsProvider>
      <Content />
    </AddCreditsProvider>
  );
}
