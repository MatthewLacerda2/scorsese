// The landing page's header: only the language and the way in, since the name
// is already the largest thing on the page (`Hero.tsx`).

import { LanguageControl } from "@/app/LanguageControl";
import { useT } from "@/i18n/I18nProvider";

export function Header({ onSignIn }: { onSignIn: () => void }) {
  const t = useT();
  return (
    <header className="dark fixed inset-x-0 top-0 z-20 mx-auto flex max-w-6xl items-center justify-end gap-2 px-4 py-5 sm:gap-3 sm:px-6">
      <LanguageControl className="border-paper/15 bg-transparent" />
      <button type="button" onClick={onSignIn} className="ghost px-4 py-1.5 text-sm">
        {t.landing.signIn}
      </button>
    </header>
  );
}
