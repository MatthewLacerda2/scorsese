// The app: one query cache and the routes. The router is passed in so the
// browser uses its address bar (`BrowserRouter`, main.tsx) and a test can
// render any URL without one (`MemoryRouter` / `StaticRouter`).

import { type QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type ReactNode, useState } from "react";
import { createQueryClient } from "@/app/queryClient";
import { AppRoutes } from "@/app/routes";
import { I18nProvider } from "@/i18n/I18nProvider";
import type { Language } from "@/i18n/language";

interface Props {
  /** The router the routes run in. */
  router: (routes: ReactNode) => ReactNode;
  /** A cache to start from — tests seed one; the browser makes a fresh one. */
  queryClient?: QueryClient;
  /** A language to start in — tests pick one; the browser reads its own. */
  language?: Language;
}

export function App({ router, queryClient, language }: Props) {
  const [client] = useState(() => queryClient ?? createQueryClient());
  return (
    <I18nProvider initial={language}>
      <QueryClientProvider client={client}>{router(<AppRoutes />)}</QueryClientProvider>
    </I18nProvider>
  );
}
