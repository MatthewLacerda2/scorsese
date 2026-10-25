// `/start` (#1535): where the Google sign-in lands a prompt written on the
// landing page. It takes what the landing kept (`prompt/handOff.ts`), which
// deletes it before anything else happens, so a reload here goes to the home
// rather than making a second project; uploads the files to the library, as
// the home's box does; and carries on as the home does
// (`useStartFromPrompt`), ending in the new project's editor. A file that
// did not come through, or that the library refused, is named there to
// attach again, with the prompt left unsent in the chat box.

import { useEffect, useRef, useState } from "react";
import { Navigate } from "react-router";
import { useStartFromPrompt } from "@/home/useStartFromPrompt";
import { useT } from "@/i18n/I18nProvider";
import { useAttachments } from "@/prompt/attach/useAttachments";
import { type Kept, take } from "@/prompt/handOff";

export function HandOffPage() {
  const t = useT();
  const start = useStartFromPrompt();
  const attachments = useAttachments();
  const { add } = attachments;
  // `undefined` while it is being read, `null` when nothing was kept.
  const [kept, setKept] = useState<Kept | null>();
  const [problem, setProblem] = useState<string>();
  const begun = useRef(false);

  const read = useRef(false);
  useEffect(() => {
    // Once: React keeps the ref across its development double-run of effects.
    if (read.current) return;
    read.current = true;
    void take().then((found) => {
      setKept(found);
      if (found && found.files.length > 0) add(found.files);
    });
  }, [add]);

  const arrived = attachments.list.length + attachments.refused.length;
  const ready = kept && arrived >= kept.files.length && !attachments.busy;
  useEffect(() => {
    if (!kept || !ready || begun.current) return;
    begun.current = true;
    const failed = attachments.list
      .filter((one) => one.status.phase === "failed")
      .map((one) => one.file.name);
    const missing = [...kept.missing, ...attachments.refused, ...failed];
    const files = attachments.list
      .filter((one) => one.status.phase !== "failed")
      .map((one) => one.file);
    start(kept.prompt, { files, items: attachments.items }, missing).catch((failure: Error) =>
      setProblem(failure.message),
    );
  }, [kept, ready, attachments, start]);

  if (kept === null) return <Navigate to="/" replace />;
  return (
    <div className="flex flex-1 items-center justify-center p-6 text-center">
      {problem ? (
        <p role="alert" className="text-destructive">
          {problem}
        </p>
      ) : (
        <p className="text-muted-foreground">{t.home.handOff.starting}</p>
      )}
    </div>
  );
}
