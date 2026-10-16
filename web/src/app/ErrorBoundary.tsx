// The last guard between a broken piece of the page and a blank screen. React
// unmounts everything under an error nobody catches, and on the dark theme
// that is a black page with no way out but a reload nobody thinks of: on
// 2026-10-16 a busy line that threw in Portuguese did exactly that. Caught
// here, the page says so and offers the reload.

import { Component, type ErrorInfo, type ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";

interface Props {
  children: ReactNode;
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
  }

  render() {
    return this.state.failed ? <Crashed /> : this.props.children;
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
