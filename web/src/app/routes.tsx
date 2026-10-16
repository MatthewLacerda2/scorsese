// Every page, by URL. Everything but the landing page and the legal pages sits behind the session
// guard and inside the shell (header, balance, upload tray).
//
// `/terms` and `/privacy` (#547) are public too, signed in or not.
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
  CreditsReturnPage,
  EditorPage,
  LandingPage,
  LibraryPage,
  LoginPage,
  NotFound,
  PrivacyPage,
  ProjectFilesPage,
  ProjectsPage,
  SpendingPage,
  TermsPage,
} from "@/app/pages";
import { signedInHint } from "@/session/hint";
import { RequireSession, useAccount } from "@/session/session";
import { Shell } from "./Shell";

/**
 * `/`: the projects for a signed-in visitor, the landing page for anyone else.
 *
 * Which one is decided before any request, by the hint the server sets beside
 * the session (#1246, `session/hint.ts`). Without it — the anonymous visitor,
 * the common case and the one whose first load has to be fast — the landing
 * page is drawn at once, its chunk and the session check going out together
 * (#1238). With it, the page waits on the session behind the landing page's
 * dark ground, so a signed-in visitor never sees the landing page flash nor
 * starts its film, and is sent on to `/projects`. A stale hint only costs that
 * wait, and is dropped when the session answers as nobody.
 */
function Home() {
  const account = useAccount();
  const ground = <div className="min-h-svh bg-ink" />;
  if (account.data) return <Navigate to="/projects" replace />;
  if (account.isPending && signedInHint()) return ground;
  return (
    <Suspense fallback={ground}>
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
      <Route
        path="/terms"
        element={
          <Suspense fallback={<div className="min-h-svh bg-ink" />}>
            <TermsPage />
          </Suspense>
        }
      />
      <Route
        path="/privacy"
        element={
          <Suspense fallback={<div className="min-h-svh bg-ink" />}>
            <PrivacyPage />
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
          <Route path="/credits/return" element={<CreditsReturnPage />} />
          <Route path="*" element={<NotFound />} />
        </Route>
      </Route>
    </Routes>
  );
}
