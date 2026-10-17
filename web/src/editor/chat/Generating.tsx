// The one line shown while any generation the conversation ordered is still
// running (#1437), in place of a line per job: a spinner, the word, and an
// ellipsis that counts up. It goes when they are all done.

import { LoaderIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useT } from "@/i18n/I18nProvider";

const DOT_MS = 500;

export function Generating() {
  const word = useT().chat.generating;
  const [dots, setDots] = useState(0);
  useEffect(() => {
    const timer = setInterval(() => setDots((count) => (count + 1) % 4), DOT_MS);
    return () => clearInterval(timer);
  }, []);
  return (
    <p
      role="status"
      className="flex items-center gap-1.5 border-t pt-2 text-xs text-muted-foreground"
    >
      <LoaderIcon className="size-3 animate-spin" />
      <span>
        {word}
        {/* Reserve the full width so the line does not twitch as the dots come and go. */}
        <span className="inline-block w-[1.2em] text-left">{".".repeat(dots)}</span>
      </span>
    </p>
  );
}
