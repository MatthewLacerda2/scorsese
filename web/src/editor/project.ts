// The project being edited, and the one way the page changes it.
//
// Every hand-edit is a tool call (`api.editor.tool`), so what the page holds
// is only ever the server's document: an edit's answer carries the project as
// it is now and replaces the cached one, and nothing is changed optimistically
// — a refused drag has nothing to undo, it simply never lands, the way the
// desktop app's drag hits a wall.
//
// **The conflict rule** (#534): an edit names the revision it was worked out
// on. If the assistant (or another tab) changed the project meanwhile, the
// server answers 409 and writes nothing; the page reads the project again and
// says so, and the user redoes the edit on what is there now.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useState } from "react";
import {
  ApiError,
  api,
  describeError,
  type EditorProject,
  type EditorTool,
  type ToolAnswer,
} from "@/api";
import { queryKeys } from "@/app/queryKeys";
import type { Messages } from "@/i18n/catalogue";
import { useT } from "@/i18n/I18nProvider";

export function useEditorProject(id: number) {
  return useQuery({ queryKey: queryKeys.editor(id), queryFn: () => api.editor.open(id) });
}

/** A tool call the editor makes: `edit` ones name the revision they were worked out on. */
export interface Edit {
  tool: EditorTool;
  args: Record<string, unknown>;
  /** Whether it changes what a drag is computed on — so it names a revision. */
  edit: boolean;
}

/** What the last edit came to, to show the user: a refusal's reason, or nothing. */
export interface EditOutcome {
  run: (edit: Edit) => Promise<ToolAnswer | null>;
  refused: string | null;
  dismiss: () => void;
  pending: boolean;
}

/** The server's words for a failed edit, in the terms a user acts on. */
function refusal(error: unknown, t: Messages["editor"]["refusal"]): string {
  return describeError(error, { 409: t.conflict }, t.failed);
}

export function useEdit(id: number): EditOutcome {
  const queryClient = useQueryClient();
  const t = useT();
  const [refused, setRefused] = useState<string | null>(null);
  const mutation = useMutation({
    mutationFn: ({ tool, args, edit }: Edit) => {
      const current = queryClient.getQueryData<EditorProject>(queryKeys.editor(id));
      return api.editor.tool(id, tool, args, edit ? current?.revision : undefined);
    },
    onSuccess: (answer, { tool }) => {
      setRefused(null);
      if (answer.project) queryClient.setQueryData(queryKeys.editor(id), answer.project);
      // An import or a template changes which files this project uses.
      if (tool === "import" || tool === "template_insert")
        queryClient.invalidateQueries({ queryKey: queryKeys.library });
    },
    onError: (error) => {
      setRefused(refusal(error, t.editor.refusal));
      if (error instanceof ApiError && error.status === 409) {
        queryClient.invalidateQueries({ queryKey: queryKeys.editor(id) });
      }
    },
  });
  const { mutateAsync } = mutation;
  const run = useCallback((edit: Edit) => mutateAsync(edit).catch(() => null), [mutateAsync]);
  return { run, refused, dismiss: () => setRefused(null), pending: mutation.isPending };
}
