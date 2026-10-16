// The browser entry: mount the app into `index.html`'s root element.
//
// `BrowserRouter` uses real paths (`/library`, not `#/library`); the deploy's
// nginx answers any path that is not `/api` or a file with `index.html`, so a
// reload or a pasted link lands on the right page.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router";
import { App } from "@/App";
import { listenForCrashes } from "@/app/crashes";
import "@/index.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("index.html has no #root element to mount the app into");
}

// What throws outside React, or rejects with nothing to catch it, is reported
// too (#1428); what throws inside it, its error boundaries report.
listenForCrashes();

createRoot(root).render(
  <StrictMode>
    <App router={(routes) => <BrowserRouter>{routes}</BrowserRouter>} />
  </StrictMode>,
);
