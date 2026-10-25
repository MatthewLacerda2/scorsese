// The landing page's last way in: one line and the same prompt box as the
// first screen, a part like the rest (`Section.tsx` says how they move).
// The sign-in button that used to sit here is gone: two ways in side by side
// would compete, and the header still has one.

import { useT } from "@/i18n/I18nProvider";
import { PromptBox } from "@/prompt/PromptBox";

export function Closing({ onSignIn }: { onSignIn: () => void }) {
  const t = useT().landing;
  return (
    <section className="mx-auto max-w-4xl px-4 pt-24 pb-14 text-center sm:px-6 sm:pt-32 sm:pb-16">
      <h2 className="display text-5xl font-medium text-balance sm:text-7xl" data-rise="0.1">
        {t.closing.title}
      </h2>
      <p className="dim mx-auto mt-6 max-w-xl text-lg" data-rise="0.2">
        {t.closing.body}
      </p>
      <div className="mx-auto mt-10 max-w-2xl" data-rise="0.3">
        <PromptBox onSubmit={onSignIn} />
      </div>
    </section>
  );
}
