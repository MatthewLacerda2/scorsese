// Every page, by URL. Everything but the landing page sits behind the session
// guard and inside the shell (header, balance, upload tray).
//
// `/` is the landing page for a visitor without a session and the projects
// list for one with (#904): they came to work. `/login` is the landing page
// with the sign-in popup open, which is where the guard sends everyone else.
//
// The editor (#545) is at `/projects/:id/edit`, inside the shell like the
// rest — the header's balance is what an assistant turn spends — which gives
// it the whole height under the header rather than a padded page.
//
// Every page comes from `pages.ts`, lazily (#896): a new page is added there
// and named here, never imported statically.

import { Suspense } from "react";
import { Navigate, Route, Routes } from "react-router";
import { PageFallback } from "@/app/PageFallback";
import {
  EditorPage,
  LandingPage,
  LibraryPage,
  LoginPage,
  NotFound,
  ProjectFilesPage,
  ProjectsPage,
  SpendingPage,
} from "@/app/pages";
import { RequireSession, useAccount } from "@/session/session";
import { Shell } from "./Shell";

/** `/`: the projects for a signed-in visitor, the landing page for anyone else. */
function Home() {
  const account = useAccount();
  // Nothing while asking: a signed-in visitor should not see the landing page flash.
  if (account.isPending) return <div className="min-h-svh bg-[#07070a]" />;
  if (account.data) return <Navigate to="/projects" replace />;
  return (
    <Suspense fallback={<PageFallback />}>
      <LandingPage />
    </Suspense>
  );
}

export function AppRoutes() {
  return (
    <Routes>
      <Route index element={<Home />} />
      <Route
        path="/login"
        element={
          <Suspense fallback={<PageFallback />}>
            <LoginPage />
          </Suspense>
        }
      />
      <Route element={<RequireSession />}>
        <Route element={<Shell />}>
          <Route path="/projects" element={<ProjectsPage />} />
          <Route path="/projects/:id" element={<ProjectFilesPage />} />
          <Route path="/projects/:id/edit" element={<EditorPage />} />
          <Route path="/library" element={<LibraryPage />} />
          <Route path="/spending" element={<SpendingPage />} />
          <Route path="*" element={<NotFound />} />
        </Route>
      </Route>
    </Routes>
  );
}
