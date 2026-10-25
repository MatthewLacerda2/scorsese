// The confirmation box for a paid tool's quote (#538): the one thing the
// assistant cannot answer for the user, so it reads in a glance (#1573): each
// item's name in its kind's hue with the words it would send under it (#709),
// then the cost in one sentence. No per-item price and no `lines` here: those
// stay in the API for the CLI, MCP and the answered history. It takes one of
// three answers, all `POST /api/chat/turns/{id}/quote`:
//
// - Confirm spends, and the turn the quote paused carries on — money moves only on
//   this click. When stills are offered as a half-price batch beside the price
//   for now (#947), there are two of these, side by side: now, or within 24
//   hours at half. Neither is the default; the person picks.
// - Decline withdraws the quote.
// - Ask for a change (its button shows once something is typed) withdraws it
//   too, spending nothing, and the turn carries on with the words typed, which the assistant takes as a change to these items: it
//   rewrites the briefs and quotes again, and that quote gets its own box.
//
// The model never sees the quote's token on any of the three.

import { useState } from "react";
import { chatApi, type QuoteAnswer, type QuoteItem, type QuoteView } from "@/api/chat";
import { queryKeys } from "@/app/queryKeys";
import { useInvalidatingMutation } from "@/app/useInvalidatingMutation";
import { Button } from "@/components/ui/button";
import { QueryError } from "@/components/ui/query-error";
import { Textarea } from "@/components/ui/textarea";
import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";
import { useEditorProject } from "../project";
import { changeAnswer, itemKind, kindHue, offer, preview } from "./quote";

export function QuoteBox({
  turn,
  project,
  quote,
}: {
  turn: number;
  project: number;
  quote: QuoteView;
}) {
  const t = useT();
  const [change, setChange] = useState("");
  // The items wait in the document the editor already holds, as sketches.
  const assets = useEditorProject(project).data?.document.assets;
  // The turn that carries on arrives on the event stream too; re-reading the
  // conversation also closes this box, whose turn now has its answer.
  const answer = useInvalidatingMutation([queryKeys.chat, queryKeys.credits], {
    mutationFn: (given: QuoteAnswer) => chatApi.answerQuote(turn, given),
  });
  const expired = quote.expires_at * 1000 < Date.now();
  const asked = changeAnswer(change);
  const offered = offer(quote);
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-warning/50 bg-warning/5 p-3 text-sm">
      {(quote.items ?? []).length > 0 && (
        <ul className="flex flex-col gap-2">
          {(quote.items ?? []).map((item) => (
            <Item key={item.subject} item={item} hue={kindHue(itemKind(item, assets))} />
          ))}
        </ul>
      )}
      <p className="font-medium">
        {offered
          ? t.chat.quote.choose(formatDollars(offered.now), formatDollars(offered.batch))
          : t.chat.quote.ask(formatDollars(quote.micros))}
      </p>
      {expired && <p className="text-xs text-muted-foreground">{t.chat.quote.expired}</p>}
      {answer.data?.note && <p className="text-xs text-muted-foreground">{answer.data.note}</p>}
      <QueryError error={answer.error} className="text-xs text-destructive" />
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
        {asked && (
          <Button
            type="submit"
            size="sm"
            variant="outline"
            className="self-start"
            disabled={answer.isPending}
          >
            {t.chat.quote.change}
          </Button>
        )}
      </form>
    </div>
  );
}

/** One quoted item: its name in its kind's hue, and the words that would be sent under it. */
function Item({ item, hue }: { item: QuoteItem; hue: string }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const { shown, clipped } = preview(item.description);
  return (
    <li className="flex flex-col gap-0.5 text-xs">
      <span className="font-medium" style={{ color: hue }}>
        {item.subject}
      </span>
      <span>
        {open ? item.description : shown}
        {clipped && (
          <Button
            type="button"
            variant="link"
            className="ml-1 h-auto p-0 text-xs text-muted-foreground underline"
            onClick={() => setOpen(!open)}
          >
            {open ? t.chat.quote.less : t.chat.quote.more}
          </Button>
        )}
      </span>
    </li>
  );
}
