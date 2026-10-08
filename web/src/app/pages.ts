// Every page, loaded on demand (#896). A page is declared here as a lazy
// import and never imported statically anywhere else, so each one lands in a
// chunk of its own and the entry chunk carries only what every signed-in page
// needs: the router, the query cache, the session, the shell and its uploads,
// the languages and the theme. Opening the login page no longer downloads the
// editor. `vite.config.ts`'s budget fails the build if the entry grows past
// that, which is how this stays true without anyone remembering it.
//
// Navigating is a transition (React Router's default), so the page on screen
// stays until the next one's chunk has arrived; only the first page of a visit
// ever shows `PageFallback`.

import { type ComponentType, type LazyExoticComponent, lazy } from "react";

/** A lazily loaded page, which can also be fetched ahead of being shown. */
export type LazyPage = LazyExoticComponent<ComponentType> & {
  /** Start downloading the page's chunk; calling it again costs nothing. */
  preload: () => Promise<unknown>;
};

/** The page named `name` in the module `load` imports, as a lazy component. */
export function lazyPage<M, K extends keyof M>(load: () => Promise<M>, name: K): LazyPage {
  let loading: Promise<M> | undefined;
  const preload = () => {
    loading ??= load();
    return loading;
  };
  const page = lazy(() => preload().then((module) => ({ default: module[name] as ComponentType })));
  return Object.assign(page, { preload });
}

export const LoginPage = lazyPage(() => import("@/pages/LoginPage"), "LoginPage");
export const ProjectsPage = lazyPage(() => import("@/pages/ProjectsPage"), "ProjectsPage");
export const LibraryPage = lazyPage(() => import("@/pages/FilesPages"), "LibraryPage");
export const ProjectFilesPage = lazyPage(() => import("@/pages/FilesPages"), "ProjectFilesPage");
export const SpendingPage = lazyPage(() => import("@/pages/SpendingPage"), "SpendingPage");
export const EditorPage = lazyPage(() => import("@/editor/EditorPage"), "EditorPage");
export const NotFound = lazyPage(() => import("@/pages/NotFound"), "NotFound");
