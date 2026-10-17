// One turn in the chat panel: what the user asked, what the assistant did on
// the way (a status word while it works; never its tools, #1437, nor its
// thinking, #767), what it said (Markdown, `Reply.tsx`), and what the turn cost. The cost is on every turn, not in a settings page, because the
// assistant spends the user's credits and a person should see each figure as
// it happens (docs/web.md, *Money*).

import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";
import { Busy } from "./Busy";
import { PickerCard } from "./PickerCard";
import { isPicker } from "./picker";
import { QuestionCard } from "./QuestionCard";
import { QuoteBox } from "./QuoteBox";
import { Reply } from "./Reply";
import { awaitingQuote, type Entry, ending, waitingQuestion, words } from "./transcript";

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
      {said && <Reply text={said} />}
      {turn.state === "running" && <Busy />}
      {(turn.questions ?? []).map((question, index) => {
        const Card = isPicker(question) ? PickerCard : QuestionCard;
        return (
          <Card
            // Questions only ever append, so a question's place is its identity.
            // biome-ignore lint/suspicious/noArrayIndexKey: see above
            key={index}
            turn={turn.id}
            question={question}
            waiting={question === waiting}
          />
        );
      })}
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
