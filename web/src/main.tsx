// The browser entry: mount the app into `index.html`'s root element.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "@/App";
import "@/index.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("index.html has no #root element to mount the app into");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
