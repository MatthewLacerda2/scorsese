// The landing page (#904): what a visitor without a session sees at `/`, and
// at `/login` with the sign-in popup already open, so the session guard's
// redirect and old bookmarks land somewhere sensible. A visitor who is signed
// in — already, or the moment the popup succeeds — goes where they were headed
// (`?next=`, else `/`, which is the projects list for them).
//
// It is built with what scorsese makes videos with: the film at the top is
// rendered by scorsese from web/landing/hero.scor, the motion is the kit's,
// and the type is the shipped faces (landing.css).

import "@/landing/landing.css";
import { useEffect, useState } from "react";
import { Navigate, useNavigate, useSearchParams } from "react-router";
import { useT } from "@/i18n/I18nProvider";
import { Header, Hero } from "@/landing/Hero";
import { Closing, HowItWorks, Price, WhatItHandles, WhatYouCanMake } from "@/landing/Sections";
import { SignInDialog } from "@/landing/SignInDialog";
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
        <Hero onSignIn={openSignIn} />
        <HowItWorks />
        <WhatYouCanMake />
        <WhatItHandles />
        <Price />
        <Closing onSignIn={openSignIn} />
      </main>
      <footer className="dim mx-auto max-w-6xl border-t border-[#f4efe6]/10 px-4 py-10 text-sm sm:px-6">
        <span className="display text-base text-[#f4efe6]">scorsese</span> · {t.landing.footer}
      </footer>
      <SignInDialog open={open} onOpenChange={onOpenChange} />
    </div>
  );
}

/** `/login`: the landing page with the popup open. */
export function LoginPage() {
  return <LandingPage signIn />;
}
