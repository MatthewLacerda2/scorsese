// The landing page's header: at the left the small mark and name (a way home;
// on the landing's own first screen the big ones are already there, so they
// show only after it scrolls away) and the Pricing, Terms and Privacy links;
// at the right the theme, the language and the way in, all one height. It sits on a
// blurred band of the page's ink, so what scrolls under it stays readable.

import { useEffect, useState } from "react";
import { Link } from "react-router";
import { LanguageControl } from "@/app/LanguageControl";
import { ThemeControl } from "@/app/ThemeControl";
import { Mark } from "@/components/Mark";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";

/** Whether the page has scrolled past most of the first screen; always true unless `watch`. */
function usePastFirstScreen(watch: boolean): boolean {
  const [past, setPast] = useState(!watch);
  useEffect(() => {
    if (!watch) return;
    const check = () => setPast(scrollY > innerHeight * 0.6);
    check();
    addEventListener("scroll", check, { passive: true });
    return () => removeEventListener("scroll", check);
  }, [watch]);
  return past;
}

const GHOST =
  "ghost h-9 px-4 py-0 text-sm hover:bg-paper/10 hover:text-paper dark:hover:bg-paper/10";

export function Header({
  onSignIn,
  signedIn = false,
  brandAfterFirstScreen = false,
}: {
  onSignIn: () => void;
  /** Signed in, the last control is a way back into the app, not a sign-in. */
  signedIn?: boolean;
  /** On the landing's first screen the big mark is already there: show the small one only once it has scrolled away. */
  brandAfterFirstScreen?: boolean;
}) {
  const pastFirstScreen = usePastFirstScreen(brandAfterFirstScreen);
  const t = useT();
  return (
    <header className="fixed inset-x-0 top-0 z-20 bg-ink/70 backdrop-blur-md">
      <div className="mx-auto flex max-w-6xl items-center gap-2 px-4 py-4 sm:gap-3 sm:px-6">
        <Link
          to="/"
          aria-hidden={!pastFirstScreen}
          tabIndex={pastFirstScreen ? undefined : -1}
          className={`display flex items-center gap-2 text-lg transition-opacity ${pastFirstScreen ? "opacity-100" : "pointer-events-none opacity-0"}`}
        >
          <Mark kind="icon" size={28} className="size-7 rounded-md" />
          <span className="max-sm:sr-only">scorsese</span>
        </Link>
        {/* On a phone there is no room beside the controls; the footer has the terms and privacy links. */}
        <nav className="ml-3 hidden gap-4 text-sm text-paper/70 sm:flex">
          <Link to="/pricing" className="hover:text-paper hover:underline">
            {t.landing.legal.pricingShort}
          </Link>
          <Link to="/terms" className="hover:text-paper hover:underline">
            {t.landing.legal.termsShort}
          </Link>
          <Link to="/privacy" className="hover:text-paper hover:underline">
            {t.landing.legal.privacyShort}
          </Link>
        </nav>
        <ThemeControl compact className="ml-auto h-9 items-center border-paper/15" />
        <LanguageControl className="border-paper/15 bg-transparent data-[size=sm]:h-9" />
        {signedIn ? (
          <Button asChild variant="ghost" className={GHOST}>
            <Link to="/">{t.landing.openApp}</Link>
          </Button>
        ) : (
          <Button type="button" variant="ghost" onClick={onSignIn} className={GHOST}>
            {t.landing.signIn}
          </Button>
        )}
      </div>
    </header>
  );
}
