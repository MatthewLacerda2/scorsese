// A working turn's status line: a spinner and a word that changes every few
// seconds (`status.ts`).

import { LoaderIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useT } from "@/i18n/I18nProvider";
import { EVERY_MS, nextWord } from "./status";

export function Busy() {
  const words = useT().chat.turn.busy;
  const [word, setWord] = useState(() => nextWord(words, null));
  useEffect(() => {
    setWord((previous) => (words.includes(previous) ? previous : nextWord(words, null)));
    const timer = setInterval(() => setWord((previous) => nextWord(words, previous)), EVERY_MS);
    return () => clearInterval(timer);
  }, [words]);
  return (
    <p className="flex items-center gap-1.5 text-xs text-muted-foreground" aria-live="polite">
      <LoaderIcon className="size-3 animate-spin" /> {word}
    </p>
  );
}
