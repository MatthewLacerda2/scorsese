// The build and the dev server, in one place.
//
// `vite build` writes static files to `dist/`, which is all the deploy's nginx
// container serves: there is no Node or Bun process in production, only files.
//
// In development the page and the API are two processes on two ports, and the
// browser would treat them as two origins. Proxying `/api` through this server
// keeps them one origin, which is the shape production has too — there the
// deploy's nginx makes the same split (deploy/nginx.conf). So a
// fetch written as `fetch("/api/...")` works unchanged in both.

import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { entryBudget } from "./src/build/budget.ts";
import { motionKit } from "./src/build/kit.ts";

// `entryBudget` fails the build when what a first visit downloads outgrows its
// budget; src/build/budget.ts has the number and why. `motionKit` joins the
// motion kit's pieces into the one script the landing page imports, as
// scorsese-render does for a page (src/build/kit.ts).
//
// Where the Rust server listens while developing. 8080 is the default the
// server crate is expected to take; override it when running it elsewhere:
// `SCORSESE_API=http://localhost:9000 bun run dev`.
const api = process.env.SCORSESE_API ?? "http://localhost:8080";

// Which build a crash report came from (#1428, src/app/crashes.ts): the UTC
// minute it was built, unless `SCORSESE_BUILD` names it.
const build = process.env.SCORSESE_BUILD ?? new Date().toISOString().slice(0, 16);

export default defineConfig({
  plugins: [react(), tailwindcss(), entryBudget(), motionKit()],
  define: { "import.meta.env.VITE_BUILD": JSON.stringify(build) },
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  server: {
    // The landing page (#904) loads the faces a video gets from outside web/,
    // never copies. (The motion kit, the other thing it shares, is read by
    // `motionKit`, src/build/kit.ts, which no file-serving rule governs.)
    fs: { allow: [".", "../crates/compositor/fonts"] },
    proxy: { "/api": { target: api, changeOrigin: true } },
  },
});
