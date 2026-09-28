// The confirmation box for a paid tool's quote (#538): the one thing the
// assistant cannot answer for the user. The model never sees the quote's
// token; a yes here is `POST /api/chat/turns/{id}/quote`, which spends and
// starts the turn that carries on — so money moves only on this click.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { chatApi, type QuoteView } from "@/api/chat";
import { Button } from "@/components/ui/button";
import { formatDollars } from "@/lib/money";

export function QuoteBox({ turn, quote }: { turn: number; quote: QuoteView }) {
  const queryClient = useQueryClient();
  const answer = useMutation({
    mutationFn: (confirm: boolean) => chatApi.answerQuote(turn, confirm),
    // The turn that carries on arrives on the event stream too; re-reading the
    // conversation also closes this box, whose turn now has its answer.
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["chat"] });
      queryClient.invalidateQueries({ queryKey: ["credits"] });
    },
  });
  const expired = quote.expires_at * 1000 < Date.now();
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-amber-500/50 bg-amber-500/5 p-3 text-sm">
      <p className="font-medium">
        This costs {formatDollars(quote.micros)} from your credits. Go ahead?
      </p>
      <ul className="flex flex-col gap-0.5 text-xs text-muted-foreground">
        {quote.lines.map((line) => (
          <li key={line}>{line}</li>
        ))}
      </ul>
      {expired && <p className="text-xs text-muted-foreground">This quote has expired.</p>}
      {answer.data?.note && <p className="text-xs text-muted-foreground">{answer.data.note}</p>}
      {answer.isError && <p className="text-xs text-destructive">{answer.error.message}</p>}
      <div className="flex gap-2">
        <Button
          size="sm"
          disabled={answer.isPending || expired}
          onClick={() => answer.mutate(true)}
        >
          Confirm
        </Button>
        <Button
          size="sm"
          variant="outline"
          disabled={answer.isPending}
          onClick={() => answer.mutate(false)}
        >
          Decline
        </Button>
      </div>
    </div>
  );
}
