// One turn in the chat panel: what the user asked, what the assistant did on
// the way (progress notes and the tools it called), what it said, and what the
// turn cost. The cost is on every turn, not in a settings page, because the
// assistant spends the user's credits and a person should see each figure as
// it happens (docs/web.md, *Money*).

import { CheckIcon, LoaderIcon, WrenchIcon, XIcon } from "lucide-react";
import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";
import { QuestionCard } from "./QuestionCard";
import { QuoteBox } from "./QuoteBox";
import { awaitingQuote, type Entry, ending, type Line, waitingQuestion, words } from "./transcript";

export function Turn({ entry }: { entry: Entry }) {
  const t = useT();
  const { turn } = entry;
  const said = words(entry);
  const ended = ending(turn.state, t.chat);
  const waiting = waitingQuestion(turn);
  return (
    <li className="flex flex-col gap-2">
      <p className="ml-6 self-end whitespace-pre-wrap rounded-lg bg-muted px-3 py-2 text-sm">
        {turn.prompt}
      </p>
      {entry.lines.length > 0 && (
        <ul className="flex flex-col gap-0.5 text-xs text-muted-foreground">
          {entry.lines.map((line, index) => (
            // Lines only ever append, and one is never moved, so its place is its identity.
            // biome-ignore lint/suspicious/noArrayIndexKey: see above
            <LineRow key={index} line={line} />
          ))}
        </ul>
      )}
      {said && <p className="whitespace-pre-wrap text-sm">{said}</p>}
      {turn.state === "running" && !said && (
        <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <LoaderIcon className="size-3 animate-spin" /> {t.chat.turn.working}
        </p>
      )}
      {(turn.questions ?? []).map((question, index) => (
        <QuestionCard
          // Questions only ever append, so a question's place is its identity.
          // biome-ignore lint/suspicious/noArrayIndexKey: see above
          key={index}
          turn={turn.id}
          question={question}
          waiting={question === waiting}
        />
      ))}
      {ended && <p className="text-xs text-destructive">{ended}</p>}
      {awaitingQuote(turn) && turn.quote && <QuoteBox turn={turn.id} quote={turn.quote} />}
      {turn.state !== "running" && (
        <p className="text-xs text-muted-foreground">
          {turn.state === "asking"
            ? t.chat.turn.costSoFar(formatDollars(turn.charged_micros))
            : t.chat.turn.cost(formatDollars(turn.charged_micros))}
          {entry.balanceAfter !== null && t.chat.turn.left(formatDollars(entry.balanceAfter))}
        </p>
      )}
    </li>
  );
}

function LineRow({ line }: { line: Line }) {
  if (line.kind === "progress") return <li className="italic">{line.text}</li>;
  const Icon = { running: WrenchIcon, answered: CheckIcon, refused: XIcon }[line.state];
  return (
    <li className="flex items-start gap-1.5">
      <Icon
        className={`mt-0.5 size-3 shrink-0 ${line.state === "refused" ? "text-destructive" : ""}`}
      />
      <span className="min-w-0">
        <span className="font-mono">{line.tool}</span>
        {line.said && <span className="line-clamp-2"> — {line.said}</span>}
      </span>
    </li>
  );
}
