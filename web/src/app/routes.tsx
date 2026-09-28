// Every page, by URL. Everything but the login sits behind the session guard
// and inside the shell (header, balance, upload tray).
//
// The editor (#545) belongs at `/projects/:id/edit`, beside `/projects/:id`:
// inside `RequireSession` always, inside `Shell` only if it wants the header.

import { Navigate, Route, Routes } from "react-router";
import { LibraryPage, ProjectFilesPage } from "@/pages/FilesPages";
import { LoginPage } from "@/pages/LoginPage";
import { NotFound } from "@/pages/NotFound";
import { ProjectsPage } from "@/pages/ProjectsPage";
import { SpendingPage } from "@/pages/SpendingPage";
import { RequireSession } from "@/session/session";
import { Shell } from "./Shell";

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route element={<RequireSession />}>
        <Route element={<Shell />}>
          <Route index element={<Navigate to="/projects" replace />} />
          <Route path="/projects" element={<ProjectsPage />} />
          <Route path="/projects/:id" element={<ProjectFilesPage />} />
          <Route path="/library" element={<LibraryPage />} />
          <Route path="/spending" element={<SpendingPage />} />
          <Route path="*" element={<NotFound />} />
        </Route>
      </Route>
    </Routes>
  );
}
