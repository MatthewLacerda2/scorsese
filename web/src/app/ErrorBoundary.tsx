// The guard between a broken piece of the page and a blank screen. React
// unmounts everything under an error nobody catches, and on the dark theme
// that is a black page with no way out but a reload nobody thinks of: on
// 2026-10-16 a busy line that threw in Portuguese did exactly that. Caught
// here, the page says so and offers the reload.
//
// One goes around the whole app as the last net (#1426); the editor puts one
// around each of its panels (#1428), so the panel that threw says so in its
// own place and the rest of the editor keeps working. Either way, what threw
// is reported to the server's log (`crashes.ts`).

import { Component, type ErrorInfo, type ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import { reportCrash } from "./crashes";

interface Props {
  children: ReactNode;
  /** What failing takes down: the whole `page` (the default) or one `panel`. */
  place?: "page" | "panel";
}

interface State {
  failed: boolean;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { failed: false };

  static getDerivedStateFromError(): State {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("the page crashed", error, info.componentStack);
    reportCrash(error, info.componentStack);
  }

  render() {
    if (!this.state.failed) return this.props.children;
    return this.props.place === "panel" ? <PanelCrashed /> : <Crashed />;
  }
}

/** What the page shows once something under it threw. */
function Crashed() {
  const t = useT();
  return (
    <div className="flex min-h-svh flex-col items-center justify-center gap-3 p-6 text-center">
      <p>{t.common.crashed.text}</p>
      <Button onClick={() => window.location.reload()}>{t.common.crashed.reload}</Button>
    </div>
  );
}

/** What a panel shows in its own place once something in it threw. */
function PanelCrashed() {
  const t = useT();
  return (
    <div className="flex h-full items-center justify-center gap-2 p-3 text-sm text-muted-foreground">
      <p>{t.common.crashed.part}</p>
      <Button size="sm" variant="outline" onClick={() => window.location.reload()}>
        {t.common.crashed.reload}
      </Button>
    </div>
  );
}
