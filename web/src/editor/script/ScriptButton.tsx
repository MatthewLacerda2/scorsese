// The Script button in the chat panel's top bar, beside the pickers (#1445),
// and the modal it opens: the project's script rendered as Markdown — the same
// safe rendering as the assistant's replies — and, on Edit, as plain text with
// Save and Cancel. The script is read each time the modal opens, so it is the
// one the assistant last wrote; `script.ts` has the reading, the writing and
// the conflict rule. A saved edit is news to the assistant: the server tells
// its next turn the person changed the script.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ScrollTextIcon } from "lucide-react";
import { useState } from "react";
import { describeError } from "@/api";
import { queryKeys } from "@/app/queryKeys";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { QueryError } from "@/components/ui/query-error";
import { Textarea } from "@/components/ui/textarea";
import { Reply } from "@/editor/chat/Reply";
import { useT } from "@/i18n/I18nProvider";
import { readScript, type ScriptRead, saveScript } from "./script";

export function ScriptButton({ projectId, labelled }: { projectId: number; labelled: boolean }) {
  const t = useT().chat.script;
  const [open, setOpen] = useState(false);
  return (
    <>
      {/* In a narrow sidebar, the icon alone; named either way. */}
      <Button
        size={labelled ? "sm" : "icon-sm"}
        variant="ghost"
        className="shrink-0"
        aria-label={t.open}
        title={t.hint}
        onClick={() => setOpen(true)}
      >
        <ScrollTextIcon />
        {labelled && t.open}
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-3xl">
          <DialogHeader>
            <DialogTitle>{t.title}</DialogTitle>
            <DialogDescription>{t.description}</DialogDescription>
          </DialogHeader>
          {open && <ScriptBody projectId={projectId} onClose={() => setOpen(false)} />}
        </DialogContent>
      </Dialog>
    </>
  );
}

/** What the modal holds: the script read, or edited. Mounted only while open. */
function ScriptBody({ projectId, onClose }: { projectId: number; onClose: () => void }) {
  const chat = useT().chat;
  const t = chat.script;
  const queryClient = useQueryClient();
  const key = queryKeys.script(projectId);
  const script = useQuery({ queryKey: key, queryFn: () => readScript(projectId), staleTime: 0 });
  const [draft, setDraft] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const save = useMutation({
    mutationFn: ({ base, text }: { base: ScriptRead; text: string }) =>
      saveScript(projectId, base, text),
    onMutate: () => setNotice(null),
    onSuccess: (saved) => {
      queryClient.setQueryData(key, saved.script);
      setDraft(null);
      if (saved.state === "moved") setNotice(t.moved);
    },
    onError: (error) => setNotice(describeError(error, { 409: t.conflict }, t.failed)),
  });

  if (script.isPending) return <p className="text-sm text-muted-foreground">{chat.loading}</p>;
  if (!script.data) return <QueryError error={script.error} />;
  const read = script.data;
  const editing = draft !== null;
  return (
    <>
      {notice && (
        <p role="status" className="rounded-md bg-muted px-2 py-1.5 text-xs text-muted-foreground">
          {notice}
        </p>
      )}
      {editing ? (
        <Textarea
          aria-label={t.title}
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          className="max-h-[60vh] min-h-64 font-mono text-sm"
        />
      ) : read.text === null ? (
        <p className="text-sm text-muted-foreground">{t.empty}</p>
      ) : (
        <div className="max-h-[60vh] overflow-y-auto rounded-md border p-3">
          <Reply text={read.text} />
        </div>
      )}
      <DialogFooter>
        {editing ? (
          <>
            <Button variant="outline" disabled={save.isPending} onClick={() => setDraft(null)}>
              {t.cancel}
            </Button>
            <Button
              disabled={save.isPending || draft === (read.text ?? "")}
              onClick={() => save.mutate({ base: read, text: draft })}
            >
              {t.save}
            </Button>
          </>
        ) : (
          <>
            <Button variant="outline" onClick={onClose}>
              {t.close}
            </Button>
            <Button onClick={() => setDraft(read.text ?? "")}>
              {read.text === null ? t.write : t.edit}
            </Button>
          </>
        )}
      </DialogFooter>
    </>
  );
}
