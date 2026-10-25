// The prompt box: a large rounded field with its send button in the corner,
// on the landing page and on the signed-in home. Enter sends, Shift+Enter
// makes a new line, and an empty box sends nothing. What sending does is the
// caller's: `onSubmit` gets the text (which stays in the box) and may answer
// with a sentence to show, in red, under the box.
//
// Files attach as they do in a chatbot (#1534): the "+" picks them, a drop on
// the box adds them, and they show as thumbnails above the text. Signed in
// they upload to the library at once and sending waits for them; signed out
// they are only held, for the sign-in round trip to carry (`handOff.ts`). `onSubmit` gets them beside the text.
//
// It takes its colours from the theme tokens, so the landing page (which
// wraps it in `dark`) and the app (light or dark) both get a box that fits.
// Its "+" and send answer a thumb on a phone (`touch-target`, #1566).

import { ArrowUpIcon } from "lucide-react";
import { type KeyboardEvent, useLayoutEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { useT } from "@/i18n/I18nProvider";
import { AttachButton } from "@/prompt/attach/AttachButton";
import { AttachmentTray } from "@/prompt/attach/AttachmentTray";
import { DropOverlay } from "@/prompt/attach/DropOverlay";
import { useAttachments } from "@/prompt/attach/useAttachments";
import { useFileDrop } from "@/prompt/attach/useFileDrop";
import { useTypewriter } from "@/prompt/useTypewriter";

/** What was attached to a prompt as it was sent. */
export interface Attached {
  /** Every file, as picked or dropped. */
  files: File[];
  /** The library items they became — empty signed out, where nothing uploads. */
  items: number[];
}

/** What sending does: a sentence to show, or a promise of the work, which may fail with one. */
export type PromptSubmit = (
  text: string,
  attached: Attached,
) => string | undefined | void | Promise<unknown>;

export function PromptBox({
  onSubmit,
  autoFocus,
}: {
  onSubmit: PromptSubmit;
  autoFocus?: boolean;
}) {
  const t = useT().prompt;
  const attachments = useAttachments();
  const drop = useFileDrop(attachments.add, attachments.prepare);
  const [text, setText] = useState("");
  const [problem, setProblem] = useState<string>();
  const [busy, setBusy] = useState(false);
  // The placeholder types itself while the box is empty; the label names it.
  const typed = useTypewriter(t.examples, text === "");
  // The box rests at a height that holds any example (two lines), and only what is
  // typed grows it: the field is not sized by its placeholder (`field-sizing-fixed`),
  // so the examples typing and deleting never move the page.
  const field = useRef<HTMLTextAreaElement>(null);
  useLayoutEffect(() => {
    const element = field.current;
    if (!element) return;
    element.style.height = "auto";
    if (text !== "") element.style.height = `${element.scrollHeight}px`;
  }, [text]);

  const send = () => {
    const prompt = text.trim();
    if (!prompt || busy || attachments.busy) return;
    const files = attachments.list.map((attachment) => attachment.file);
    const result = onSubmit(prompt, { files, items: attachments.items });
    if (result instanceof Promise) {
      setProblem(undefined);
      setBusy(true);
      result.catch((failure: Error) => setProblem(failure.message)).finally(() => setBusy(false));
    } else setProblem(typeof result === "string" ? result : undefined);
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) return;
    event.preventDefault();
    send();
  };

  return (
    <div className="w-full">
      {/* biome-ignore lint/a11y/noStaticElementInteractions: a drop target, and a hint to ask what the "+" takes; the "+" is the keyboard way to attach */}
      <div
        className="relative rounded-3xl border border-border bg-card text-card-foreground shadow-lg transition-[color,box-shadow,border-color] focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/30"
        onPointerEnter={attachments.prepare}
        onFocus={attachments.prepare}
        {...drop.handlers}
      >
        {drop.dragging && <DropOverlay rounded="rounded-3xl" />}
        <AttachmentTray attachments={attachments} />
        <Textarea
          ref={field}
          value={text}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder={typed}
          // biome-ignore lint/a11y/noAutofocus: the box is the whole point of the screen it is on.
          autoFocus={autoFocus}
          aria-label={t.label}
          rows={2}
          className="max-h-60 min-h-28 field-sizing-fixed resize-none rounded-3xl border-0 bg-transparent px-5 pt-4 pb-14 text-base shadow-none focus-visible:ring-0 md:text-base dark:bg-transparent"
        />
        <AttachButton attachments={attachments} className="touch-target absolute bottom-3 left-3" />
        <Button
          type="button"
          size="icon-lg"
          onClick={send}
          disabled={!text.trim() || busy || attachments.busy}
          aria-label={t.send}
          className="touch-target absolute right-3 bottom-3 rounded-full"
        >
          <ArrowUpIcon />
        </Button>
      </div>
      {problem && (
        <p role="alert" className="mt-3 text-center text-sm text-destructive">
          {problem}
        </p>
      )}
    </div>
  );
}
