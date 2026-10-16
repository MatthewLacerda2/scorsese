// The landing page (#904): what a visitor without a session sees at `/`, and
// at `/login` with the sign-in popup already open, so the session guard's
// redirect and old bookmarks land somewhere sensible. A visitor who is signed
// in — already, or the moment the popup succeeds — goes where they were headed
// (`?next=`, else `/`, which is the projects list for them).
//
// What it shows is what scorsese makes: the films in it are cuts of videos made
// with scorsese, and the motion is the kit's (motion.ts).

import "@/landing/landing.css";
import { useEffect, useState } from "react";
import { Navigate, useNavigate, useSearchParams } from "react-router";
import { useT } from "@/i18n/I18nProvider";
import { Header } from "@/landing/Header";
import { Hero } from "@/landing/Hero";
import { Showcase } from "@/landing/Showcase";
import { SignInDialog } from "@/landing/SignInDialog";
import { Slides } from "@/landing/Slides";
import { Closing } from "@/landing/sections/Closing";
import { HowItWorks } from "@/landing/sections/HowItWorks";
import { WhatItHandles } from "@/landing/sections/WhatItHandles";
import { Footer } from "@/legal/Footer";
import { safeNext, useAccount } from "@/session/session";

/** The page's title and description, in the visitor's language, for as long as it is shown. */
function useMeta(title: string, description: string) {
  useEffect(() => {
    const before = document.title;
    document.title = title;
    const meta = document.querySelector<HTMLMetaElement>('meta[name="description"]');
    const was = meta?.content;
    if (meta) meta.content = description;
    return () => {
      document.title = before;
      if (meta && was !== undefined) meta.content = was;
    };
  }, [title, description]);
}

export function LandingPage({ signIn = false }: { signIn?: boolean }) {
  const t = useT();
  const account = useAccount();
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const [open, setOpen] = useState(signIn);
  useMeta(t.landing.meta.title, t.landing.meta.description);

  if (account.data) return <Navigate to={safeNext(params.get("next"))} replace />;

  const openSignIn = () => setOpen(true);
  const onOpenChange = (next: boolean) => {
    setOpen(next);
    // Closed at `/login`, the address follows: the popup is what that URL means.
    if (!next && signIn) navigate("/", { replace: true });
  };

  return (
    <div className="landing">
      <div className="grain" aria-hidden />
      <Header onSignIn={openSignIn} />
      <main>
        <Slides>
          <Hero onSignIn={openSignIn} />
          <Showcase />
          <HowItWorks />
          <WhatItHandles />
          <div>
            <Closing onSignIn={openSignIn} />
            <Footer>
              <p className="mb-4">
                <span className="display text-base text-paper">scorsese</span> · {t.landing.footer}
              </p>
            </Footer>
          </div>
        </Slides>
      </main>
      <SignInDialog open={open} onOpenChange={onOpenChange} />
    </div>
  );
}
