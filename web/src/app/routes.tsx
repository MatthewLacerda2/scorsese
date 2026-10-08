// Every page, by URL. Everything but the login sits behind the session guard
// and inside the shell (header, balance, upload tray).
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
  LibraryPage,
  LoginPage,
  NotFound,
  ProjectFilesPage,
  ProjectsPage,
  SpendingPage,
} from "@/app/pages";
import { RequireSession } from "@/session/session";
import { Shell } from "./Shell";

export function AppRoutes() {
  return (
    <Routes>
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
          <Route index element={<Navigate to="/projects" replace />} />
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
