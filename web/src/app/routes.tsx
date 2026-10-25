// Every page, by URL. Everything but the landing page, the pricing and the legal pages sits behind
// the session guard: the library and spending pages inside the sidebar layout, the rest inside the
// shell (header, balance, upload tray).
//
// `/terms` and `/privacy` (#547) and `/pricing` are public too, signed in or not.
//
// `/` is the landing page for a visitor without a session and the home (the
// sidebar and the prompt box) for one with (#904): they came to work.
// `/projects` is that home too. `/login` is the landing page
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
  HomePage,
  LandingPage,
  LibraryPage,
  LoginPage,
  NotFound,
  PricingPage,
  PrivacyPage,
  ProjectFilesPage,
  SidebarLayout,
  SpendingPage,
  TermsPage,
} from "@/app/pages";
import { signedInHint } from "@/session/hint";
import { RequireSession, useAccount } from "@/session/session";
import { Shell } from "./Shell";

/**
 * `/`: the home for a signed-in visitor, the landing page for anyone else.
 *
 * Which one is decided before any request, by the hint the server sets beside
 * the session (#1246, `session/hint.ts`). Without it — the anonymous visitor,
 * the common case and the one whose first load has to be fast — the landing
 * page is drawn at once, its chunk and the session check going out together
 * (#1238). With it, the page waits on the session behind the landing page's
 * dark ground, so a signed-in visitor never sees the landing page flash nor
 * starts its film, and gets the home. A stale hint only costs that
 * wait, and is dropped when the session answers as nobody.
 */
function Home() {
  const account = useAccount();
  const ground = <div className="min-h-svh bg-ink" />;
  if (account.isPending && signedInHint()) return ground;
  if (account.data) {
    return (
      <Suspense fallback={ground}>
        <HomePage />
      </Suspense>
    );
  }
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
        path="/pricing"
        element={
          <Suspense fallback={<div className="min-h-svh bg-ink" />}>
            <PricingPage />
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
        <Route path="/projects" element={<Navigate to="/" replace />} />
        <Route
          element={
            <Suspense fallback={<PageFallback />}>
              <SidebarLayout />
            </Suspense>
          }
        >
          <Route path="/library" element={<LibraryPage />} />
          <Route path="/spending" element={<SpendingPage />} />
        </Route>
        <Route element={<Shell />}>
          <Route path="/projects/:id" element={<ProjectFilesPage />} />
          <Route path="/projects/:id/edit" element={<EditorPage />} />
          <Route path="/credits/return" element={<CreditsReturnPage />} />
          <Route path="*" element={<NotFound />} />
        </Route>
      </Route>
    </Routes>
  );
}
