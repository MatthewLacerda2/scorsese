// The landing page's middle way in (#1566): right after the film, once the
// visitor has seen what comes out, one plain line and the same prompt box as
// the first screen, with the same hand-off. The first screen and `Closing`
// are the other two.

import { useT } from "@/i18n/I18nProvider";
import { PromptBox, type PromptSubmit } from "@/prompt/PromptBox";

export function Ask({ onPrompt }: { onPrompt: PromptSubmit }) {
  const t = useT().landing.ask;
  return (
    <section className="mx-auto max-w-2xl px-4 pb-16 text-center sm:px-6 sm:pb-20">
      <h2 className="display text-2xl font-medium text-balance sm:text-3xl" data-rise="0.1">
        {t.title}
      </h2>
      <div className="mt-7" data-rise="0.2">
        <PromptBox onSubmit={onPrompt} />
      </div>
    </section>
  );
}
