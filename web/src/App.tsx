// The app: one query cache and the routes. The router is passed in so the
// browser uses its address bar (`BrowserRouter`, main.tsx) and a test can
// render any URL without one (`MemoryRouter` / `StaticRouter`).

import { type QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type ReactNode, useState } from "react";
import { createQueryClient } from "@/app/queryClient";
import { AppRoutes } from "@/app/routes";

interface Props {
  /** The router the routes run in. */
  router: (routes: ReactNode) => ReactNode;
  /** A cache to start from — tests seed one; the browser makes a fresh one. */
  queryClient?: QueryClient;
}

export function App({ router, queryClient }: Props) {
  const [client] = useState(() => queryClient ?? createQueryClient());
  return <QueryClientProvider client={client}>{router(<AppRoutes />)}</QueryClientProvider>;
}
