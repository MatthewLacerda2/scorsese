// The public pages' header (the landing, pricing and the legal pages): at the
// left the small mark and name, a way home, and the Pricing, Terms and Privacy
// links; at the right the theme, the language and the way in, all one height.
// It sits on a blurred band of the page's ink, so what scrolls under it stays
// readable.

import { Link } from "react-router";
import { LanguageControl } from "@/app/LanguageControl";
import { ThemeControl } from "@/app/ThemeControl";
import { Mark } from "@/components/Mark";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";

const NAV_LINK =
  "rounded-sm text-paper/70 transition-colors hover:text-paper focus-visible:text-paper focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-paper";
const WAY_IN = "h-9 rounded-md bg-paper px-4 text-sm text-ink hover:bg-paper/85";

export function Header({
  onSignIn,
  signedIn = false,
}: {
  onSignIn: () => void;
  /** Signed in, the last control is a way back into the app, not a sign-in. */
  signedIn?: boolean;
}) {
  const t = useT();
  return (
    <header className="fixed inset-x-0 top-0 z-20 border-b border-paper/5 bg-ink/75 backdrop-blur-md">
      <div className="mx-auto flex h-16 max-w-6xl items-center gap-2 px-4 sm:gap-3 sm:px-6">
        <Link to="/" className={`display flex items-center gap-2 text-lg text-paper ${NAV_LINK}`}>
          <Mark kind="icon" size={28} className="size-7 rounded-md" />
          <span className="max-sm:sr-only">scorsese</span>
        </Link>
        {/* On a phone there is no room beside the controls; the footer has the same links. */}
        <nav className="ml-5 hidden gap-5 text-sm sm:flex">
          <Link to="/pricing" className={NAV_LINK}>
            {t.landing.legal.pricingShort}
          </Link>
          <Link to="/terms" className={NAV_LINK}>
            {t.landing.legal.termsShort}
          </Link>
          <Link to="/privacy" className={NAV_LINK}>
            {t.landing.legal.privacyShort}
          </Link>
        </nav>
        <ThemeControl compact className="ml-auto h-9 items-center border-paper/15" />
        <LanguageControl className="border-paper/15 bg-transparent data-[size=sm]:h-9" />
        {signedIn ? (
          <Button asChild className={WAY_IN}>
            <Link to="/">{t.landing.openApp}</Link>
          </Button>
        ) : (
          <Button type="button" onClick={onSignIn} className={WAY_IN}>
            {t.landing.signIn}
          </Button>
        )}
      </div>
    </header>
  );
}
