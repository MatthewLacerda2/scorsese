// `/pricing`: public and crawlable, in the landing's look and theme, with its
// header and footer. Signed out, every button opens the sign-in popup; signed
// in, the free one goes home and a buy one opens Add credits with its amount.

import "@/landing/landing.css";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { AddCreditsProvider, useAddCredits } from "@/app/addCredits";
import { useT } from "@/i18n/I18nProvider";
import { Header } from "@/landing/Header";
import { SignInDialog } from "@/landing/SignInDialog";
import { Footer } from "@/legal/Footer";
import { Costs } from "@/pricing/Costs";
import { Faq } from "@/pricing/Faq";
import { PlanCards } from "@/pricing/PlanCards";
import { useAccount } from "@/session/session";

function Content() {
  const t = useT().pricing;
  const account = useAccount();
  const navigate = useNavigate();
  const addCredits = useAddCredits();
  const [signIn, setSignIn] = useState(false);
  const signedIn = Boolean(account.data);

  useEffect(() => {
    const before = document.title;
    document.title = t.meta.title;
    return () => {
      document.title = before;
    };
  }, [t.meta.title]);

  return (
    <div className="landing">
      <div className="grain" aria-hidden />
      <Header signedIn={signedIn} onSignIn={() => setSignIn(true)} />
      <main className="mx-auto max-w-6xl px-4 pt-32 pb-8 sm:px-6">
        <h1 className="display text-5xl font-semibold text-balance sm:text-6xl">{t.title}</h1>
        <p className="dim mt-5 max-w-2xl text-lg leading-relaxed">{t.lead}</p>
        <PlanCards
          onStart={() => (signedIn ? navigate("/") : setSignIn(true))}
          onBuy={(reais) => (signedIn ? addCredits(reais) : setSignIn(true))}
        />
        <Costs />
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
