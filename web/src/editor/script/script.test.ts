// Reading and writing the script (#1445): no script reads as `null`, a save
// names the revision it was read at, and a project that moved on is re-read —
// saved over when only the timeline moved, refused when the script did.

import { afterEach, describe, expect, test } from "bun:test";
import { setFetch } from "@/api/client";
import { readScript, saveScript } from "./script";

let restore: Parameters<typeof setFetch>[0] | undefined;
afterEach(() => restore && setFetch(restore));

/** A server holding `text` at `revision`, that refuses writes naming `stale`. */
function serve(state: { text: string | null; revision: number }, stale?: number) {
  const writes: unknown[] = [];
  const answer = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status });
  const project = () => ({
    id: 3,
    revision: state.revision,
    document: state.text === null ? {} : { script: "script.md" },
  });
  restore = setFetch(async (input, init) => {
    const url = String(input);
    const body = init?.body ? JSON.parse(String(init.body)) : null;
    if (url.endsWith("/tools/script_read"))
      return answer({ said: [{ text: state.text, image: null }], project: project() });
    if (url.endsWith("/tools/script_write")) {
      writes.push(body);
      if (body.revision === stale) return answer({ error: "the project moved on" }, 409);
      state.text = body.arguments.text;
      state.revision += 1;
      return answer({ said: [{ text: "written", image: null }], project: project() });
    }
    return answer(project());
  });
  return writes;
}

describe("the script", () => {
  test("a project without one reads as none, with its revision", async () => {
    serve({ text: null, revision: 4 });
    expect(await readScript(3)).toEqual({ text: null, revision: 4 });
  });

  test("a save names the revision it was read at", async () => {
    const writes = serve({ text: "# One", revision: 4 });
    const base = await readScript(3);
    expect(base).toEqual({ text: "# One", revision: 4 });
    const saved = await saveScript(3, base, "# Two");
    expect(saved).toEqual({ state: "saved", script: { text: "# Two", revision: 5 } });
    expect(writes).toEqual([{ arguments: { text: "# Two" }, revision: 4 }]);
  });

  test("only the timeline moved: the save goes through on the new revision", async () => {
    const state = { text: "# One", revision: 4 };
    const writes = serve(state, 4);
    const base = { text: "# One", revision: 4 };
    state.revision = 6;
    const saved = await saveScript(3, base, "# Mine");
    expect(saved).toEqual({ state: "saved", script: { text: "# Mine", revision: 7 } });
    expect(writes).toHaveLength(2);
  });

  test("the assistant rewrote it: nothing is written, and it is shown as it is now", async () => {
    const state = { text: "# Theirs", revision: 6 };
    const writes = serve(state, 4);
    const saved = await saveScript(3, { text: "# One", revision: 4 }, "# Mine");
    expect(saved).toEqual({ state: "moved", script: { text: "# Theirs", revision: 6 } });
    expect(writes).toHaveLength(1);
    expect(state.text).toBe("# Theirs");
  });
});
