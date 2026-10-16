// The landing page's header: only the language and the way in, since the name
// is already the largest thing on the page (`Hero.tsx`).

import { LanguageControl } from "@/app/LanguageControl";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";

export function Header({ onSignIn }: { onSignIn: () => void }) {
  const t = useT();
  return (
    <header className="dark fixed inset-x-0 top-0 z-20 mx-auto flex max-w-6xl items-center justify-end gap-2 px-4 py-5 sm:gap-3 sm:px-6">
      <LanguageControl className="border-paper/15 bg-transparent" />
      <Button
        type="button"
        variant="ghost"
        onClick={onSignIn}
        className="ghost h-auto px-4 py-1.5 text-sm hover:bg-paper/10 hover:text-paper dark:hover:bg-paper/10"
      >
        {t.landing.signIn}
      </Button>
    </header>
  );
}
