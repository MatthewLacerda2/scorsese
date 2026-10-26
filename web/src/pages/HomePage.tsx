// The signed-in home, `/` for a visitor with a session: the sidebar (library,
// credits, projects, settings, the way out) and, in the middle, the landing's
// headline and the line under it (version `a`: the A/B test is the landing's,
// for visitors) over the prompt box (#1593). Enter makes a project from the prompt and opens
// it (`useStartFromPrompt`).

import { useEffect } from "react";
import { EditorPage } from "@/app/pages";
import { SidebarLayout } from "@/home/SidebarLayout";
import { useStartFromPrompt } from "@/home/useStartFromPrompt";
import { useT } from "@/i18n/I18nProvider";
import { PromptBox } from "@/prompt/PromptBox";

/**
 * Fetch the editor's chunk once this page is idle, so opening a project does not wait on a
 * download (#896). Safari has no `requestIdleCallback`, so there it is a short timeout.
 */
function usePreloadEditor() {
  useEffect(() => {
    const preload = () => void EditorPage.preload();
    if ("requestIdleCallback" in window) {
      const id = requestIdleCallback(preload);
      return () => cancelIdleCallback(id);
    }
    const id = setTimeout(preload, 1000);
    return () => clearTimeout(id);
  }, []);
}

function Start() {
  const t = useT().landing.hero;
  const start = useStartFromPrompt();
  usePreloadEditor();
  return (
    <div className="flex flex-1 flex-col items-center justify-center px-4 pb-24 text-center">
      <h1 className="font-heading text-3xl leading-tight font-semibold tracking-tight text-balance sm:text-5xl">
        {t.title}
      </h1>
      <p className="mt-3 text-base text-balance text-muted-foreground sm:text-lg">{t.subtitle}</p>
      <div className="mt-8 w-full max-w-2xl">
        <PromptBox onSubmit={start} autoFocus />
      </div>
    </div>
  );
}

export function HomePage() {
  return (
    <SidebarLayout>
      <Start />
    </SidebarLayout>
  );
}
