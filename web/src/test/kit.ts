// `bun test`'s copy of the Vite plugin in src/build/kit.ts, loaded by
// `bunfig.toml`'s preload: the landing page imports the motion kit as
// `MOTION_KIT`, which only a plugin can answer.

import { plugin } from "bun";
import { kitSource, MOTION_KIT } from "@/build/kit";

plugin({
  name: "motion-kit",
  setup(build) {
    build.module(MOTION_KIT, () => ({ contents: kitSource(), loader: "js" }));
  },
});
