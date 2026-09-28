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
import { ApiError, api, type EditorProject, type EditorTool, type ToolAnswer } from "@/api";

/** The cache key the editor's project lives under — in the projects area. */
export const editorKey = (id: number) => ["projects", "editor", id] as const;

export function useEditorProject(id: number) {
  return useQuery({ queryKey: editorKey(id), queryFn: () => api.editor.open(id) });
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
export function refusal(error: unknown): string {
  if (!(error instanceof ApiError)) return "the edit could not be made";
  if (error.status === 409) {
    return "The project changed while you were editing (the assistant, or another tab), so that edit was not made. Here is what is there now.";
  }
  return error.message;
}

export function useEdit(id: number): EditOutcome {
  const queryClient = useQueryClient();
  const [refused, setRefused] = useState<string | null>(null);
  const mutation = useMutation({
    mutationFn: ({ tool, args, edit }: Edit) => {
      const current = queryClient.getQueryData<EditorProject>(editorKey(id));
      return api.editor.tool(id, tool, args, edit ? current?.revision : undefined);
    },
    onSuccess: (answer, { tool }) => {
      setRefused(null);
      if (answer.project) queryClient.setQueryData(editorKey(id), answer.project);
      // An import changes which files this project uses.
      if (tool === "import") queryClient.invalidateQueries({ queryKey: ["library"] });
    },
    onError: (error) => {
      setRefused(refusal(error));
      if (error instanceof ApiError && error.status === 409) {
        queryClient.invalidateQueries({ queryKey: editorKey(id) });
      }
    },
  });
  const { mutateAsync } = mutation;
  const run = useCallback((edit: Edit) => mutateAsync(edit).catch(() => null), [mutateAsync]);
  return { run, refused, dismiss: () => setRefused(null), pending: mutation.isPending };
}
