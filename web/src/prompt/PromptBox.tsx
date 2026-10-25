// The prompt box: a large rounded field with its send button in the corner,
// on the landing page and on the signed-in home. Enter sends, Shift+Enter
// makes a new line, and an empty box sends nothing. What sending does is the
// caller's: `onSubmit` gets the text (which stays in the box) and may answer
// with a sentence to show, in red, under the box.
//
// It takes its colours from the theme tokens, so the landing page (which
// wraps it in `dark`) and the app (light or dark) both get a box that fits.

import { ArrowUpIcon } from "lucide-react";
import { type KeyboardEvent, useLayoutEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { useT } from "@/i18n/I18nProvider";
import { useTypewriter } from "@/prompt/useTypewriter";

export function PromptBox({
  onSubmit,
  autoFocus,
}: {
  /** What sending does: a sentence to show, or a promise of the work, which may fail with one. */
  onSubmit: (text: string) => string | undefined | void | Promise<unknown>;
  autoFocus?: boolean;
}) {
  const t = useT().prompt;
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
    if (!prompt || busy) return;
    const result = onSubmit(prompt);
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
      <div className="relative rounded-3xl border border-border bg-card text-card-foreground shadow-lg transition-[color,box-shadow,border-color] focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/30">
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
          className="max-h-60 min-h-24 field-sizing-fixed resize-none rounded-3xl border-0 bg-transparent px-5 py-4 pr-16 text-base shadow-none focus-visible:ring-0 md:text-base dark:bg-transparent"
        />
        <Button
          type="button"
          size="icon-lg"
          onClick={send}
          disabled={!text.trim() || busy}
          aria-label={t.send}
          className="absolute right-3 bottom-3 rounded-full"
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
