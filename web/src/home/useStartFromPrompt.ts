// What Enter does in the home's prompt box: make a project named from the
// prompt (`projectName`), send the prompt to its assistant as the first message
// so it is not lost — with the files attached to it, which the server brings
// into the new project (#1534) — and open the project. A refusal (no credit, an assistant
// already at work) comes back as the sentence the server wrote, shown under the
// box; the project already made is kept, so sending again does not make a
// second one.

import { useQueryClient } from "@tanstack/react-query";
import { useRef } from "react";
import { useNavigate } from "react-router";
import { api } from "@/api";
import { chatApi } from "@/api/chat";
import { queryKeys } from "@/app/queryKeys";
import { projectName } from "@/home/projectName";
import { useT } from "@/i18n/I18nProvider";
import type { Attached } from "@/prompt/PromptBox";

export function useStartFromPrompt(): (prompt: string, attached?: Attached) => Promise<void> {
  const t = useT();
  const navigate = useNavigate();
  const client = useQueryClient();
  const made = useRef<{ prompt: string; id: number } | null>(null);

  return async (prompt, attached) => {
    const now = new Date();
    const day = `${String(now.getDate()).padStart(2, "0")}/${String(now.getMonth() + 1).padStart(2, "0")}`;
    if (made.current?.prompt !== prompt) {
      const project = await api.projects.create({
        name: projectName(prompt, t.home.untitled(day)),
      });
      made.current = { prompt, id: project.id };
      await client.invalidateQueries({ queryKey: queryKeys.projects });
    }
    const { id } = made.current;
    await chatApi.send(id, prompt, false, attached?.items ?? []);
    navigate(`/projects/${id}/edit`);
  };
}
