// What Enter does in a prompt box, signed in: make a project named from the
// prompt (`projectName`), send the prompt to its assistant as the first message
// — with the files attached to it, which the server brings into the new
// project (#1534) — and open the project. The signed-in home calls it, and so
// does the hand-off page a signed-out prompt arrives at after the sign-in
// (#1535). Sending before opening means a reload of the editor never resends.
//
// When the message cannot go — the server refuses it (no credit at all), or
// files did not come through the sign-in — the project is still made and
// opened, with the prompt left in its chat box (`prompt/draft.ts`), the files
// in its tray and the reason above them. Only a project that could not be
// made fails here, with the sentence the server wrote.

import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { api } from "@/api";
import { chatApi } from "@/api/chat";
import { queryKeys } from "@/app/queryKeys";
import { projectName } from "@/home/projectName";
import { useT } from "@/i18n/I18nProvider";
import { leaveDraft } from "@/prompt/draft";
import type { Attached } from "@/prompt/PromptBox";

export function useStartFromPrompt(): (
  prompt: string,
  attached?: Attached,
  missing?: string[],
) => Promise<void> {
  const t = useT();
  const navigate = useNavigate();
  const client = useQueryClient();

  return async (prompt, attached, missing = []) => {
    const now = new Date();
    const day = `${String(now.getDate()).padStart(2, "0")}/${String(now.getMonth() + 1).padStart(2, "0")}`;
    const { id } = await api.projects.create({
      name: projectName(prompt, t.home.untitled(day)),
    });
    await client.invalidateQueries({ queryKey: queryKeys.projects });
    const files = attached?.files ?? [];
    if (missing.length > 0) leaveDraft(id, { prompt, files, missing });
    else {
      try {
        await chatApi.send(id, prompt, false, attached?.items ?? []);
      } catch (refusal) {
        leaveDraft(id, { prompt, files, refusal });
      }
    }
    navigate(`/projects/${id}/edit`);
  };
}
