// One turn in the chat panel: what the user asked, what the assistant did on
// the way (a status word while it works; never its tools, #1437, nor its
// thinking, #767), what it said (Markdown, `Reply.tsx`), each answered quote as
// one line with the person's words under it (#1440), and what the turn cost. The cost is on every turn, not in a settings page, because the
// assistant spends the user's credits and a person should see each figure as
// it happens (docs/web.md, *Money*).

import type { QuoteReply } from "@/api/chat";
import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";
import { Busy } from "./Busy";
import { PickerCard } from "./PickerCard";
import { isPicker } from "./picker";
import { QuestionCard } from "./QuestionCard";
import { QuoteBox } from "./QuoteBox";
import { awaitingQuote } from "./quote";
import { Reply } from "./Reply";
import { type Entry, ending, waitingQuestion, words } from "./transcript";

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
      {(turn.quote_replies ?? []).map((reply, index) => (
        // Replies only ever append, so a reply's place is its identity.
        // biome-ignore lint/suspicious/noArrayIndexKey: see above
        <QuoteLine key={index} reply={reply} />
      ))}
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

/** An answered quote as one line, and the person's own words under it when they wrote any. */
function QuoteLine({ reply }: { reply: QuoteReply }) {
  const t = useT();
  return (
    <>
      <p className="text-xs text-muted-foreground">
        {t.chat.quote.answered[reply.answer](formatDollars(reply.micros))}
      </p>
      {reply.words && (
        <p className="ml-6 self-end whitespace-pre-wrap rounded-lg bg-muted px-3 py-2 text-sm">
          {reply.words}
        </p>
      )}
    </>
  );
}

