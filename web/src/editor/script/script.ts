// The project's script as the Script button shows it (#1445), read and
// written through the same tools the assistant uses — `script_read` and
// `script_write` over the editor's route — so the server keeps no script logic
// of its own.
//
// **The conflict rule** (#534) holds, narrowed to what the person can see: a
// save names the revision the script was read at. When the project moved on
// meanwhile, the script is read again. If it is the text the person started
// from — the assistant was busy on the timeline, not the script — the save
// goes through on the new revision; if the assistant rewrote it, nothing is
// written and the modal shows the script as it is now.

import { ApiError, api } from "@/api";

/** The script and the revision it was read at; `text` is `null` when there is none. */
export interface ScriptRead {
  text: string | null;
  revision: number;
}

/** How a save came out: kept, or refused because the assistant rewrote it first. */
export type Saved = { state: "saved"; script: ScriptRead } | { state: "moved"; script: ScriptRead };

/** Read project `id`'s script as it is now. */
export async function readScript(id: number): Promise<ScriptRead> {
  const project = await api.editor.open(id);
  if (!project.document.script) return { text: null, revision: project.revision };
  const answer = await api.editor.tool(id, "script_read", {});
  const text = answer.said.map((part) => part.text).join("\n");
  return { text, revision: answer.project?.revision ?? project.revision };
}

/** Write `text` as project `id`'s script, over `base` — what the person started from. */
export async function saveScript(id: number, base: ScriptRead, text: string): Promise<Saved> {
  try {
    return { state: "saved", script: await write(id, text, base.revision) };
  } catch (error) {
    if (!(error instanceof ApiError) || error.status !== 409) throw error;
  }
  const now = await readScript(id);
  if (now.text !== base.text) return { state: "moved", script: now };
  return { state: "saved", script: await write(id, text, now.revision) };
}

async function write(id: number, text: string, revision: number): Promise<ScriptRead> {
  const answer = await api.editor.tool(id, "script_write", { text }, revision);
  return { text, revision: answer.project?.revision ?? revision + 1 };
}
