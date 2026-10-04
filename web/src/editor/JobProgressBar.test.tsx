// The bar a running render shows (#698): drawn from the server's
// `job_progress` events, with the phase said where the frames say nothing.

import { describe, expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import type { JobView } from "@/api/events";
import { folded, JobProgressBar, progressWords } from "./JobProgressBar";

const running: JobView = {
  id: 12,
  kind: "render",
  state: "running",
  attempts: 1,
  result: null,
  error: null,
  created_at: 0,
  started_at: 1,
  finished_at: null,
  interrupted_at: null,
  progress: null,
};
const drawing = { percent: 42, phase: "drawing", done: 760, of: 1800 } as const;

describe("a render's progress", () => {
  test("draws a bar with its percentage once an event arrives", () => {
    expect(renderToString(<JobProgressBar job={running} />)).toBe("");
    const job = folded(running, { type: "job_progress", id: 12, progress: drawing });
    const html = renderToString(<JobProgressBar job={job} />);
    expect(html).toContain("42%");
    expect(html).toContain("translateX(-58%)");
  });

  test("says the phase where the percentage stands still", () => {
    const at = (phase: JobView["progress"] & object) => progressWords(phase);
    expect(at({ ...drawing, percent: 0, phase: "mixing" })).toBe("0% · mixing the sound…");
    expect(at({ ...drawing, percent: 99, phase: "finishing" })).toBe("99% · finishing the file…");
    expect(at(drawing)).toBe("42%");
  });

  test("is folded only into its own running job, and ends with it", () => {
    const event = { type: "job_progress", id: 13, progress: drawing } as const;
    expect(folded(running, event)).toBe(running);
    const job = folded(running, { ...event, id: 12 });
    const done = folded(job, { type: "job", ...running, state: "done" });
    expect(done.progress).toBeNull();
    expect(renderToString(<JobProgressBar job={done} />)).toBe("");
    const again = folded(job, { type: "job", ...running });
    expect(again.progress).toEqual(drawing);
  });
});
