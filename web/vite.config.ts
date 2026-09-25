// The build and the dev server, in one place.
//
// `vite build` writes static files to `dist/`, which is all the deploy's nginx
// container serves: there is no Node or Bun process in production, only files.
//
// In development the page and the API are two processes on two ports, and the
// browser would treat them as two origins. Proxying `/api` through this server
// keeps them one origin, which is the shape production has too — there the
// split between `/api` and everything else happens in front of nginx. So a
// fetch written as `fetch("/api/...")` works unchanged in both.

import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Where the Rust server listens while developing. 8080 is the default the
// server crate is expected to take; override it when running it elsewhere:
// `SCORSESE_API=http://localhost:9000 bun run dev`.
const api = process.env.SCORSESE_API ?? "http://localhost:8080";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  server: {
    proxy: { "/api": { target: api, changeOrigin: true } },
  },
});
