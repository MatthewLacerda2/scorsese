// The landing page's last way in: one line and the sign-in button, a slide
// like the rest (`HowItWorks.tsx` says how they move).

import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";

export function Closing({ onSignIn }: { onSignIn: () => void }) {
  const t = useT().landing;
  return (
    <section className="mx-auto max-w-6xl px-4 py-24 text-center sm:px-6 sm:py-32">
      <h2 className="display text-5xl font-medium italic sm:text-7xl" data-rise="0.1">
        {t.closing.title}
      </h2>
      <p className="dim mx-auto mt-6 max-w-xl text-lg" data-rise="0.2">
        {t.closing.body}
      </p>
      <div className="mt-10" data-rise="0.3">
        <Button
          type="button"
          onClick={onSignIn}
          className="h-auto cta px-8 py-3.5 text-base hover:bg-(--gold) hover:text-(--ink)"
        >
          {t.signIn}
        </Button>
      </div>
    </section>
  );
}
