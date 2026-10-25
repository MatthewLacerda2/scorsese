// The signed-in home, `/` for a visitor with a session: the sidebar (library,
// credits, projects, settings, the way out) and, in the middle, a title that
// changes and the prompt box. Enter makes a project from the prompt and opens
// it (`useStartFromPrompt`).

import { useEffect } from "react";
import { EditorPage } from "@/app/pages";
import { RotatingTitle } from "@/home/RotatingTitle";
import { SidebarLayout } from "@/home/SidebarLayout";
import { useStartFromPrompt } from "@/home/useStartFromPrompt";
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
  const start = useStartFromPrompt();
  usePreloadEditor();
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-8 px-4 pb-24 text-center">
      <RotatingTitle />
      <div className="w-full max-w-2xl">
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
