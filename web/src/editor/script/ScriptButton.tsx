// The Script button in the chat panel's top bar, beside the pickers (#1445),
// and the modal it opens: the project's script rendered as Markdown — the same
// safe rendering as the assistant's replies — and, on Edit, as plain text with
// Save and Cancel. The script is read each time the modal opens, and again
// whenever the project moves on while it is open, so it is the one the
// assistant last wrote (#1488); `script.ts` has the reading, the writing and
// the conflict rule. An edit is worked out on the script it started from: when
// the assistant rewrites it meanwhile, the save is refused and the person's
// version stays on screen, read-only, to copy from — never lost. A saved edit
// is news to the assistant: the server tells its next turn the person changed
// the script.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ScrollTextIcon } from "lucide-react";
import { useState } from "react";
import { describeError } from "@/api";
import { useServerEvents } from "@/app/events";
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
  const script = useQuery({
    queryKey: key,
    queryFn: () => readScript(projectId, queryClient.getQueryData<ScriptRead>(key)),
    staleTime: 0,
  });
  // The script an edit started from travels with it, so a re-read underneath
  // never turns someone else's rewrite into the base of this save.
  const [editing, setEditing] = useState<{ base: ScriptRead; text: string } | null>(null);
  // The person's version of a save the assistant's rewrite refused.
  const [kept, setKept] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  useServerEvents((event) => {
    const moved =
      event.type === "resync" ||
      (event.type === "project" &&
        event.id === projectId &&
        event.revision !== script.data?.revision);
    if (moved) void queryClient.invalidateQueries({ queryKey: key });
  });
  const save = useMutation({
    mutationFn: ({ base, text }: { base: ScriptRead; text: string }) =>
      saveScript(projectId, base, text),
    onMutate: () => setNotice(null),
    onSuccess: (saved, { text }) => {
      queryClient.setQueryData(key, saved.script);
      setEditing(null);
      setKept(saved.state === "moved" ? text : null);
      if (saved.state === "moved") setNotice(t.moved);
    },
    onError: (error) => setNotice(describeError(error, { 409: t.conflict }, t.failed)),
  });

  if (script.isPending) return <p className="text-sm text-muted-foreground">{chat.loading}</p>;
  if (!script.data) return <QueryError error={script.error} />;
  const read = script.data;
  const changed = editing !== null && read.text !== editing.base.text;
  const said = notice ?? (changed ? t.changed : null);
  return (
    <>
      {said && (
        <p role="status" className="rounded-md bg-muted px-2 py-1.5 text-xs text-muted-foreground">
          {said}
        </p>
      )}
      {editing ? (
        <Textarea
          aria-label={t.title}
          value={editing.text}
          onChange={(event) => setEditing({ ...editing, text: event.target.value })}
          className="max-h-[60vh] min-h-64 font-mono text-sm"
        />
      ) : read.text === null ? (
        <p className="text-sm text-muted-foreground">{t.empty}</p>
      ) : (
        <div className="max-h-[60vh] overflow-y-auto rounded-md border p-3">
          <Reply text={read.text} />
        </div>
      )}
      {kept !== null && (
        <div className="space-y-1">
          <p className="text-xs text-muted-foreground">{t.yours}</p>
          <Textarea
            readOnly
            aria-label={t.yours}
            value={kept}
            className="max-h-[30vh] min-h-24 font-mono text-sm text-muted-foreground"
          />
        </div>
      )}
      <DialogFooter>
        {editing ? (
          <>
            <Button variant="outline" disabled={save.isPending} onClick={() => setEditing(null)}>
              {t.cancel}
            </Button>
            <Button
              disabled={save.isPending || editing.text === (editing.base.text ?? "")}
              onClick={() => save.mutate(editing)}
            >
              {t.save}
            </Button>
          </>
        ) : (
          <>
            <Button variant="outline" onClick={onClose}>
              {t.close}
            </Button>
            <Button onClick={() => setEditing({ base: read, text: read.text ?? "" })}>
              {read.text === null ? t.write : t.edit}
            </Button>
          </>
        )}
      </DialogFooter>
    </>
  );
}
