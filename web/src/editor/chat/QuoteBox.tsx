// The confirmation box for a paid tool's quote (#538): the one thing the
// assistant cannot answer for the user. It shows what each item would send
// beside its price (#709), and takes one of three answers, all
// `POST /api/chat/turns/{id}/quote`:
//
// - Confirm spends and starts the turn that carries on — money moves only on
//   this click. When stills are offered as a half-price batch beside the price
//   for now (#947), there are two of these, side by side: now, or within 24
//   hours at half. Neither is the default; the person picks.
// - Decline withdraws the quote.
// - Ask for a change withdraws it too, spending nothing, and starts a turn with
//   the words typed, which the assistant takes as a change to these items: it
//   rewrites the briefs and quotes again, and that quote gets its own box.
//
// The model never sees the quote's token on any of the three.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { chatApi, type QuoteAnswer, type QuoteItem, type QuoteView } from "@/api/chat";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";
import { changeAnswer, offer, preview } from "./quote";

export function QuoteBox({ turn, quote }: { turn: number; quote: QuoteView }) {
  const t = useT();
  const queryClient = useQueryClient();
  const [change, setChange] = useState("");
  const answer = useMutation({
    mutationFn: (answer: QuoteAnswer) => chatApi.answerQuote(turn, answer),
    // The turn that carries on arrives on the event stream too; re-reading the
    // conversation also closes this box, whose turn now has its answer.
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["chat"] });
      queryClient.invalidateQueries({ queryKey: ["credits"] });
    },
  });
  const expired = quote.expires_at * 1000 < Date.now();
  const asked = changeAnswer(change);
  const offered = offer(quote);
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-amber-500/50 bg-amber-500/5 p-3 text-sm">
      <p className="font-medium">
        {offered
          ? t.chat.quote.choose(formatDollars(offered.now), formatDollars(offered.batch))
          : t.chat.quote.ask(formatDollars(quote.micros))}
      </p>
      {(quote.items ?? []).length > 0 && (
        <ul className="flex flex-col gap-2">
          {(quote.items ?? []).map((item) => (
            <Item key={item.subject} item={item} />
          ))}
        </ul>
      )}
      <ul className="flex flex-col gap-0.5 text-xs text-muted-foreground">
        {quote.lines.map((line) => (
          <li key={line}>{line}</li>
        ))}
      </ul>
      {expired && <p className="text-xs text-muted-foreground">{t.chat.quote.expired}</p>}
      {answer.data?.note && <p className="text-xs text-muted-foreground">{answer.data.note}</p>}
      {answer.isError && <p className="text-xs text-destructive">{answer.error.message}</p>}
      <div className="flex gap-2">
        <Button
          size="sm"
          disabled={answer.isPending || expired}
          onClick={() => answer.mutate({ confirm: true })}
        >
          {offered ? t.chat.quote.now(formatDollars(offered.now)) : t.chat.quote.confirm}
        </Button>
        {offered && (
          <Button
            size="sm"
            disabled={answer.isPending || expired}
            onClick={() => answer.mutate({ confirm: true, batch: true })}
          >
            {t.chat.quote.batch(formatDollars(offered.batch))}
          </Button>
        )}
        <Button
          size="sm"
          variant="outline"
          disabled={answer.isPending}
          onClick={() => answer.mutate({ confirm: false })}
        >
          {t.chat.quote.decline}
        </Button>
      </div>
      <form
        className="flex flex-col gap-1.5"
        onSubmit={(event) => {
          event.preventDefault();
          if (asked) answer.mutate(asked);
        }}
      >
        <Textarea
          value={change}
          onChange={(event) => setChange(event.target.value)}
          placeholder={t.chat.quote.changePlaceholder}
          disabled={answer.isPending}
          className="min-h-10 text-xs"
        />
        <Button
          type="submit"
          size="sm"
          variant="outline"
          className="self-start"
          disabled={answer.isPending || !asked}
        >
          {t.chat.quote.change}
        </Button>
      </form>
    </div>
  );
}

/** One quoted item: what it is, its price, and the words that would be sent. */
function Item({ item }: { item: QuoteItem }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const { shown, clipped } = preview(item.description);
  return (
    <li className="flex flex-col gap-0.5 text-xs">
      <span className="text-muted-foreground">
        <span className="font-medium text-foreground">{item.subject}</span>: {item.says}
      </span>
      <span>
        <span className="text-muted-foreground">{t.chat.quote.brief[item.brief]}: </span>
        {open ? item.description : shown}
        {clipped && (
          <button
            type="button"
            className="ml-1 text-muted-foreground underline"
            onClick={() => setOpen(!open)}
          >
            {open ? t.chat.quote.less : t.chat.quote.more}
          </button>
        )}
      </span>
    </li>
  );
}
